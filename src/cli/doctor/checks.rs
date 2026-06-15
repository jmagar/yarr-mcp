//! Individual pre-flight check functions for the doctor command.
//!
//! Each `check_*` function is self-contained: it validates one aspect of the
//! environment and returns a `DoctorCheck`. No side effects other than network
//! calls in `check_upstream`.
//!
//! # TEMPLATE
//! Add new `check_*` functions here, then call them from `run_doctor` in the
//! parent module. See the existing functions for the expected signature shape.

#[cfg(test)]
#[path = "checks_tests.rs"]
mod tests;

use std::net::TcpListener;
use std::path::Path;
use std::time::Instant;

use crate::{
    app::RustarrService,
    config::Config,
    server::{resolve_auth_policy_kind, AuthPolicyKind},
};

use super::DoctorCheck;

// ── Config and filesystem ─────────────────────────────────────────────────────

/// Check that the config file exists in the data directory.
///
/// The template looks for `<data_dir>/config.toml` (e.g. `~/.rustarr/config.toml`).
/// A missing config file is non-fatal — the binary works with env vars alone —
/// but the check warns so operators know where to place one if needed.
///
/// # TEMPLATE
/// If your service requires config.toml to be present, change `pass` to `fail`
/// when the file is not found.
pub fn check_config_file(data_dir: &Path) -> DoctorCheck {
    let config_path = data_dir.join("config.toml");

    if config_path.exists() {
        DoctorCheck::pass("config", "Config file", config_path.display().to_string())
    } else {
        // Non-fatal: env vars can supply all config.
        // TEMPLATE: Change `pass` → `fail` if config.toml is mandatory.
        DoctorCheck {
            category: "config",
            name: "Config file".into(),
            ok: true, // warning-level: missing is OK, env vars cover it
            value: Some(format!(
                "{} (not found — using env vars / defaults)",
                config_path.display()
            )),
            hint: None,
            latency_ms: None,
        }
    }
}

/// Check that a directory exists and is writable by the current process.
///
/// For missing directories the check returns a failure — many operations
/// (logging, auth DB, etc.) require a writable data dir.
///
/// # TEMPLATE
/// `label` is shown in the left column ("Data directory", "Log directory", …).
/// Add this check for every directory your service writes to.
pub fn check_dir_writable(label: &str, dir: &Path) -> DoctorCheck {
    let name = format!("{label}: {}", dir.display());

    // Attempt to create the directory if missing (idempotent).
    if let Err(e) = std::fs::create_dir_all(dir) {
        return DoctorCheck::fail(
            "config",
            name,
            format!(
                "Cannot create {}: {e}\n    → Check parent directory permissions.",
                dir.display()
            ),
        );
    }

    // Test writability by creating and removing a temp file.
    let test_file = dir.join(".doctor_write_test");
    match std::fs::write(&test_file, b"") {
        Ok(_) => {
            if let Err(e) = std::fs::remove_file(&test_file) {
                return DoctorCheck::fail(
                    "config",
                    name,
                    format!(
                        "Writable, but cleanup failed for {}: {e}",
                        test_file.display()
                    ),
                );
            }

            DoctorCheck::pass("config", name, "writable")
        }
        Err(e) => DoctorCheck::fail(
            "config",
            name,
            format!("Not writable: {e}\n    → Run: chmod u+w {}", dir.display()),
        ),
    }
}

// ── Binary in PATH ────────────────────────────────────────────────────────────

/// Check that the binary is on `$PATH`.
///
/// Claude Code stdio config (`~/.claude/settings.json`) resolves the binary by
/// name. If it is not in PATH the stdio transport will silently fail.
///
/// # TEMPLATE
/// Replace `"rustarr"` with your binary name (matches Cargo.toml `[[bin]] name`).
pub fn check_binary_in_path(binary: &str) -> DoctorCheck {
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(':') {
        let candidate = std::path::Path::new(dir).join(binary);
        if candidate.is_file() {
            return DoctorCheck::pass(
                "config",
                format!("Binary in PATH: {binary}"),
                candidate.display().to_string(),
            );
        }
    }

    DoctorCheck::fail(
        "config",
        format!("Binary in PATH: {binary}"),
        format!(
            "`{binary}` not found in $PATH.\n    \
             → Run: install.sh   (installs to ~/.local/bin)\n    \
             → Or:  cargo install --path .  (builds from source)\n    \
             → Then add ~/.local/bin to your PATH."
        ),
    )
}

