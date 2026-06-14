use anyhow::{bail, Result};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

pub mod assertions;
pub mod guard;
pub mod http;
pub mod matrix;
pub mod process;
pub mod report;

const MATRIX_PATH: &str = "tests/live/service_matrix.json";
const REPORT_PATH: &str = "target/live-full/report.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suite {
    Guard,
    Cli,
    Rest,
    Mcp,
    Services,
    All,
}

pub fn run(args: &[String]) -> Result<()> {
    let options = Options::parse(args)?;
    let guarded = guard::load(None, options.allow_partial)?;
    let matrix = matrix::load(Path::new(MATRIX_PATH))?;
    let binary = std::env::var("RUSTARR_BIN").unwrap_or_else(|_| "target/release/rustarr".into());
    let rustarr = process::RustarrProcess::new(binary, &guarded);
    let mut report = report::Report::default();

    run_guard(&mut report, &guarded);
    match options.suite {
        Suite::Guard => {}
        Suite::Cli => run_cli(&mut report, &rustarr, &matrix)?,
        Suite::Rest => run_rest(&mut report, &rustarr)?,
        Suite::Mcp => run_mcp(&mut report, &rustarr, &matrix)?,
        Suite::Services => run_services(&mut report, &rustarr, &matrix)?,
        Suite::All => {
            run_cli(&mut report, &rustarr, &matrix)?;
            run_rest(&mut report, &rustarr)?;
            run_mcp(&mut report, &rustarr, &matrix)?;
            run_services(&mut report, &rustarr, &matrix)?;
        }
    }

    report.write_json(Path::new(REPORT_PATH))?;
    println!("{} live checks recorded in {REPORT_PATH}", report.len());
    if report.is_success() {
        Ok(())
    } else {
        bail!("one or more live checks failed")
    }
}

fn run_guard(report: &mut report::Report, guarded: &guard::GuardedEnv) {
    report.pass(
        "guard complete shart env",
        format!("{} services", guarded.services.len()),
    );
}

