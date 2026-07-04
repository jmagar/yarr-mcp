//! doctor — pre-flight environment validation command.
//!
//! Pattern §48: Every server binary MUST implement a `doctor` subcommand that
//! validates the environment and reports what's missing before the user tries
//! to start the server.
//!
//! # Usage
//!
//! ```text
//! yarr doctor           # human-readable coloured output; exit 0/1
//! yarr doctor --json    # machine-readable JSON; exit 0/1
//! ```
//!
//! # Extending diagnostics
//!
//! Add Rustarr-specific checks by implementing focused `check_*` helpers in the
//! sibling `checks` module and wiring them into `run_doctor`. Business logic for
//! checks belongs in those helpers, not in the report renderer.

mod checks;

use checks::{
    check_auth_config, check_binary_in_path, check_config_file, check_dir_writable,
    check_port_available, check_required_var, check_service_url, check_upstream,
};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::config::{Config, default_data_dir};
use crate::{app::RustarrService, yarr::RustarrClient};

// ── Public entry point ────────────────────────────────────────────────────────

/// Run the doctor command.
///
/// Executes all pre-flight checks in order and prints a summary. Exits with
/// code 1 if any check fails; 0 if all pass.
///
/// # Extending diagnostics
/// Add calls to new `check_*` functions below to extend Rustarr diagnostics.
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
    let services_configured = if config.yarr.services.is_empty() {
        ""
    } else {
        "configured"
    };
    checks.push(check_required_var("YARR_SERVICES", services_configured));

    // Each configured service must carry a non-empty base URL.
    for configured in &config.yarr.services {
        checks.push(check_service_url(&configured.name, &configured.base_url));
    }

    // ── 4. Upstream connectivity ──────────────────────────────────────────────
    //
    // If no services are configured, the required-var check above already
    // flagged it. Otherwise use the service-specific status endpoint.
    if !config.yarr.services.is_empty() {
        match RustarrClient::new(&config.yarr) {
            Ok(client) => {
                let service = RustarrService::new(client, config.yarr.clone());
                for configured in &config.yarr.services {
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
    // `config.mcp.port` comes from env/config and defaults to Rustarr's 40070.
    checks.push(check_port_available(&config.mcp.host, config.mcp.port).await);

    // ── 6. Auth configuration ─────────────────────────────────────────────────
    //
    // The auth check reports the active auth mode and warns about unsafe bind/auth
    // combinations.
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
/// # JSON contract
/// Serialises directly to the `--json` output. Add fields here only when
/// consumers need additional stable metadata.
#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    /// Logical category for grouping in human output and JSON filtering.
    ///
    /// Defined by each `check_*` function. Current categories:
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
    /// Keep hints actionable: tell the user exactly what to type.
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
/// # Report layout
/// Add new sections here when adding check categories beyond the current five.
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
            bold!("yarr serve")
        );
    } else {
        let noun = if issues == 1 { "issue" } else { "issues" };
        println!(
            "  {}  {} {noun} found. Fix before running: {}",
            red!("✗"),
            red!(issues.to_string()),
            bold!("yarr serve")
        );
    }
    println!();
}

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod tests;
