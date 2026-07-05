//! Configuration structs for the Yarr MCP server.
//!
//! Values are loaded in priority order:
//!   1. `config.toml` (checked in, defaults only — no secrets)
//!   2. Environment variables (`YARR_*`, `YARR_MCP_*`)
//!
//! Service credentials are loaded from `YARR_SERVICES` plus per-service
//! `YARR_<NAME>_*` environment variables.
//!
//! This module is a facade: the concrete config types are split by concern into
//! the `config/` submodules and re-exported here so the rest of the crate keeps
//! importing them from `crate::config::*`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    #[serde(alias = "rustarr")]
    pub yarr: YarrConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct YarrConfig {
    pub services: Vec<ServiceConfig>,
}

// ── Config loading ────────────────────────────────────────────────────────────

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        migrate_legacy_process_env()?;
        let mut config = Config::default();

        // Search for config.toml in priority order (§25: appdata convention):
        //   1. YARR_CONFIG                         — explicit operator override
        //   2. YARR_HOME/config.toml
        //   3. ~/.yarr/config.toml                — user's persistent config
        //   4. ~/.rustarr/config.toml             — legacy fallback during rebrand
        //
        // Deliberately do not read ./config.toml by default. This repo can contain
        // ignored local examples; loading them implicitly has caused stale identity
        // and unsafe bind-address drift in local runs.
        let candidate_paths = config_candidate_paths();

        for path in &candidate_paths {
            match std::fs::read_to_string(path) {
                Ok(contents) => {
                    if path.ends_with(".rustarr/config.toml") {
                        tracing::warn!(
                            legacy = %path.display(),
                            "loading legacy rustarr config.toml during yarr migration; move it to ~/.yarr/config.toml"
                        );
                    }
                    config = toml::from_str(&contents)
                        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {e}", path.display()))?;
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(anyhow::anyhow!("Failed to read {}: {e}", path.display())),
            }
        }

        load_dotenv_defaults()?;

        // Env overrides — YARR_MCP_* for server config.
        env_str("YARR_MCP_HOST", &mut config.mcp.host);
        env_parse("YARR_MCP_PORT", &mut config.mcp.port)?;
        env_str("YARR_MCP_SERVER_NAME", &mut config.mcp.server_name);
        env_bool("YARR_MCP_NO_AUTH", &mut config.mcp.no_auth)?;
        env_bool("YARR_NOAUTH", &mut config.mcp.trusted_gateway)?;
        env_opt_str("YARR_MCP_TOKEN", &mut config.mcp.api_token);
        env_list("YARR_MCP_ALLOWED_HOSTS", &mut config.mcp.allowed_hosts);
        env_list("YARR_MCP_ALLOWED_ORIGINS", &mut config.mcp.allowed_origins);
        env_opt_str("YARR_MCP_PUBLIC_URL", &mut config.mcp.auth.public_url);
        env_str(
            "YARR_MCP_AUTH_ADMIN_EMAIL",
            &mut config.mcp.auth.admin_email,
        );
        env_opt_str(
            "YARR_MCP_GOOGLE_CLIENT_ID",
            &mut config.mcp.auth.google_client_id,
        );
        env_opt_str(
            "YARR_MCP_GOOGLE_CLIENT_SECRET",
            &mut config.mcp.auth.google_client_secret,
        );
        env_list(
            "YARR_MCP_AUTH_ALLOWED_EMAILS",
            &mut config.mcp.auth.allowed_emails,
        );
        env_str(
            "YARR_MCP_AUTH_SQLITE_PATH",
            &mut config.mcp.auth.sqlite_path,
        );
        env_str("YARR_MCP_AUTH_KEY_PATH", &mut config.mcp.auth.key_path);
        env_parse(
            "YARR_MCP_AUTH_ACCESS_TOKEN_TTL_SECS",
            &mut config.mcp.auth.access_token_ttl_secs,
        )?;
        env_parse(
            "YARR_MCP_AUTH_REFRESH_TOKEN_TTL_SECS",
            &mut config.mcp.auth.refresh_token_ttl_secs,
        )?;
        env_parse(
            "YARR_MCP_AUTH_CODE_TTL_SECS",
            &mut config.mcp.auth.auth_code_ttl_secs,
        )?;
        env_parse(
            "YARR_MCP_AUTH_REGISTER_RPM",
            &mut config.mcp.auth.register_rpm,
        )?;
        env_parse(
            "YARR_MCP_AUTH_AUTHORIZE_RPM",
            &mut config.mcp.auth.authorize_rpm,
        )?;
        env_list(
            "YARR_MCP_AUTH_ALLOWED_CLIENT_REDIRECT_URIS",
            &mut config.mcp.auth.allowed_client_redirect_uris,
        );
        if let Ok(v) = std::env::var("YARR_MCP_AUTH_MODE")
            && !v.is_empty()
        {
            config.mcp.auth.mode = match v.to_lowercase().as_str() {
                "oauth" => AuthMode::OAuth,
                "bearer" => AuthMode::Bearer,
                other => {
                    return Err(anyhow::anyhow!(
                        "invalid YARR_MCP_AUTH_MODE {:?}: must be \"bearer\" or \"oauth\"",
                        other
                    ));
                }
            };
        }

        load_services_from_env(&mut config.yarr)?;

        Ok(config)
    }
}

