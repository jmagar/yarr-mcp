use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

pub mod assertions;
pub mod guard;
pub mod http;
pub mod matrix;
pub mod mcporter;
pub mod process;
pub mod report;
pub mod suites;
pub mod surface;

const MATRIX_PATH: &str = "tests/live/service_matrix.json";
const REPORT_PATH: &str = "target/live-full/report.json";
pub(super) const LIVE_PORT: u16 = 40170;
const LIVE_SERVE_DEFAULT_PORT: u16 = 40171;
const LIVE_SERVE_MCP_PORT: u16 = 40172;
pub(super) const LIVE_AUTH_PORT: u16 = 40173;
pub(super) const LIVE_OAUTH_PORT: u16 = 40174;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suite {
    Guard,
    Cli,
    Rest,
    Mcp,
    Mcporter,
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
    let surface_markers = surface::runtime_markers();

    run_guard(&mut report, &guarded);
    match options.suite {
        Suite::Guard => {}
        Suite::Cli => run_cli(&mut report, &rustarr, &matrix)?,
        Suite::Rest => suites::run_rest(&mut report, &rustarr)?,
        Suite::Mcp => suites::run_mcp(&mut report, &rustarr, &matrix)?,
        Suite::Mcporter => mcporter::run(&mut report, &rustarr, &matrix)?,
        Suite::Services => run_services(&mut report, &rustarr, &matrix)?,
        Suite::All => {
            run_cli(&mut report, &rustarr, &matrix)?;
            suites::run_rest(&mut report, &rustarr)?;
            suites::run_mcp(&mut report, &rustarr, &matrix)?;
            mcporter::run(&mut report, &rustarr, &matrix)?;
            run_services(&mut report, &rustarr, &matrix)?;
        }
    }

    if matches!(options.suite, Suite::All) {
        ensure_surface_markers_recorded(&report, &surface_markers)?;
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
    let actual_kinds: std::collections::BTreeSet<_> =
        guarded.kinds.values().map(String::as_str).collect();
    let required_kinds = guard::required_kinds();
    report.pass(
        "guard complete shart env",
        format!(
            "{} services, {} required kinds",
            guarded.services.len(),
            actual_kinds.intersection(&required_kinds).count()
        ),
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
        bail!(
            "doctor --json failed: {}",
            String::from_utf8_lossy(&doctor.stderr)
        );
    }
    let doctor_json: Value = serde_json::from_slice(&doctor.stdout)?;
    let doctor_checks = doctor_json.as_array().ok_or_else(|| {
        anyhow::anyhow!("doctor --json did not return a check array: {doctor_json}")
    })?;
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
    report.pass(
        "cli doctor --json",
        format!("{} checks passed", doctor_checks.len()),
    );

    for service in &matrix.services {
        let status = rustarr.json(&["status", "--service", &service.name])?;
        assertions::assert_value(&status, &service.status)?;
        report.pass(
            format!("cli status {}", service.name),
            format!("semantic status matched ({})", service.kind),
        );

        for get_case in &service.get {
            let payload =
                rustarr.json(&["get", "--service", &service.name, "--path", &get_case.path])?;
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

    let setup_env = isolated_setup_env("setup-repair")?;
    let setup_repair = rustarr.output_with_env(&["setup", "repair"], &setup_env)?;
    if !setup_repair.status.success() {
        bail!(
            "setup repair failed: {}",
            String::from_utf8_lossy(&setup_repair.stderr)
        );
    }
    report.pass("cli setup repair", "isolated appdata repaired");

    let install_env = isolated_setup_env("setup-install")?;
    let setup_install = rustarr.output_with_env(&["setup", "install"], &install_env)?;
    if !setup_install.status.success() {
        bail!(
            "setup install failed: {}",
            String::from_utf8_lossy(&setup_install.stderr)
        );
    }
    let installed = Path::new("target/live-full/tmp/setup-install/home/.local/bin/rustarr");
    if !installed.is_file() {
        bail!(
            "setup install did not copy binary to {}",
            installed.display()
        );
    }
    report.pass("cli setup install", installed.display().to_string());

    let unknown = rustarr.output(&["__rustarr_live_unknown__"])?;
    if unknown.status.success() {
        bail!("unknown command unexpectedly succeeded");
    }
    let unknown_text = format!(
        "{}{}",
        String::from_utf8_lossy(&unknown.stdout),
        String::from_utf8_lossy(&unknown.stderr)
    );
    if !unknown_text.contains("Unknown command") {
        bail!("unknown command did not produce expected error: {unknown_text}");
    }
    report.pass("cli unknown command error", "unknown command rejected");

    let invalid_watch = rustarr.output(&["watch", "--interval", "0"])?;
    if invalid_watch.status.success() {
        bail!("watch --interval 0 unexpectedly succeeded");
    }
    let invalid_watch_text = format!(
        "{}{}",
        String::from_utf8_lossy(&invalid_watch.stdout),
        String::from_utf8_lossy(&invalid_watch.stderr)
    );
    if !invalid_watch_text.contains("positive integer") {
        bail!("invalid watch interval did not produce expected error: {invalid_watch_text}");
    }
    report.pass(
        "cli parser rejects invalid watch interval",
        "watch --interval 0 rejected",
    );

    let default_base = format!("http://127.0.0.1:{LIVE_SERVE_DEFAULT_PORT}");
    let mut default_server =
        rustarr.start_server_args(&[], "127.0.0.1", LIVE_SERVE_DEFAULT_PORT, &BTreeMap::new())?;
    default_server.wait_healthy(&default_base)?;
    report.pass(
        "cli serve default lifecycle",
        "default serve became healthy",
    );

    let serve_mcp_base = format!("http://127.0.0.1:{LIVE_SERVE_MCP_PORT}");
    let mut serve_mcp_server = rustarr.start_server_args(
        &["serve", "mcp"],
        "127.0.0.1",
        LIVE_SERVE_MCP_PORT,
        &BTreeMap::new(),
    )?;
    serve_mcp_server.wait_healthy(&serve_mcp_base)?;
    report.pass("cli serve mcp lifecycle", "serve mcp became healthy");

    let init_line = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-03-26\",\"capabilities\":{},\"clientInfo\":{\"name\":\"rustarr-live-stdio\",\"version\":\"1\"}}}\n";
    let stdio = rustarr.output_with_stdin(&["mcp"], init_line, Duration::from_secs(5))?;
    if !stdio.status.success() {
        bail!(
            "stdio mcp initialize failed: {}",
            String::from_utf8_lossy(&stdio.stderr)
        );
    }
    let stdio_json: Value = serde_json::from_slice(&stdio.stdout)?;
    assertions::assert_value(
        &stdio_json,
        &matrix::Expectation {
            json_path: Some("result.serverInfo.name".into()),
            equals: Some(json!("rustarr-mcp")),
            equals_any: None,
            value_type: None,
            contains: None,
            xml_root: None,
        },
    )?;
    report.pass(
        "cli mcp stdio initialize",
        "rustarr-mcp initialized over stdio",
    );

    let base = live_base_url();
    let mut server = rustarr.start_server(LIVE_PORT)?;
    server.wait_healthy(&base)?;
    let watch = rustarr.output_until_timeout(
        &["watch", "--url", &base, "--interval", "1"],
        Duration::from_secs(3),
    )?;
    let watch_text = String::from_utf8_lossy(&watch.stdout);
    if !watch_text.contains("[rustarr] UP") {
        bail!("watch did not emit initial UP line: {watch_text}");
    }
    report.pass("cli watch", "initial UP line emitted");
    Ok(())
}

fn run_services(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
    matrix: &matrix::Matrix,
) -> Result<()> {
    for service in &matrix.services {
        let status = rustarr.json(&["status", "--service", &service.name])?;
        assertions::assert_value(&status, &service.status)?;
        report.pass(
            format!("service_status {}", service.name),
            format!("semantic status matched ({})", service.kind),
        );

        for get_case in &service.get {
            let payload =
                rustarr.json(&["get", "--service", &service.name, "--path", &get_case.path])?;
            assertions::assert_value(&payload, &get_case.expectation)?;
            report.pass(
                format!("api_get {} {}", service.name, get_case.path),
                "semantic GET matched",
            );
        }

        let blocked_body = service.post_blocked.body.to_string();
        let blocked = rustarr.output(&[
            "post",
            "--service",
            &service.name,
            "--path",
            &service.post_blocked.path,
            "--body",
            &blocked_body,
        ])?;
        let blocked_text = format!(
            "{}{}",
            String::from_utf8_lossy(&blocked.stdout),
            String::from_utf8_lossy(&blocked.stderr)
        );
        assertions::assert_expected_error(
            &blocked_text,
            std::slice::from_ref(&service.post_blocked.error_contains),
        )?;
        report.pass(
            format!("api_post blocked {}", service.name),
            "confirm guard prevented mutation",
        );

        let expected_body = service.post_expected_error.body.to_string();
        let expected = rustarr.output(&[
            "post",
            "--service",
            &service.name,
            "--path",
            &service.post_expected_error.path,
            "--body",
            &expected_body,
            "--confirm",
        ])?;
        let expected_text = format!(
            "{}{}",
            String::from_utf8_lossy(&expected.stdout),
            String::from_utf8_lossy(&expected.stderr)
        );
        assertions::assert_expected_error(
            &expected_text,
            &service.post_expected_error.error_contains_any,
        )?;
        report.pass(
            format!("api_post confirmed upstream error {}", service.name),
            "confirm=true reached upstream and returned the expected service error shape",
        );
    }
    Ok(())
}

pub(super) fn configured_service_names(value: &Value) -> Result<Vec<String>> {
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

pub(super) fn live_base_url() -> String {
    format!("http://127.0.0.1:{LIVE_PORT}")
}

fn ensure_surface_markers_recorded(
    report: &report::Report,
    expected_markers: &[&'static str],
) -> Result<()> {
    for marker in expected_markers {
        if !report.contains_check(marker) {
            bail!("live suite did not record required surface marker: {marker}");
        }
    }
    Ok(())
}

fn isolated_setup_env(name: &str) -> Result<BTreeMap<String, String>> {
    let root = Path::new("target/live-full/tmp").join(name);
    if root.exists() {
        std::fs::remove_dir_all(&root)?;
    }
    let home = root.join("home");
    let rustarr_home = root.join("rustarr-home");
    std::fs::create_dir_all(&home)?;
    let mut env = BTreeMap::new();
    env.insert("HOME".into(), home.display().to_string());
    env.insert("RUSTARR_HOME".into(), rustarr_home.display().to_string());
    env.insert("RUSTARR_MCP_PORT".into(), "0".into());
    env.insert("RUSTARR_MCP_NO_AUTH".into(), "true".into());
    Ok(env)
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
                        "mcporter" => Suite::Mcporter,
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
    println!("cargo xtask live --suite <guard|cli|rest|mcp|mcporter|services|all>");
    println!("  --allow-partial  Only permitted for legacy live-read-smoke guard checks");
}
