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
                 → Or export in your shell: export {var_name}=<your_value>\n    \
                 TEMPLATE: Replace ~/.rustarr/ with your service data dir."
            ),
        )
    }
}

fn redact(s: &str) -> String {
    if s.len() <= 4 {
        return "*".repeat(s.len());
    }
    format!("{}****", &s[..4])
}

// ── Upstream connectivity ─────────────────────────────────────────────────────

/// Check that the upstream service is reachable via HTTP GET.
///
/// Attempts `GET <url>/health` with a 5-second timeout. Records round-trip
/// latency. This check is non-fatal (ok=true on timeout) — a misconfigured
/// upstream should not block the doctor report entirely.
///
/// # TEMPLATE
/// Replace `/health` with your upstream's actual health endpoint.
/// If your upstream is not HTTP (e.g. GraphQL, gRPC), adapt this check.
/// If the upstream requires auth, add the API key header:
///   `.header("x-api-key", api_key)`
// L22: A new reqwest::Client is built per invocation. Acceptable here (doctor
// is a one-shot CLI, not a request handler). Do NOT copy this pattern into
// hot paths — use a shared Client on AppState instead.
pub async fn check_upstream(base_url: &str) -> DoctorCheck {
    // TEMPLATE: Change "/health" to your upstream's actual probe path.
    let health_url = format!("{}/health", base_url.trim_end_matches('/'));

    // Use strict TLS by default. Set RUSTARR_DOCTOR_ACCEPT_INVALID_CERTS=true
    // only for dev environments with self-signed certificates.
    // TEMPLATE: Replace the env var prefix when adapting this template.
    let accept_invalid_certs = std::env::var("RUSTARR_DOCTOR_ACCEPT_INVALID_CERTS")
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false);
    let client = match reqwest::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .danger_accept_invalid_certs(accept_invalid_certs)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return DoctorCheck::fail(
                "connectivity",
                "Upstream reachable",
                format!("Could not build HTTP client: {e}"),
            );
        }
    };

    let start = Instant::now();
    match client.get(&health_url).send().await {
        Ok(resp) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let status = resp.status();
            if status.is_success() {
                DoctorCheck::pass_with_latency(
                    "connectivity",
                    "Upstream reachable",
                    format!("{health_url} → {status} ({elapsed} ms)"),
                    elapsed,
                )
            } else {
                DoctorCheck::fail_with_latency(
                    "connectivity",
                    "Upstream reachable",
                    format!(
                        "HTTP {status} from {health_url}\n    \
                         → Check that the upstream service is healthy.\n    \
                         TEMPLATE: Verify the correct health endpoint path."
                    ),
                    elapsed,
                )
            }
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            DoctorCheck::fail_with_latency(
                "connectivity",
                "Upstream reachable",
                format!(
                    "Could not reach {health_url}: {e}\n    \
                     → Check RUSTARR_API_URL is correct and the service is running.\n    \
                     TEMPLATE: Replace RUSTARR_API_URL with your service's env var."
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
/// # TEMPLATE
/// Port 3000 is the template default. Your service's port is in config.toml
/// `[mcp] port` (e.g. 6970 for unrust, 9158 for rustify).
pub fn check_port_available(host: &str, port: u16) -> DoctorCheck {
    let bind = format!("{host}:{port}");
    match TcpListener::bind((host, port)) {
        Ok(_) => DoctorCheck::pass("server", format!("MCP bind {bind}"), "available"),
        Err(e) => DoctorCheck::fail(
            "server",
            format!("MCP bind {bind}"),
            format!(
                "Bind address {bind} is unavailable: {e}\n    \
                 → Set RUSTARR_MCP_PORT to a different port.\n    \
                 → Or stop the process using this address: ss -tlnp | grep :{port}\n    \
                 TEMPLATE: Replace RUSTARR_MCP_PORT with your service prefix."
            ),
        ),
    }
}

// ── Auth configuration ────────────────────────────────────────────────────────

/// Check that the auth configuration is consistent and safe.
///
/// Validates:
/// - Binding 0.0.0.0 without auth is rejected (§27).
/// - Reports which auth mode is active.
/// - Warns if no auth is configured.
///
/// # TEMPLATE
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
                 4. Upstream gateway:    RUSTARR_NOAUTH=true\n    \
                 TEMPLATE: Replace RUSTARR_ prefix with your service prefix."
            ),
        ),
    }
}
