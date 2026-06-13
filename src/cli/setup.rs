//! Plugin setup and repair commands.
//!
//! These are operational commands that check and mutate appdata, write .env
//! files, and validate auth/port configuration before the server starts.
//! Business logic stays in `app.rs`; this module is allowed to touch the
//! filesystem and network only for diagnostics and setup purposes.

use std::net::TcpListener;
use std::path::{Path, PathBuf};

use crate::{
    config::{default_data_dir, AuthMode, Config},
    server::resolve_auth_policy_kind,
};
use anyhow::{bail, Result};

// ── public surface ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupCommand {
    Check,
    Repair,
    /// Copy this binary into ~/.local/bin so it is callable as a bare command
    /// in the user's own terminal, independent of Claude Code.
    Install,
    PluginHook {
        no_repair: bool,
    },
}

/// Mapping from `CLAUDE_PLUGIN_OPTION_<OPTION>` to the `RUSTARR_*` env var name.
///
/// This is the Rust port of the env-var block that previously lived in the
/// plugin's `hooks/plugin-setup.sh` adapter. Each `CLAUDE_PLUGIN_OPTION_*` var
/// is injected by Claude Code from the plugin's `userConfig` fields; we copy
/// the non-empty ones into the `RUSTARR_*` env vars that `Config::load` reads.
///
/// When adding or renaming a `userConfig` field in `plugin.json`, update this
/// table to match.
const PLUGIN_OPTION_MAP: &[(&str, &str)] = &[
    ("CLAUDE_PLUGIN_OPTION_API_TOKEN", "RUSTARR_MCP_TOKEN"),
    ("CLAUDE_PLUGIN_OPTION_SERVER_URL", "RUSTARR_SERVER_URL"),
    ("CLAUDE_PLUGIN_OPTION_RUSTARR_SERVICES", "RUSTARR_SERVICES"),
    ("CLAUDE_PLUGIN_OPTION_SONARR_URL", "RUSTARR_SONARR_URL"),
    (
        "CLAUDE_PLUGIN_OPTION_SONARR_API_KEY",
        "RUSTARR_SONARR_API_KEY",
    ),
    ("CLAUDE_PLUGIN_OPTION_RADARR_URL", "RUSTARR_RADARR_URL"),
    (
        "CLAUDE_PLUGIN_OPTION_RADARR_API_KEY",
        "RUSTARR_RADARR_API_KEY",
    ),
    ("CLAUDE_PLUGIN_OPTION_PROWLARR_URL", "RUSTARR_PROWLARR_URL"),
    (
        "CLAUDE_PLUGIN_OPTION_PROWLARR_API_KEY",
        "RUSTARR_PROWLARR_API_KEY",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_OVERSEERR_URL",
        "RUSTARR_OVERSEERR_URL",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_OVERSEERR_API_KEY",
        "RUSTARR_OVERSEERR_API_KEY",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_QBITTORRENT_URL",
        "RUSTARR_QBITTORRENT_URL",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_QBITTORRENT_USERNAME",
        "RUSTARR_QBITTORRENT_USERNAME",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_QBITTORRENT_PASSWORD",
        "RUSTARR_QBITTORRENT_PASSWORD",
    ),
    ("CLAUDE_PLUGIN_OPTION_PLEX_URL", "RUSTARR_PLEX_URL"),
    ("CLAUDE_PLUGIN_OPTION_PLEX_TOKEN", "RUSTARR_PLEX_TOKEN"),
    ("CLAUDE_PLUGIN_OPTION_JELLYFIN_URL", "RUSTARR_JELLYFIN_URL"),
    (
        "CLAUDE_PLUGIN_OPTION_JELLYFIN_API_KEY",
        "RUSTARR_JELLYFIN_API_KEY",
    ),
    ("CLAUDE_PLUGIN_OPTION_AUTH_MODE", "RUSTARR_MCP_AUTH_MODE"),
    ("CLAUDE_PLUGIN_OPTION_NO_AUTH", "RUSTARR_MCP_NO_AUTH"),
    ("CLAUDE_PLUGIN_OPTION_PUBLIC_URL", "RUSTARR_MCP_PUBLIC_URL"),
    (
        "CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_ID",
        "RUSTARR_MCP_GOOGLE_CLIENT_ID",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_SECRET",
        "RUSTARR_MCP_GOOGLE_CLIENT_SECRET",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_AUTH_ADMIN_EMAIL",
        "RUSTARR_MCP_AUTH_ADMIN_EMAIL",
    ),
];

