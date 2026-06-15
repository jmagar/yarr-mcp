//! doctor — pre-flight environment validation command.
//!
//! Pattern §48: Every server binary MUST implement a `doctor` subcommand that
//! validates the environment and reports what's missing before the user tries
//! to start the server.
//!
//! # Usage
//!
//! ```text
//! rustarr doctor           # human-readable coloured output; exit 0/1
//! rustarr doctor --json    # machine-readable JSON; exit 0/1
//! ```
//!
//! # TEMPLATE
//!
//! This is the reference implementation for the rustarr family. When you
//! clone the template for a real service, the things you MUST change are:
//!
//! 1. Replace Rustarr's service inventory env vars with your service's required vars.
//! 2. Replace `"rustarr"` binary name with your binary name in `check_binary_in_path`.
//! 3. Replace `~/.rustarr/` data dir with your service's data dir (see `config::default_data_dir`).
//! 4. Add any service-specific checks (e.g. database connectivity, auth token format).
//! 5. Update the `print_doctor_report` section headings and hint text to match your service.
//!
//! Nothing else here needs changing for a basic deployment. Business logic for the
//! checks belongs in the individual `check_*` functions — never in `run_doctor`.

mod checks;

use checks::{
    check_auth_config, check_binary_in_path, check_config_file, check_dir_writable,
    check_port_available, check_required_var, check_service_url, check_upstream,
};

use anyhow::{bail, Result};
use serde::Serialize;

use crate::config::{default_data_dir, Config};
use crate::{app::RustarrService, rustarr::RustarrClient};

// ── Public entry point ────────────────────────────────────────────────────────

/// Run the doctor command.
///
/// Executes all pre-flight checks in order and prints a summary. Exits with
/// code 1 if any check fails; 0 if all pass.
///
/// # TEMPLATE
/// This function is the canonical §48 implementation. Add calls to new
/// `check_*` functions below to extend the rustarr diagnostics.
pub async fn run_doctor(config: &Config, json: bool) -> Result<()> {
    let mut checks: Vec<DoctorCheck> = Vec::new();

    // ── 1. Config and filesystem ──────────────────────────────────────────────
    //
    // In Docker this resolves to /data; bare-metal uses ~/.rustarr/.
    let data_dir = default_data_dir()?;

    checks.push(check_config_file(&data_dir));
    checks.push(check_dir_writable("Data directory", &data_dir));
    checks.push(check_dir_writable("Log directory", &data_dir.join("logs")));

    // ── 2. Binary in PATH ─────────────────────────────────────────────────────
    //
    checks.push(check_binary_in_path("rustarr"));

    // ── 3. Required environment variables / config ────────────────────────────
    //
    // Required vars fail with ✗.  Optional vars warn with ⚠.
    let services_configured = if config.rustarr.services.is_empty() {
        ""
    } else {
        "configured"
    };
    checks.push(check_required_var("RUSTARR_SERVICES", services_configured));

    // Each configured service must carry a non-empty base URL.
    for configured in &config.rustarr.services {
        checks.push(check_service_url(&configured.name, &configured.base_url));
    }

    // ── 4. Upstream connectivity ──────────────────────────────────────────────
    //
    // If no services are configured, the required-var check above already
    // flagged it. Otherwise use the service-specific status endpoint.
    if !config.rustarr.services.is_empty() {
        match RustarrClient::new(&config.rustarr) {
            Ok(client) => {
                let service = RustarrService::new(client, config.rustarr.clone());
                for configured in &config.rustarr.services {
                    checks.push(check_upstream(&service, &configured.name).await);
                }
            }
            Err(error) => checks.push(DoctorCheck::fail(
                "connectivity",
                "Upstream client",
                format!("Could not build upstream HTTP client: {error}"),
            )),
        }
    }

    // ── 5. MCP server port ────────────────────────────────────────────────────
    //
    // TEMPLATE: config.mcp.port defaults to 3000 for the template.
    //           Your service's port is set in config.toml [mcp] port.
    checks.push(check_port_available(&config.mcp.host, config.mcp.port).await);

    // ── 6. Auth configuration ─────────────────────────────────────────────────
    //
    // TEMPLATE: The auth check inspects the combination of host / auth settings
    //           and reports which auth mode is active, or warns if 0.0.0.0 has
    //           no auth configured.
    checks.push(check_auth_config(config));

    // ── Render output ─────────────────────────────────────────────────────────

    let issues = checks.iter().filter(|c| !c.ok).count();

    if json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        print_doctor_report(&checks);
    }

    // Exit code 1 when any check fails.
    if issues > 0 {
        bail!("doctor found {issues} issue(s)");
    }
    Ok(())
}

// ── DoctorCheck struct ────────────────────────────────────────────────────────

/// A single pre-flight check result.
///
/// `ok = true`  → the check passed; `value` shows what was found.
/// `ok = false` → the check failed; `hint` explains how to fix it.
///
/// # TEMPLATE
/// Serialises directly to the `--json` output. Add fields here if you need
/// additional metadata (e.g. `severity: "warning" | "error"`, `doc_url`).
#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    /// Logical category for grouping in human output and JSON filtering.
    ///
    /// TEMPLATE: Defined by each `check_*` function. Categories in the template:
    ///   "config" | "credentials" | "connectivity" | "server" | "auth"
    pub category: &'static str,

    /// Short human-readable name for the check (shown in the left column).
    pub name: String,

    /// `true` = passed (✓), `false` = failed (✗).
    pub ok: bool,

    /// What was found — shown in the right column when ok=true.
    /// For failed checks, the hint is more useful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// How to fix the problem — only present when `ok = false`.
    ///
    /// TEMPLATE: Make hints actionable — tell the user exactly what to type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,

    /// Round-trip latency in milliseconds — only for connectivity checks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

