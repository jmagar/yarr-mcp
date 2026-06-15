//! Configuration structs for the Rustarr MCP server.
//!
//! Values are loaded in priority order:
//!   1. `config.toml` (checked in, defaults only — no secrets)
//!   2. Environment variables (`RUSTARR_*`, `RUSTARR_MCP_*`)
//!
//! Service credentials are loaded from `RUSTARR_SERVICES` plus per-service
//! `RUSTARR_<NAME>_*` environment variables.
//!
//! This module is a facade: the concrete config types are split by concern into
//! the `config/` submodules and re-exported here so the rest of the crate keeps
//! importing them from `crate::config::*`.

use serde::{Deserialize, Serialize};

pub mod auth;
pub mod mcp;
pub mod services;

// Re-export the public config surface so existing `crate::config::*` import
// paths keep working unchanged.
pub use auth::{AuthConfig, AuthMode};
pub use mcp::McpConfig;
pub use services::{ServiceConfig, ServiceKind, default_data_dir, resolve_data_dir};

// Bring the private env helpers into this module's scope. They are used by
// `Config::load` below and exercised by the colocated tests via `super::*`.
use mcp::{env_bool, env_list, env_opt_str, env_parse, env_str};
use services::{SERVICE_HOME_DIRNAME, load_services_from_env};

/// Top-level config (maps to `config.toml` sections).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub mcp: McpConfig,
    pub rustarr: RustarrConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RustarrConfig {
    pub services: Vec<ServiceConfig>,
}

// ── Config loading ────────────────────────────────────────────────────────────

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Config::default();

        // Search for config.toml in priority order (§25: appdata convention):
        //   1. RUSTARR_CONFIG                         — explicit operator override
        //   2. RUSTARR_HOME/config.toml
        //   3. ~/<SERVICE_HOME_DIRNAME>/config.toml   — user's persistent config
        //
        // Deliberately do not read ./config.toml by default. This repo can contain
        // ignored local examples; loading them implicitly has caused stale identity
        // and unsafe bind-address drift in local runs.
        let candidate_paths = {
            let mut paths = vec![];
            if let Some(path) = std::env::var_os("RUSTARR_CONFIG") {
                paths.push(std::path::PathBuf::from(path));
            }
            if let Some(data_dir) = std::env::var_os("RUSTARR_HOME") {
                paths.push(std::path::PathBuf::from(data_dir).join("config.toml"));
            }
            if let Some(home) = std::env::var_os("HOME") {
                paths.push(
                    std::path::PathBuf::from(home)
                        .join(SERVICE_HOME_DIRNAME)
                        .join("config.toml"),
                );
            }
            paths
        };

        for path in &candidate_paths {
            match std::fs::read_to_string(path) {
                Ok(contents) => {
                    config = toml::from_str(&contents)
                        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {e}", path.display()))?;
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(anyhow::anyhow!("Failed to read {}: {e}", path.display())),
            }
        }

        load_dotenv_defaults()?;

        // Env overrides — RUSTARR_MCP_* for server config.
        env_str("RUSTARR_MCP_HOST", &mut config.mcp.host);
        env_parse("RUSTARR_MCP_PORT", &mut config.mcp.port)?;
        env_str("RUSTARR_MCP_SERVER_NAME", &mut config.mcp.server_name);
        env_bool("RUSTARR_MCP_NO_AUTH", &mut config.mcp.no_auth)?;
        env_bool("RUSTARR_NOAUTH", &mut config.mcp.trusted_gateway)?;
        env_opt_str("RUSTARR_MCP_TOKEN", &mut config.mcp.api_token);
        env_list("RUSTARR_MCP_ALLOWED_HOSTS", &mut config.mcp.allowed_hosts);
        env_list(
            "RUSTARR_MCP_ALLOWED_ORIGINS",
            &mut config.mcp.allowed_origins,
        );
        env_opt_str("RUSTARR_MCP_PUBLIC_URL", &mut config.mcp.auth.public_url);
        env_str(
            "RUSTARR_MCP_AUTH_ADMIN_EMAIL",
            &mut config.mcp.auth.admin_email,
        );
        env_opt_str(
            "RUSTARR_MCP_GOOGLE_CLIENT_ID",
            &mut config.mcp.auth.google_client_id,
        );
        env_opt_str(
            "RUSTARR_MCP_GOOGLE_CLIENT_SECRET",
            &mut config.mcp.auth.google_client_secret,
        );
        env_list(
            "RUSTARR_MCP_AUTH_ALLOWED_EMAILS",
            &mut config.mcp.auth.allowed_emails,
        );
        env_str(
            "RUSTARR_MCP_AUTH_SQLITE_PATH",
            &mut config.mcp.auth.sqlite_path,
        );
        env_str("RUSTARR_MCP_AUTH_KEY_PATH", &mut config.mcp.auth.key_path);
        env_parse(
            "RUSTARR_MCP_AUTH_ACCESS_TOKEN_TTL_SECS",
            &mut config.mcp.auth.access_token_ttl_secs,
        )?;
        env_parse(
            "RUSTARR_MCP_AUTH_REFRESH_TOKEN_TTL_SECS",
            &mut config.mcp.auth.refresh_token_ttl_secs,
        )?;
        env_parse(
            "RUSTARR_MCP_AUTH_CODE_TTL_SECS",
            &mut config.mcp.auth.auth_code_ttl_secs,
        )?;
        env_parse(
            "RUSTARR_MCP_AUTH_REGISTER_RPM",
            &mut config.mcp.auth.register_rpm,
        )?;
        env_parse(
            "RUSTARR_MCP_AUTH_AUTHORIZE_RPM",
            &mut config.mcp.auth.authorize_rpm,
        )?;
        env_list(
            "RUSTARR_MCP_AUTH_ALLOWED_CLIENT_REDIRECT_URIS",
            &mut config.mcp.auth.allowed_client_redirect_uris,
        );
        if let Ok(v) = std::env::var("RUSTARR_MCP_AUTH_MODE")
            && !v.is_empty()
        {
            config.mcp.auth.mode = match v.to_lowercase().as_str() {
                "oauth" => AuthMode::OAuth,
                "bearer" => AuthMode::Bearer,
                other => {
                    return Err(anyhow::anyhow!(
                        "invalid RUSTARR_MCP_AUTH_MODE {:?}: must be \"bearer\" or \"oauth\"",
                        other
                    ));
                }
            };
        }

        load_services_from_env(&mut config.rustarr)?;

        Ok(config)
    }
}