// ── Environment variables / credentials ──────────────────────────────────────

/// Check that a required environment variable / config value is non-empty.
///
/// `var_name` is the env var name (for display and the hint message).
/// `value` is the resolved value from the loaded `Config` (which merges env +
/// config.toml, so a non-empty value here means it is actually configured).
///
/// # TEMPLATE
/// Call this once per required variable. Add entries for every var that must be
/// set before `rustarr serve` will work.
pub fn check_required_var(var_name: &str, value: &str) -> DoctorCheck {
    if !value.is_empty() {
        let display = redact(value);
        DoctorCheck::pass(
            "credentials",
            var_name.to_string(),
            format!("{display} (set)"),
        )
    } else {
        DoctorCheck::fail(
            "credentials",
            var_name.to_string(),
            format!(
                "Not set.\n    \
                 → Add to ~/.rustarr/.env:  {var_name}=<your_value>\n    \
                 → Or export in your shell: export {var_name}=<your_value>"
            ),
        )
    }
}

/// Check that a configured service has a non-empty base URL.
///
/// A service named in `RUSTARR_SERVICES` with no `RUSTARR_<NAME>_URL` cannot be
/// reached. `Config::load()` now fails fast on this (see
/// `config::services::load_services_from_env`), but the doctor mirrors the
/// validation so an operator inspecting an explicitly-constructed config still
/// gets an actionable failure here.
pub fn check_service_url(service_name: &str, base_url: &str) -> DoctorCheck {
    if !base_url.is_empty() {
        // Show only the origin (scheme://host[:port]) — never the full URL, which
        // can embed credentials in userinfo (`http://user:pass@host`) or a token in
        // the query string. This detail is printed verbatim, including under `--json`.
        let display = match url::Url::parse(base_url) {
            Ok(u) => match u.host_str() {
                Some(host) => match u.port() {
                    Some(port) => format!("{}://{host}:{port}", u.scheme()),
                    None => format!("{}://{host}", u.scheme()),
                },
                None => "<no host>".to_string(),
            },
            Err(_) => "<invalid URL>".to_string(),
        };
        return DoctorCheck::pass(
            "credentials",
            format!("Service URL: {service_name}"),
            display,
        );
    }

    let env_name: String = service_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();

    DoctorCheck::fail(
        "credentials",
        format!("Service URL: {service_name}"),
        format!(
            "RUSTARR_{env_name}_URL is required for service {service_name}.\n    \
             → Add to ~/.rustarr/.env:  RUSTARR_{env_name}_URL=<your_url>\n    \
             → Or export in your shell: export RUSTARR_{env_name}_URL=<your_url>"
        ),
    )
}

fn redact(s: &str) -> String {
    if s.len() <= 4 {
        return "*".repeat(s.len());
    }
    format!("{}****", &s[..4])
}

// ── Upstream connectivity ─────────────────────────────────────────────────────

/// Check that one configured upstream service is reachable through the normal
/// Rustarr client path, including service-specific status endpoint and auth.
pub async fn check_upstream(service: &RustarrService, service_name: &str) -> DoctorCheck {
    let start = Instant::now();
    match service.service_status(service_name).await {
        Ok(_) => {
            let elapsed = start.elapsed().as_millis() as u64;
            DoctorCheck::pass_with_latency(
                "connectivity",
                format!("Upstream reachable: {service_name}"),
                format!("service_status succeeded ({elapsed} ms)"),
                elapsed,
            )
        }
        Err(error) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let env_name = service_name
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() {
                        ch.to_ascii_uppercase()
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            DoctorCheck::fail_with_latency(
                "connectivity",
                format!("Upstream reachable: {service_name}"),
                format!(
                    "{service_name} status check failed: {error}\n    \
                     → Check RUSTARR_{env_name}_URL and credentials, then retry rustarr doctor."
                ),
                elapsed,
            )
        }
    }
}