/// Copy `CLAUDE_PLUGIN_OPTION_*` values into the corresponding `RUSTARR_*` env
/// vars so that a subsequent `Config::load` picks them up.
///
/// Must run **before** `Config::load()` (see `main.rs::run_cli`) — config is
/// loaded once up front, so setting these vars later has no effect. Values
/// containing newlines or carriage returns are rejected (skipped), matching the
/// `reject_unsafe_value` guard in the former bash adapter. Empty values are
/// skipped so an unset plugin option never clobbers an existing env value.
pub fn apply_plugin_options() {
    for (option_var, rustarr_var) in PLUGIN_OPTION_MAP {
        let Some(value) = std::env::var_os(option_var) else {
            continue;
        };
        let Some(value) = value.to_str() else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        if value.contains('\n') || value.contains('\r') {
            eprintln!("rustarr setup: {option_var} must not contain newlines; skipping");
            continue;
        }
        // SAFETY (edition 2021): set_var is not marked unsafe on this edition.
        // This runs single-threaded before any worker threads are spawned.
        std::env::set_var(rustarr_var, value);
    }
}

pub async fn run_setup(config: &Config, command: SetupCommand) -> Result<()> {
    let report = match command {
        SetupCommand::Check => setup_check(config, true),
        SetupCommand::Repair => setup_repair(config)?,
        SetupCommand::Install => {
            let dest = install_self()?;
            println!("installed -> {}", dest.display());
            return Ok(());
        }
        SetupCommand::PluginHook { no_repair } => setup_plugin_hook(config, no_repair)?,
    };

    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.blocking_failures.is_empty() {
        bail!(
            "setup found {} blocking failure(s)",
            report.blocking_failures.len()
        );
    }
    Ok(())
}

// ── internal types ────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
struct SetupFailure {
    code: &'static str,
    message: String,
}

#[derive(Debug, serde::Serialize)]
struct SetupReport {
    exit_policy: &'static str,
    ran_repair: bool,
    no_repair: bool,
    blocking_failures: Vec<SetupFailure>,
    advisory_failures: Vec<SetupFailure>,
}

impl SetupReport {
    fn new(no_repair: bool) -> Self {
        Self {
            exit_policy: "success",
            ran_repair: false,
            no_repair,
            blocking_failures: Vec::new(),
            advisory_failures: Vec::new(),
        }
    }

    fn finish(mut self) -> Self {
        self.exit_policy = if !self.blocking_failures.is_empty() {
            "blocking_failure"
        } else if !self.advisory_failures.is_empty() {
            "advisory_failure"
        } else {
            "success"
        };
        self
    }
}

// ── setup logic ───────────────────────────────────────────────────────────────

/// Copy the running binary into `~/.local/bin/<name>` so it is callable as a
/// bare command in the user's own terminal, independent of Claude Code.
///
/// Uses the running executable's own file name as the destination, so this is
/// identical across every server repo. Copy (not symlink) so it survives
/// `/plugin update`, which changes the plugin cache path a symlink would dangle
/// to. Atomic via temp + rename; idempotent; depends only on std + anyhow.
fn install_self() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let name = exe
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine binary name from {}", exe.display()))?;
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    let bin_dir = PathBuf::from(home).join(".local").join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    let dest = bin_dir.join(name);

    // Running the already-installed copy: nothing to do.
    if dest == exe {
        return Ok(dest);
    }

    let tmp = bin_dir.join(format!(".{}.tmp", name.to_string_lossy()));
    std::fs::copy(&exe, &tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&tmp, &dest).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;

    let on_path = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d == bin_dir))
        .unwrap_or(false);
    if !on_path {
        eprintln!(
            "note: {} is not on your PATH; add:  export PATH=\"$HOME/.local/bin:$PATH\"",
            bin_dir.display()
        );
    }
    Ok(dest)
}

fn setup_plugin_hook(config: &Config, no_repair: bool) -> Result<SetupReport> {
    // Keep the user's terminal copy in ~/.local/bin fresh each session so it
    // survives `/plugin update`. Best-effort: never fail the hook over it.
    if let Err(e) = install_self() {
        eprintln!("setup plugin-hook: self-install skipped: {e}");
    }
    let initial = setup_check(config, no_repair);
    if initial.blocking_failures.is_empty() || no_repair {
        return Ok(initial);
    }
    setup_repair(config)
}