fn load_dotenv_defaults() -> anyhow::Result<()> {
    let data_dir = resolve_data_dir()?;
    let path = data_dir.join(".env");
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(anyhow::anyhow!(
                "Failed to read {}: {error}",
                path.display()
            ));
        }
    };
    for (line_no, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = line.split_once('=') else {
            anyhow::bail!("{}:{}: expected KEY=VALUE", path.display(), line_no + 1);
        };
        let key = key.trim();
        if key.is_empty() || key.contains(char::is_whitespace) || key.contains('\0') {
            anyhow::bail!("{}:{}: invalid env key", path.display(), line_no + 1);
        }
        // Only inject keys in rustarr's own namespace (plus the documented
        // `RUST_LOG`). A `.env` lives in a writable appdata dir, so without this
        // an attacker who can write it could smuggle in process-wide vars such as
        // `PATH`, `LD_PRELOAD`, or `SSL_CERT_FILE`. Skip-and-warn rather than bail
        // so an unexpected key never hard-fails startup.
        if !is_injectable_env_key(key) {
            tracing::warn!(
                key,
                file = %path.display(),
                "ignoring non-RUSTARR key in .env; only RUSTARR_* and RUST_LOG are loaded"
            );
            continue;
        }
        if std::env::var_os(key).is_some() {
            continue;
        }
        let value = parse_dotenv_value(raw_value.trim())?;
        if value.contains('\0') {
            anyhow::bail!(
                "{}:{}: env value contains a null byte",
                path.display(),
                line_no + 1
            );
        }
        // SAFETY: runs during early startup config load, before any task that
        // reads the process environment is spawned onto the runtime, so there is
        // no concurrent env access. (The tokio worker threads that exist at this
        // point are parked and do not touch the environment.)
        unsafe {
            std::env::set_var(key, value);
        }
    }
    Ok(())
}

/// Keys a `.env` file is allowed to inject into the process environment:
/// rustarr's own `RUSTARR_*` namespace, plus the documented `RUST_LOG`. Anything
/// else is skipped so a writable `.env` cannot smuggle in process-wide variables.
fn is_injectable_env_key(key: &str) -> bool {
    key.starts_with("RUSTARR_") || key == "RUST_LOG"
}

fn parse_dotenv_value(raw: &str) -> anyhow::Result<String> {
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        let inner = &raw[1..raw.len() - 1];
        let mut out = String::new();
        let mut chars = inner.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(ch);
            }
        }
        Ok(out)
    } else {
        Ok(raw.to_owned())
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
