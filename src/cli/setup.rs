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
    PluginHook { no_repair: bool },
}

pub async fn run_setup(config: &Config, command: SetupCommand) -> Result<()> {
    let report = match command {
        SetupCommand::Check => setup_check(config, true),
        SetupCommand::Repair => setup_repair(config)?,
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

fn setup_plugin_hook(config: &Config, no_repair: bool) -> Result<SetupReport> {
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
    // L11: setup_data_dir uses CLAUDE_PLUGIN_DATA/RUSTARR_HOME while Config::load
    // searches ~/.rustarr/config.toml first. In the plugin context CLAUDE_PLUGIN_DATA
    // and the config search path should coincide, but they can diverge in non-standard
    // deployments. TEMPLATE: align these when adapting the template.
    if let Some(val) =
        std::env::var_os("CLAUDE_PLUGIN_DATA").or_else(|| std::env::var_os("RUSTARR_HOME"))
    {
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
