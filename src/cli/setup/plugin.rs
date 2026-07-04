//! Plugin-option ingestion: copy `CLAUDE_PLUGIN_OPTION_*` env vars injected by
//! Claude Code into the `YARR_*` env vars that `Config::load` reads, and
//! bridge per-service credentials to the bundled fallback skills' config files.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Mapping from `CLAUDE_PLUGIN_OPTION_<OPTION>` to the `YARR_*` env var name.
///
/// Each `CLAUDE_PLUGIN_OPTION_*` var is injected by Claude Code from the
/// plugin's `userConfig` fields; we copy the non-empty ones into the
/// `YARR_*` env vars that `Config::load` reads.
///
/// When adding or renaming a `userConfig` field in `plugin.json`, update this
/// table to match.
const PLUGIN_OPTION_MAP: &[(&str, &str)] = &[
    ("CLAUDE_PLUGIN_OPTION_API_TOKEN", "YARR_MCP_TOKEN"),
    ("CLAUDE_PLUGIN_OPTION_SERVER_URL", "YARR_SERVER_URL"),
    ("CLAUDE_PLUGIN_OPTION_YARR_SERVICES", "YARR_SERVICES"),
    ("CLAUDE_PLUGIN_OPTION_SONARR_URL", "YARR_SONARR_URL"),
    ("CLAUDE_PLUGIN_OPTION_SONARR_API_KEY", "YARR_SONARR_API_KEY"),
    ("CLAUDE_PLUGIN_OPTION_RADARR_URL", "YARR_RADARR_URL"),
    ("CLAUDE_PLUGIN_OPTION_RADARR_API_KEY", "YARR_RADARR_API_KEY"),
    ("CLAUDE_PLUGIN_OPTION_PROWLARR_URL", "YARR_PROWLARR_URL"),
    (
        "CLAUDE_PLUGIN_OPTION_PROWLARR_API_KEY",
        "YARR_PROWLARR_API_KEY",
    ),
    ("CLAUDE_PLUGIN_OPTION_OVERSEERR_URL", "YARR_OVERSEERR_URL"),
    (
        "CLAUDE_PLUGIN_OPTION_OVERSEERR_API_KEY",
        "YARR_OVERSEERR_API_KEY",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_QBITTORRENT_URL",
        "YARR_QBITTORRENT_URL",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_QBITTORRENT_USERNAME",
        "YARR_QBITTORRENT_USERNAME",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_QBITTORRENT_PASSWORD",
        "YARR_QBITTORRENT_PASSWORD",
    ),
    ("CLAUDE_PLUGIN_OPTION_PLEX_URL", "YARR_PLEX_URL"),
    ("CLAUDE_PLUGIN_OPTION_PLEX_TOKEN", "YARR_PLEX_TOKEN"),
    ("CLAUDE_PLUGIN_OPTION_JELLYFIN_URL", "YARR_JELLYFIN_URL"),
    (
        "CLAUDE_PLUGIN_OPTION_JELLYFIN_API_KEY",
        "YARR_JELLYFIN_API_KEY",
    ),
    ("CLAUDE_PLUGIN_OPTION_AUTH_MODE", "YARR_MCP_AUTH_MODE"),
    ("CLAUDE_PLUGIN_OPTION_NO_AUTH", "YARR_MCP_NO_AUTH"),
    ("CLAUDE_PLUGIN_OPTION_PUBLIC_URL", "YARR_MCP_PUBLIC_URL"),
    (
        "CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_ID",
        "YARR_MCP_GOOGLE_CLIENT_ID",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_SECRET",
        "YARR_MCP_GOOGLE_CLIENT_SECRET",
    ),
    (
        "CLAUDE_PLUGIN_OPTION_AUTH_ADMIN_EMAIL",
        "YARR_MCP_AUTH_ADMIN_EMAIL",
    ),
];