fn setup_check(config: &Config, no_repair: bool) -> SetupReport {
    let mut report = SetupReport::new(no_repair);
    let data_dir = match setup_data_dir() {
        Ok(d) => d,
        Err(e) => {
            report.blocking_failures.push(SetupFailure {
                code: "appdata_dir_unknown",
                message: format!(
                    "cannot determine appdata directory: {e} — set HOME or RUNNING_IN_CONTAINER=1"
                ),
            });
            return report.finish();
        }
    };

    if !data_dir.is_dir() {
        report.blocking_failures.push(SetupFailure {
            code: "appdata_missing",
            message: format!("appdata directory does not exist: {}", data_dir.display()),
        });
    }
    let env_path = data_dir.join(".env");
    if !env_path.is_file() {
        report.advisory_failures.push(SetupFailure {
            code: "env_file_missing",
            message: format!(
                "{} does not exist; setup repair will create one, but process env can supply values",
                env_path.display()
            ),
        });
    }
    if config.rustarr.services.is_empty() {
        report.blocking_failures.push(SetupFailure {
            code: "missing_rustarr_services",
            message: "RUSTARR_SERVICES or [rustarr.services] configuration is required".into(),
        });
    }

    check_auth(config, &mut report);
    check_port(&config.mcp.host, config.mcp.port, &mut report);

    report.finish()
}

fn setup_repair(config: &Config) -> Result<SetupReport> {
    // L8: Two concurrent `setup repair` invocations can clobber each other's
    // .env.tmp → .env rename. For a plugin hook triggered by a single Claude
    // session this is benign, but a proper fix would use flock(2) on the data dir.
    let data_dir = setup_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    write_env(&data_dir, config)?;

    // Re-run check after repair; `appdata_missing` is now resolved since
    // `create_dir_all` succeeded above.
    let mut report = setup_check(config, false);
    report.ran_repair = true;

    Ok(report.finish())
}

fn require_oauth_field(
    report: &mut SetupReport,
    value: &Option<String>,
    code: &'static str,
    message: &str,
) {
    if value.as_deref().unwrap_or("").is_empty() {
        report.blocking_failures.push(SetupFailure {
            code,
            message: message.into(),
        });
    }
}

fn check_auth(config: &Config, report: &mut SetupReport) {
    if let Err(error) = resolve_auth_policy_kind(config, config.mcp.trusted_gateway) {
        report.blocking_failures.push(SetupFailure {
            code: "invalid_auth_policy",
            message: error.to_string(),
        });
        return;
    }

    if config.mcp.no_auth {
        return;
    }

    if config.mcp.auth.mode == AuthMode::OAuth {
        require_oauth_field(
            report,
            &config.mcp.auth.public_url,
            "missing_oauth_public_url",
            "RUSTARR_MCP_PUBLIC_URL is required for OAuth mode",
        );
        require_oauth_field(
            report,
            &config.mcp.auth.google_client_id,
            "missing_oauth_client_id",
            "RUSTARR_MCP_GOOGLE_CLIENT_ID is required for OAuth mode",
        );
        require_oauth_field(
            report,
            &config.mcp.auth.google_client_secret,
            "missing_oauth_client_secret",
            "RUSTARR_MCP_GOOGLE_CLIENT_SECRET is required for OAuth mode",
        );
        require_oauth_field(
            report,
            &Some(config.mcp.auth.admin_email.clone()),
            "missing_oauth_admin_email",
            "RUSTARR_MCP_AUTH_ADMIN_EMAIL is required for OAuth mode",
        );
    } else if config.mcp.api_token.as_deref().unwrap_or("").is_empty() {
        report.blocking_failures.push(SetupFailure {
            code: "missing_mcp_token",
            message: "RUSTARR_MCP_TOKEN is required unless no_auth or OAuth mode is enabled".into(),
        });
    }
}

fn check_port(host: &str, port: u16, report: &mut SetupReport) {
    if let Err(error) = TcpListener::bind((host, port)) {
        report.blocking_failures.push(SetupFailure {
            code: "mcp_port_in_use",
            message: format!(
                "MCP bind address {host}:{port} is unavailable: {error}. Find the listener with: ss -tlnp | grep :{port}"
            ),
        });
    }
}

fn setup_data_dir() -> anyhow::Result<PathBuf> {
    // Writes go to the canonical service appdata dir — `~/.rustarr/` on bare
    // metal, `/data` in a container (see `default_data_dir`). This is the SAME
    // location the binary loads `.env` from (`config::load_dotenv_defaults`), so
    // the plugin hook's writes and the server's reads always agree.
    //
    // An explicit `RUSTARR_HOME` override is honored (used by tests).
    // `CLAUDE_PLUGIN_DATA` is intentionally NOT consulted: the plugin's sandboxed
    // data dir must not diverge from `~/.rustarr/`.
    if let Some(val) = std::env::var_os("RUSTARR_HOME") {
        return Ok(PathBuf::from(val));
    }
    default_data_dir()
}