impl DoctorCheck {
    fn pass(category: &'static str, name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            ok: true,
            value: Some(value.into()),
            hint: None,
            latency_ms: None,
        }
    }

    fn fail(category: &'static str, name: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            category,
            name: name.into(),
            ok: false,
            value: None,
            hint: Some(hint.into()),
            latency_ms: None,
        }
    }

    fn pass_with_latency(
        category: &'static str,
        name: impl Into<String>,
        value: impl Into<String>,
        latency_ms: u64,
    ) -> Self {
        Self {
            category,
            name: name.into(),
            ok: true,
            value: Some(value.into()),
            hint: None,
            latency_ms: Some(latency_ms),
        }
    }

    fn fail_with_latency(
        category: &'static str,
        name: impl Into<String>,
        hint: impl Into<String>,
        latency_ms: u64,
    ) -> Self {
        Self {
            category,
            name: name.into(),
            ok: false,
            value: None,
            hint: Some(hint.into()),
            latency_ms: Some(latency_ms),
        }
    }
}

// ── Human-readable report ─────────────────────────────────────────────────────

/// Print the doctor report in human-readable coloured format.
///
/// Output follows the §48 layout:
///
/// ```text
/// rustarr-mcp v0.1.0 — environment check
///
///   Config
///   ────────────────────────────────────────────
///   ✓ Config file:  ~/.rustarr/config.toml
///   ✗ Data dir:     not writable
///     → Fix: chmod u+w ~/.rustarr
///   ...
/// ```
///
/// # TEMPLATE
/// Section headings and the version string are the main things to customise.
/// Add new sections if you add new check categories beyond the five defaults.
fn print_doctor_report(checks: &[DoctorCheck]) {
    use std::io::IsTerminal;
    let color = std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none();

    // ── ANSI helpers ──────────────────────────────────────────────────────────
    macro_rules! green {
        ($s:expr) => {
            if color {
                format!("\x1b[32m{}\x1b[0m", $s)
            } else {
                $s.to_string()
            }
        };
    }
    macro_rules! red {
        ($s:expr) => {
            if color {
                format!("\x1b[31m{}\x1b[0m", $s)
            } else {
                $s.to_string()
            }
        };
    }
    macro_rules! yellow {
        ($s:expr) => {
            if color {
                format!("\x1b[33m{}\x1b[0m", $s)
            } else {
                $s.to_string()
            }
        };
    }
    macro_rules! bold {
        ($s:expr) => {
            if color {
                format!("\x1b[1m{}\x1b[0m", $s)
            } else {
                $s.to_string()
            }
        };
    }
    macro_rules! dim {
        ($s:expr) => {
            if color {
                format!("\x1b[2m{}\x1b[0m", $s)
            } else {
                $s.to_string()
            }
        };
    }

    println!();
    println!(
        "{}",
        bold!(format!(
            "rustarr-mcp v{} — environment check",
            env!("CARGO_PKG_VERSION")
        ))
    );
    println!();

    // Group checks by category and print in order.
    let categories: &[(&str, &str)] = &[
        ("config", "Config"),
        ("credentials", "Service credentials"),
        ("connectivity", "Connectivity"),
        ("server", "MCP server"),
        ("auth", "Authentication"),
    ];

    for (cat_key, cat_label) in categories {
        let cat_checks: Vec<&DoctorCheck> =
            checks.iter().filter(|c| c.category == *cat_key).collect();
        if cat_checks.is_empty() {
            continue;
        }

        println!("  {}", bold!(cat_label));
        println!("  {}", dim!("─".repeat(44)));

        for check in &cat_checks {
            if check.ok {
                let value = check.value.as_deref().unwrap_or("");
                let latency = check
                    .latency_ms
                    .map(|ms| format!(" ({ms} ms)"))
                    .unwrap_or_default();
                println!(
                    "  {}  {:<28}  {}{}",
                    green!("✓"),
                    check.name,
                    value,
                    latency
                );
            } else {
                println!("  {}  {}", red!("✗"), check.name);
                if let Some(hint) = &check.hint {
                    for line in hint.lines() {
                        println!("    {}", yellow!(line));
                    }
                }
            }
        }

        println!();
    }

    // ── Summary line ──────────────────────────────────────────────────────────
    let issues = checks.iter().filter(|c| !c.ok).count();
    println!("  {}", dim!("━".repeat(44)));

    if issues == 0 {
        println!(
            "  {}  All checks passed. Run: {}",
            green!("✓"),
            bold!("rustarr serve")
        );
    } else {
        let noun = if issues == 1 { "issue" } else { "issues" };
        println!(
            "  {}  {} {noun} found. Fix before running: {}",
            red!("✗"),
            red!(issues.to_string()),
            bold!("rustarr serve")
        );
    }
    println!();
}

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod tests;