/// Copy `CLAUDE_PLUGIN_OPTION_*` values into the corresponding `YARR_*` env
/// vars so that a subsequent `Config::load` picks them up.
///
/// Must run **before** `Config::load()` (see `main.rs::run_cli`) — config is
/// loaded once up front, so setting these vars later has no effect. Values
/// containing newlines or carriage returns are rejected (skipped), matching the
/// `reject_unsafe_value` guard in the former bash adapter. Empty values are
/// skipped so an unset plugin option never clobbers an existing env value.
pub fn apply_plugin_options() {
    for (option_var, yarr_var) in PLUGIN_OPTION_MAP {
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
            eprintln!("yarr setup: {option_var} must not contain newlines; skipping");
            continue;
        }
        // SAFETY: the binary entrypoint calls this before `Config::load()` and
        // before constructing the Tokio runtime, so no runtime worker can
        // concurrently read environment variables.
        unsafe {
            std::env::set_var(yarr_var, value);
        }
    }
}

/// Service prefixes whose `CLAUDE_PLUGIN_OPTION_<PREFIX>_*` values are bridged
/// to `~/.config/lab-<prefix>/config.env` so the per-service skills bundled in
/// the yarr plugin can talk to each service over HTTP when the MCP server is
/// unavailable. The skill scripts read those env files and expect the env-var
/// names to match their `userConfig` keys (e.g. `SONARR_URL`, `RADARR_API_KEY`).
const SKILL_SERVICES: &[&str] = &[
    "sonarr",
    "radarr",
    "prowlarr",
    "overseerr",
    "sabnzbd",
    "qbittorrent",
    "plex",
    "jellyfin",
    "tautulli",
    "tracearr",
    "bazarr",
];

/// Base directory the skills read their `lab-<service>/config.env` files from:
/// `$XDG_CONFIG_HOME` if set and non-empty, else `$HOME/.config`.
fn skill_config_base() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
}

/// Group `CLAUDE_PLUGIN_OPTION_*` values by service prefix into the body of each
/// `config.env` file. Values are single-quoted with embedded quotes escaped, so
/// the file is safe to `source` from the skills' shell scripts. Non-service
/// options (e.g. `SERVER_URL`, `API_TOKEN`) and empty/multi-line values are
/// skipped. Pure (no I/O) so it is unit-testable without touching the process
/// environment.
fn build_skill_fallback_bodies<I>(vars: I) -> BTreeMap<&'static str, String>
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut per_service: BTreeMap<&'static str, String> = BTreeMap::new();
    for (name, value) in vars {
        let Some(opt) = name.strip_prefix("CLAUDE_PLUGIN_OPTION_") else {
            continue;
        };
        if value.is_empty() || value.contains('\n') || value.contains('\r') {
            continue;
        }
        let lower = opt.to_ascii_lowercase();
        for service in SKILL_SERVICES {
            if lower
                .strip_prefix(service)
                .is_some_and(|rest| rest.starts_with('_'))
            {
                let escaped = value.replace('\'', "'\\''");
                per_service
                    .entry(service)
                    .or_default()
                    .push_str(&format!("{opt}='{escaped}'\n"));
                break;
            }
        }
    }
    per_service
}

/// Write per-service `lab-<service>/config.env` files from the current
/// `CLAUDE_PLUGIN_OPTION_*` environment so the bundled fallback skills share the
/// yarr plugin's credentials. Files are written atomically with mode `0600`
/// and their directories with `0700`. Best-effort: the caller logs any error and
/// never fails the plugin hook over it.
pub(crate) fn write_skill_fallback_config() -> std::io::Result<()> {
    let Some(base) = skill_config_base() else {
        return Ok(());
    };
    let bodies = build_skill_fallback_bodies(std::env::vars());
    for (service, body) in bodies {
        let dir = base.join(format!("lab-{service}"));
        std::fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        }
        let path = dir.join("config.env");
        let tmp = dir.join(format!("config.env.tmp.{}", std::process::id()));
        let contents = format!(
            "# Generated by yarr plugin-hook for the bundled fallback skills. Do not edit by hand.\n{body}"
        );
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;

            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .mode(0o600)
                .open(&tmp)?;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
        }
        #[cfg(not(unix))]
        std::fs::write(&tmp, contents.as_bytes())?;
        std::fs::rename(&tmp, &path).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "plugin_tests.rs"]
mod tests;