// ── MCP server port ───────────────────────────────────────────────────────────

/// Check that the configured MCP port is available (not already in use).
///
/// Binding on a port that is already taken causes `rustarr serve` to fail at
/// startup. This check catches that problem before the server starts.
///
/// Rustarr's default MCP HTTP port is 40070. Override with
/// `RUSTARR_MCP_PORT` or config.toml `[mcp] port`.
pub async fn check_port_available(host: &str, port: u16) -> DoctorCheck {
    let bind = format!("{host}:{port}");
    match TcpListener::bind((host, port)) {
        Ok(_) => DoctorCheck::pass("server", format!("MCP bind {bind}"), "available"),
        Err(e) => match probe_running_server(host, port).await {
            Ok(()) => DoctorCheck::pass(
                "server",
                format!("MCP bind {bind}"),
                format!("already running and healthy at {}", health_url(host, port)),
            ),
            Err(probe_error) => DoctorCheck::fail(
                "server",
                format!("MCP bind {bind}"),
                format!(
                    "Bind address {bind} is unavailable: {e}; health probe also failed: {probe_error}\n    \
                     → Set RUSTARR_MCP_PORT to a different port.\n    \
                     → Or stop the process using this address: ss -tlnp | grep :{port}"
                ),
            ),
        },
    }
}

async fn probe_running_server(host: &str, port: u16) -> Result<(), String> {
    let url = health_url(host, port);
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|error| error.to_string())?
        .get(&url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("GET {url} returned HTTP {}", response.status()))
    }
}

fn health_url(host: &str, port: u16) -> String {
    let probe_host = match host {
        "0.0.0.0" => "127.0.0.1".to_string(),
        "::" | "[::]" => "[::1]".to_string(),
        value if value.contains(':') && !value.starts_with('[') => format!("[{value}]"),
        value => value.to_string(),
    };
    format!("http://{probe_host}:{port}/health")
}

// ── Auth configuration ────────────────────────────────────────────────────────

/// Check that the auth configuration is consistent and safe.
///
/// Validates:
/// - Binding 0.0.0.0 without auth is rejected (§27).
/// - Reports which auth mode is active.
/// - Warns if no auth is configured.
///
/// This check mirrors `resolve_auth_policy_kind()` but produces a friendly
/// report instead of aborting. No logic changes needed unless you add a new
/// auth mode.
pub fn check_auth_config(config: &Config) -> DoctorCheck {
    match resolve_auth_policy_kind(config, config.mcp.trusted_gateway) {
        Ok(AuthPolicyKind::LoopbackDev) => {
            DoctorCheck::pass("auth", "Auth mode", "no-auth (loopback bind)")
        }
        Ok(AuthPolicyKind::TrustedGatewayUnscoped) => DoctorCheck::pass(
            "auth",
            "Auth mode",
            "trusted gateway unscoped (RUSTARR_NOAUTH=true — upstream handles auth and authz)",
        ),
        Ok(AuthPolicyKind::MountedOAuth) => {
            DoctorCheck::pass("auth", "Auth mode", "OAuth (Google)")
        }
        Ok(AuthPolicyKind::MountedBearer) => {
            DoctorCheck::pass("auth", "Auth mode", "bearer token (set)")
        }
        Err(error) => DoctorCheck::fail(
            "auth",
            "Auth mode",
            format!(
                "{error}\n    \
                 Fix ONE of:\n    \
                 1. Bind to loopback:    RUSTARR_MCP_HOST=127.0.0.1\n    \
                 2. Set a bearer token:  RUSTARR_MCP_TOKEN=$(openssl rand -hex 32)\n    \
                 3. Enable OAuth:        RUSTARR_MCP_AUTH_MODE=oauth\n    \
                 4. Upstream gateway:    RUSTARR_NOAUTH=true"
            ),
        ),
    }
}