pub fn config_candidate_paths() -> Vec<std::path::PathBuf> {
    let mut paths = vec![];
    if let Some(path) = std::env::var_os("YARR_CONFIG") {
        paths.push(std::path::PathBuf::from(path));
    }
    if let Some(data_dir) = std::env::var_os("YARR_HOME") {
        paths.push(std::path::PathBuf::from(data_dir).join("config.toml"));
    } else if let Some(data_dir) = std::env::var_os("RUSTARR_HOME") {
        paths.push(std::path::PathBuf::from(data_dir).join("config.toml"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        paths.push(home.join(SERVICE_HOME_DIRNAME).join("config.toml"));
        paths.push(home.join(".rustarr").join("config.toml"));
    }
    paths
}

fn load_dotenv_defaults() -> anyhow::Result<()> {
    let data_dir = resolve_data_dir()?;
    migrate_legacy_dotenv(&data_dir)?;
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
    apply_dotenv_contents(&path, &contents)?;
    Ok(())
}

fn apply_dotenv_contents(path: &std::path::Path, contents: &str) -> anyhow::Result<()> {
    let mut pending = BTreeMap::new();
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
        // Only inject keys in yarr's own namespace (plus the documented
        // `RUST_LOG`). A `.env` lives in a writable appdata dir, so without this
        // an attacker who can write it could smuggle in process-wide vars such as
        // `PATH`, `LD_PRELOAD`, or `SSL_CERT_FILE`. Skip-and-warn rather than bail
        // so an unexpected key never hard-fails startup.
        if !is_injectable_env_key(key) {
            tracing::warn!(
                key,
                file = %path.display(),
                "ignoring unsupported key in .env; only YARR_*, legacy RUSTARR_*, and RUST_LOG are loaded"
            );
            continue;
        }
        let target_key = migrated_env_key(key);
        let value = parse_dotenv_value(raw_value.trim())?;
        if value.contains('\0') {
            anyhow::bail!(
                "{}:{}: env value contains a null byte",
                path.display(),
                line_no + 1
            );
        }
        if let Some(existing_value) = pending.get(&target_key)
            && existing_value != &value
        {
            anyhow::bail!(
                "{}:{}: conflicting values for {target_key} after legacy key migration",
                path.display(),
                line_no + 1
            );
        }
        pending.insert(target_key, value);
    }
    for (key, value) in pending {
        if std::env::var_os(&key).is_some() {
            continue;
        }
        // SAFETY: the binary entrypoint calls `Config::load()` before constructing
        // the Tokio runtime, and tests that mutate env use `testing::env_lock()`.
        // That keeps this process-global mutation serialized with env readers.
        unsafe {
            std::env::set_var(&key, value);
        }
    }
    Ok(())
}

/// Keys a `.env` file is allowed to inject into the process environment:
/// yarr's own `YARR_*` namespace, plus the documented `RUST_LOG`. Anything
/// else is skipped so a writable `.env` cannot smuggle in process-wide variables.
fn is_injectable_env_key(key: &str) -> bool {
    key.starts_with("YARR_") || key.starts_with("RUSTARR_") || key == "RUST_LOG"
}

fn migrated_env_key(key: &str) -> String {
    key.strip_prefix("RUSTARR_")
        .map(|suffix| format!("YARR_{suffix}"))
        .unwrap_or_else(|| key.to_owned())
}

fn migrate_legacy_process_env() -> anyhow::Result<()> {
    for (legacy_key, legacy_value) in
        std::env::vars().filter(|(key, _)| key.starts_with("RUSTARR_"))
    {
        let yarr_key = migrated_env_key(&legacy_key);
        match std::env::var(&yarr_key) {
            Ok(yarr_value) if yarr_value != legacy_value => {
                anyhow::bail!(
                    "conflicting legacy env {legacy_key} and new env {yarr_key}; unset one or make them match"
                );
            }
            Ok(_) => {}
            Err(std::env::VarError::NotPresent) => {
                tracing::warn!(
                    legacy = legacy_key,
                    target = yarr_key,
                    "using legacy RUSTARR_* environment variable during yarr migration"
                );
                // SAFETY: Config::load runs before runtime startup in production,
                // and tests hold ENV_LOCK while mutating process env.
                unsafe {
                    std::env::set_var(yarr_key, legacy_value);
                }
            }
            Err(std::env::VarError::NotUnicode(_)) => {
                anyhow::bail!("legacy env {legacy_key} contains non-unicode data");
            }
        }
    }
    Ok(())
}

fn migrate_legacy_dotenv(data_dir: &std::path::Path) -> anyhow::Result<()> {
    let yarr_dotenv = data_dir.join(".env");
    if yarr_dotenv.exists() {
        return Ok(());
    }
    let Some(home) = std::env::var_os("HOME") else {
        return Ok(());
    };
    let legacy_dotenv = std::path::PathBuf::from(home).join(".rustarr").join(".env");
    if !legacy_dotenv.exists() {
        return Ok(());
    }
    let contents = std::fs::read_to_string(&legacy_dotenv).map_err(|error| {
        anyhow::anyhow!(
            "Failed to read legacy env {}: {error}",
            legacy_dotenv.display()
        )
    })?;
    let migrated = contents
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                return line.to_owned();
            }
            match line.split_once('=') {
                Some((key, value)) => format!("{}={value}", migrated_env_key(key.trim())),
                None => line.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::create_dir_all(data_dir)
        .map_err(|error| anyhow::anyhow!("Failed to create {}: {error}", data_dir.display()))?;
    let tmp = data_dir.join(".env.migrating");
    write_private_file(&tmp, format!("{migrated}\n").as_bytes())
        .map_err(|error| anyhow::anyhow!("Failed to write {}: {error}", tmp.display()))?;
    std::fs::rename(&tmp, &yarr_dotenv).map_err(|error| {
        anyhow::anyhow!(
            "Failed to install migrated env {}: {error}",
            yarr_dotenv.display()
        )
    })?;
    tracing::warn!(
        legacy = %legacy_dotenv.display(),
        target = %yarr_dotenv.display(),
        "migrated legacy rustarr .env to yarr appdata"
    );
    Ok(())
}

#[cfg(unix)]
fn write_private_file(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents)
}

#[cfg(not(unix))]
fn write_private_file(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, contents)
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

/// True when `YARR_ALLOW_DESTRUCTIVE` is set truthy (`1`/`true`/`yes`/`on`).
///
/// A GLOBAL override that lets destructive operations run without per-call
/// confirmation: destructive generated `op`s and `api_delete` skip the `--confirm`
/// gate, destructive curated commands run immediately, and the Code Mode mid-script
/// delete refusal is lifted. Default-off; intended ONLY for dedicated, disposable
/// test stacks (e.g. the shart contract harness), never production.
pub fn destructive_allowed() -> bool {
    std::env::var("YARR_ALLOW_DESTRUCTIVE")
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "on"
        })
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
