use anyhow::{Result, bail};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

pub mod assertions;
mod cli;
pub mod coverage;
pub mod guard;
pub mod http;
pub mod matrix;
pub mod mcporter;
pub mod process;
pub mod report;
mod services;
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
    CoverageCheck,
}

pub fn run(args: &[String]) -> Result<()> {
    let options = Options::parse(args)?;
    if matches!(options.suite, Suite::CoverageCheck) {
        coverage::check_markdown(
            Path::new("docs/LIVE_ENDPOINT_COVERAGE.md"),
            Path::new(REPORT_PATH),
        )?;
        println!("docs/LIVE_ENDPOINT_COVERAGE.md is current for {REPORT_PATH}");
        return Ok(());
    }

    let guarded = guard::load(None, options.allow_partial)?;
    let matrix = matrix::load(Path::new(MATRIX_PATH))?;
    let binary = rustarr_binary()?;
    let rustarr = process::RustarrProcess::new(binary, &guarded);
    let mut report = report::Report::default();
    let surface_markers = surface::runtime_markers();

    run_guard(&mut report, &guarded);
    match options.suite {
        Suite::Guard => {}
        Suite::Cli => cli::run(&mut report, &rustarr, &matrix)?,
        Suite::Rest => suites::run_rest(&mut report, &rustarr)?,
        Suite::Mcp => suites::run_mcp(&mut report, &rustarr, &matrix)?,
        Suite::Mcporter => mcporter::run(&mut report, &rustarr, &matrix)?,
        Suite::Services => services::run(&mut report, &rustarr, &matrix)?,
        Suite::CoverageCheck => unreachable!("coverage check returns before live services load"),
        Suite::All => {
            cli::run(&mut report, &rustarr, &matrix)?;
            suites::run_rest(&mut report, &rustarr)?;
            suites::run_mcp(&mut report, &rustarr, &matrix)?;
            mcporter::run(&mut report, &rustarr, &matrix)?;
            services::run(&mut report, &rustarr, &matrix)?;
        }
    }

    if matches!(options.suite, Suite::All) {
        ensure_surface_markers_recorded(&report, &surface_markers)?;
    }
    report.write_json(Path::new(REPORT_PATH))?;
    if matches!(options.suite, Suite::All) {
        coverage::write_markdown(
            Path::new("docs/LIVE_ENDPOINT_COVERAGE.md"),
            &report,
            REPORT_PATH,
        )?;
    }
    println!("{} live checks recorded in {REPORT_PATH}", report.len());
    if report.is_success() {
        Ok(())
    } else {
        bail!("one or more live checks failed")
    }
}

fn rustarr_binary() -> Result<String> {
    if let Ok(binary) = std::env::var("RUSTARR_BIN") {
        return Ok(binary);
    }

    let status = Command::new("cargo")
        .args(["build", "--bin", "rustarr"])
        .env_remove("CARGO_PROFILE_DEV_CODEGEN_BACKEND")
        .status()?;
    if !status.success() {
        bail!("failed to build rustarr debug binary for live suite");
    }
    Ok("target/debug/rustarr".into())
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
                        "coverage-check" => Suite::CoverageCheck,
                        _ => bail!("unknown live suite: {value}"),
                    };
                }
                "--coverage-check" => suite = Suite::CoverageCheck,
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
    println!("cargo xtask live --suite <guard|cli|rest|mcp|mcporter|services|all|coverage-check>");
    println!("cargo xtask live --coverage-check");
    println!("  --allow-partial  Only permitted for legacy live-read-smoke guard checks");
}