fn write_env(data_dir: &Path, config: &Config) -> Result<()> {
    let service_names = config
        .rustarr
        .services
        .iter()
        .map(|service| service.name.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let mut lines = vec![
        dotenv_assignment("RUSTARR_SERVICES", &service_names)?,
        dotenv_assignment("RUSTARR_MCP_HOST", &config.mcp.host)?,
        dotenv_assignment("RUSTARR_MCP_PORT", &config.mcp.port.to_string())?,
        dotenv_assignment("RUSTARR_MCP_NO_AUTH", &config.mcp.no_auth.to_string())?,
    ];
    for service in &config.rustarr.services {
        let prefix = service.name.to_ascii_uppercase().replace('-', "_");
        lines.push(dotenv_assignment(
            &format!("RUSTARR_{prefix}_KIND"),
            service.kind.as_str(),
        )?);
        lines.push(dotenv_assignment(
            &format!("RUSTARR_{prefix}_URL"),
            &service.base_url,
        )?);
        if let Some(api_key) = &service.api_key {
            lines.push(dotenv_assignment(
                &format!("RUSTARR_{prefix}_API_KEY"),
                api_key,
            )?);
        }
        if let Some(username) = &service.username {
            lines.push(dotenv_assignment(
                &format!("RUSTARR_{prefix}_USERNAME"),
                username,
            )?);
        }
        if let Some(password) = &service.password {
            lines.push(dotenv_assignment(
                &format!("RUSTARR_{prefix}_PASSWORD"),
                password,
            )?);
        }
        if let Some(token) = &service.token {
            lines.push(dotenv_assignment(
                &format!("RUSTARR_{prefix}_TOKEN"),
                token,
            )?);
        }
    }

    if let Some(token) = config.mcp.api_token.as_deref().filter(|v| !v.is_empty()) {
        lines.push(dotenv_assignment("RUSTARR_MCP_TOKEN", token)?);
    }
    if config.mcp.auth.mode == AuthMode::OAuth {
        lines.push("RUSTARR_MCP_AUTH_MODE=oauth".into());
        if let Some(v) = &config.mcp.auth.public_url {
            lines.push(dotenv_assignment("RUSTARR_MCP_PUBLIC_URL", v)?);
        }
        if let Some(v) = &config.mcp.auth.google_client_id {
            lines.push(dotenv_assignment("RUSTARR_MCP_GOOGLE_CLIENT_ID", v)?);
        }
        if let Some(v) = &config.mcp.auth.google_client_secret {
            lines.push(dotenv_assignment("RUSTARR_MCP_GOOGLE_CLIENT_SECRET", v)?);
        }
        if !config.mcp.auth.admin_email.is_empty() {
            lines.push(dotenv_assignment(
                "RUSTARR_MCP_AUTH_ADMIN_EMAIL",
                &config.mcp.auth.admin_email,
            )?);
        }
    }

    let env_path = data_dir.join(".env");
    let temp_path = data_dir.join(".env.tmp");
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        // mode(0o600) sets permissions atomically at creation — no second chmod needed.
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&temp_path)?;
        writeln!(file, "{}", lines.join("\n"))?;
        file.sync_all()?;
    }
    #[cfg(not(unix))]
    std::fs::write(&temp_path, format!("{}\n", lines.join("\n")))?;
    std::fs::rename(&temp_path, &env_path).inspect_err(|_| {
        // Best-effort cleanup: remove the temp file if rename fails to avoid leaking
        // a partially-written file containing secrets.
        let _ = std::fs::remove_file(&temp_path);
    })?;
    Ok(())
}

fn dotenv_assignment(key: &str, value: &str) -> Result<String> {
    Ok(format!("{key}={}", dotenv_value(value)?))
}

fn dotenv_value(value: &str) -> Result<String> {
    if value.chars().any(|c| matches!(c, '\n' | '\r' | '\0')) {
        bail!("dotenv values cannot contain newlines or NUL bytes");
    }

    if value.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '-' | '.' | '/' | ':' | '@' | '%' | '+' | '=' | ',')
    }) {
        return Ok(value.to_string());
    }

    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!("\"{escaped}\""))
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