fn run_cli(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
    matrix: &matrix::Matrix,
) -> Result<()> {
    let version = rustarr.output(&["--version"])?;
    let version_text = String::from_utf8_lossy(&version.stdout);
    if !version.status.success() || !version_text.to_ascii_lowercase().contains("rustarr") {
        bail!("--version failed or did not mention rustarr: {version_text}");
    }
    report.pass("cli --version", version_text.trim());

    let help = rustarr.output(&["--help"])?;
    let help_text = format!(
        "{}{}",
        String::from_utf8_lossy(&help.stdout),
        String::from_utf8_lossy(&help.stderr)
    );
    if !help.status.success() || !help_text.contains("Usage:") {
        bail!("--help did not print usage");
    }
    report.pass("cli --help", "usage printed");

    let help_json = rustarr.json(&["help"])?;
    assertions::assert_value(
        &help_json,
        &matrix::Expectation {
            json_path: Some("actions".into()),
            equals: None,
            equals_any: None,
            value_type: Some("array".into()),
            contains: None,
            xml_root: None,
        },
    )?;
    report.pass("cli help action", "structured help contains actions");

    let integrations = rustarr.json(&["integrations"])?;
    let configured = configured_service_names(&integrations)?;
    for service in &matrix.services {
        if !configured.iter().any(|name| name == &service.name) {
            bail!("integrations missing configured service {}", service.name);
        }
    }
    report.pass(
        "cli integrations",
        format!("{} configured services returned", configured.len()),
    );

    let doctor = rustarr.output(&["doctor", "--json"])?;
    if !doctor.status.success() {
        bail!("doctor --json failed: {}", String::from_utf8_lossy(&doctor.stderr));
    }
    let doctor_json: Value = serde_json::from_slice(&doctor.stdout)?;
    let doctor_checks = doctor_json
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("doctor --json did not return a check array: {doctor_json}"))?;
    if doctor_checks.is_empty() {
        bail!("doctor --json returned no checks");
    }
    let failed: Vec<_> = doctor_checks
        .iter()
        .filter(|check| check.get("ok").and_then(Value::as_bool) != Some(true))
        .collect();
    if !failed.is_empty() {
        bail!("doctor --json reported failed checks: {failed:?}");
    }
    report.pass("cli doctor --json", format!("{} checks passed", doctor_checks.len()));

    for service in &matrix.services {
        let status = rustarr.json(&["status", "--service", &service.name])?;
        assertions::assert_value(&status, &service.status)?;
        report.pass(format!("cli status {}", service.name), "semantic status matched");

        for get_case in &service.get {
            let payload = rustarr.json(&["get", "--service", &service.name, "--path", &get_case.path])?;
            assertions::assert_value(&payload, &get_case.expectation)?;
            report.pass(
                format!("cli get {} {}", service.name, get_case.path),
                "semantic GET matched",
            );
        }

        let body = service.post_blocked.body.to_string();
        let blocked = rustarr.output(&[
            "post",
            "--service",
            &service.name,
            "--path",
            &service.post_blocked.path,
            "--body",
            &body,
        ])?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&blocked.stdout),
            String::from_utf8_lossy(&blocked.stderr)
        );
        assertions::assert_expected_error(
            &combined,
            std::slice::from_ref(&service.post_blocked.error_contains),
        )?;
        report.pass(
            format!("cli post confirm guard {}", service.name),
            "blocked before upstream mutation",
        );
    }

    let setup_check = rustarr.output(&["setup", "check"])?;
    report.pass(
        "cli setup check",
        format!("exit={}", setup_check.status.code().unwrap_or(-1)),
    );

    let setup_hook = rustarr.output(&["setup", "plugin-hook", "--no-repair"])?;
    report.pass(
        "cli setup plugin-hook --no-repair",
        format!("exit={}", setup_hook.status.code().unwrap_or(-1)),
    );

    let mut server = rustarr.start_server(40070)?;
    server.wait_healthy("http://127.0.0.1:40070")?;
    let watch = rustarr.output_until_timeout(
        &["watch", "--url", "http://127.0.0.1:40070", "--interval", "1"],
        Duration::from_secs(3),
    )?;
    let watch_text = String::from_utf8_lossy(&watch.stdout);
    if !watch_text.contains("[rustarr] UP") {
        bail!("watch did not emit initial UP line: {watch_text}");
    }
    report.pass("cli watch", "initial UP line emitted");
    Ok(())
}

fn run_rest(_report: &mut report::Report, _rustarr: &process::RustarrProcess) -> Result<()> {
    bail!("REST suite is not implemented yet")
}

fn run_mcp(
    _report: &mut report::Report,
    _rustarr: &process::RustarrProcess,
    _matrix: &matrix::Matrix,
) -> Result<()> {
    bail!("MCP suite is not implemented yet")
}

fn run_services(
    _report: &mut report::Report,
    _rustarr: &process::RustarrProcess,
    _matrix: &matrix::Matrix,
) -> Result<()> {
    bail!("services suite is not implemented yet")
}

fn configured_service_names(value: &Value) -> Result<Vec<String>> {
    let configured = value
        .get("configured")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("integrations missing configured array"))?;
    configured
        .iter()
        .map(|item| {
            item.get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| anyhow::anyhow!("configured item missing name: {item}"))
        })
        .collect()
}

#[derive(Debug)]
struct Options {
    suite: Suite,
    allow_partial: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut suite = Suite::All;
        let mut allow_partial = false;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--allow-partial" => allow_partial = true,
                "--suite" => {
                    index += 1;
                    let value = args.get(index).map(String::as_str).unwrap_or("");
                    suite = match value {
                        "guard" => Suite::Guard,
                        "cli" => Suite::Cli,
                        "rest" => Suite::Rest,
                        "mcp" => Suite::Mcp,
                        "services" => Suite::Services,
                        "all" => Suite::All,
                        _ => bail!("unknown live suite: {value}"),
                    };
                }
                other => bail!("unknown live option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            suite,
            allow_partial,
        })
    }
}

fn print_help() {
    println!("cargo xtask live --suite <guard|cli|rest|mcp|services|all>");
    println!("  --allow-partial  Only permitted for legacy live-read-smoke guard checks");
}
