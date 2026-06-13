//! Configuration structs for the Rustarr MCP server.
//!
//! Values are loaded in priority order:
//!   1. `config.toml` (checked in, defaults only — no secrets)
//!   2. Environment variables (`RUSTARR_*`, `RUSTARR_MCP_*`)
//!
//! Service credentials are loaded from `RUSTARR_SERVICES` plus per-service
//! `RUSTARR_<NAME>_*` environment variables.

use serde::{Deserialize, Serialize};

const SERVICE_HOME_DIRNAME: &str = ".rustarr";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ServiceConfig {
    pub name: String,
    pub kind: ServiceKind,
    pub base_url: String,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: ServiceKind::Sonarr,
            base_url: String::new(),
            api_key: None,
            username: None,
            password: None,
            token: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceKind {
    Sonarr,
    Radarr,
    Prowlarr,
    Tautulli,
    Overseerr,
    Bazarr,
    Tracearr,
    Lidarr,
    Readarr,
    Sabnzbd,
    Qbittorrent,
    Wizarr,
    Notifiarr,
    Plex,
    Jellyfin,
}

impl ServiceKind {
    pub const ALL: [Self; 15] = [
        Self::Sonarr,
        Self::Radarr,
        Self::Prowlarr,
        Self::Tautulli,
        Self::Overseerr,
        Self::Bazarr,
        Self::Tracearr,
        Self::Lidarr,
        Self::Readarr,
        Self::Sabnzbd,
        Self::Qbittorrent,
        Self::Wizarr,
        Self::Notifiarr,
        Self::Plex,
        Self::Jellyfin,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sonarr => "sonarr",
            Self::Radarr => "radarr",
            Self::Prowlarr => "prowlarr",
            Self::Tautulli => "tautulli",
            Self::Overseerr => "overseerr",
            Self::Bazarr => "bazarr",
            Self::Tracearr => "tracearr",
            Self::Lidarr => "lidarr",
            Self::Readarr => "readarr",
            Self::Sabnzbd => "sabnzbd",
            Self::Qbittorrent => "qbittorrent",
            Self::Wizarr => "wizarr",
            Self::Notifiarr => "notifiarr",
            Self::Plex => "plex",
            Self::Jellyfin => "jellyfin",
        }
    }

    pub fn default_status_path(self) -> &'static str {
        match self {
            Self::Sonarr | Self::Radarr => "/api/v3/system/status",
            Self::Prowlarr | Self::Lidarr | Self::Readarr => "/api/v1/system/status",
            Self::Overseerr => "/api/v1/status",
            Self::Sabnzbd => "/api?mode=version",
            Self::Qbittorrent => "/api/v2/app/version",
            Self::Jellyfin => "/System/Info/Public",
            Self::Plex => "/identity",
            Self::Tautulli => "/api/v2?cmd=get_server_info",
            Self::Bazarr => "/api/system/status",
            Self::Tracearr => "/health",
            Self::Wizarr => "/api/status",
            Self::Notifiarr => "/api/ping",
        }
    }
}

impl std::str::FromStr for ServiceKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> anyhow::Result<Self> {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "sonarr" => Ok(Self::Sonarr),
            "radarr" => Ok(Self::Radarr),
            "prowlarr" => Ok(Self::Prowlarr),
            "tautulli" => Ok(Self::Tautulli),
            "overseerr" => Ok(Self::Overseerr),
            "bazarr" => Ok(Self::Bazarr),
            "tracearr" => Ok(Self::Tracearr),
            "lidarr" => Ok(Self::Lidarr),
            "readarr" => Ok(Self::Readarr),
            "sabnzbd" => Ok(Self::Sabnzbd),
            "qbittorrent" | "qbit" | "qb" => Ok(Self::Qbittorrent),
            "wizarr" => Ok(Self::Wizarr),
            "notifiarr" => Ok(Self::Notifiarr),
            "plex" => Ok(Self::Plex),
            "jellyfin" => Ok(Self::Jellyfin),
            other => anyhow::bail!("unknown rustarr service kind: {other}"),
        }
    }
}

/// MCP HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    /// Bind host (RUSTARR_MCP_HOST). Default: `127.0.0.1` (loopback).
    /// Set to `0.0.0.0` to listen on all interfaces — requires auth configured.
    #[serde(default = "default_mcp_host")]
    pub host: String,
    /// Bind port (RUSTARR_MCP_PORT). Default: `40070`.
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    /// MCP server name advertised to clients (RUSTARR_MCP_SERVER_NAME).
    #[serde(default = "default_server_name")]
    pub server_name: String,
    /// Disable auth entirely — only safe when bound to loopback (RUSTARR_MCP_NO_AUTH).
    pub no_auth: bool,
    /// Allow unauthenticated access on non-loopback when behind a trusted reverse proxy
    /// that enforces its own auth (RUSTARR_NOAUTH). Loaded here so it participates in
    /// typed config rather than being a raw env read at call sites.
    pub trusted_gateway: bool,
    /// Static bearer token for simple auth (RUSTARR_MCP_TOKEN).
    pub api_token: Option<String>,
    /// Additional allowed Host header values (comma-separated in env).
    pub allowed_hosts: Vec<String>,
    /// Additional allowed CORS origins (comma-separated in env).
    pub allowed_origins: Vec<String>,
    /// OAuth sub-config (nested under `[mcp.auth]` in config.toml).
    pub auth: AuthConfig,
}

impl McpConfig {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Return true if the configured bind host resolves to a loopback address.
    ///
    /// Uses `IpAddr::is_loopback()` for numeric addresses. Accepts "localhost"
    /// as a canonical loopback hostname. Any other hostname or parse failure is
    /// treated as non-loopback — callers must not assume safety in that case.
    pub fn is_loopback(&self) -> bool {
        let host = &self.host;
        // Match "localhost" literal and numeric loopback addresses.
        // Strip bracket notation ([::1]) before parsing so IPv6 loopback works.
        host == "localhost"
            || host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<std::net::IpAddr>()
                .map(|ip| ip.is_loopback())
                .unwrap_or(false)
    }
}

/// OAuth / JWT auth sub-config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub public_url: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub admin_email: String,
    pub allowed_emails: Vec<String>,
    pub sqlite_path: String,
    pub key_path: String,
    pub access_token_ttl_secs: u64,
    pub refresh_token_ttl_secs: u64,
    pub auth_code_ttl_secs: u64,
    pub register_rpm: u32,
    pub authorize_rpm: u32,
    pub allowed_client_redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    #[default]
    Bearer,
    OAuth,
}

// ── defaults ──────────────────────────────────────────────────────────────────

fn default_mcp_host() -> String {
    // Default to loopback for safety. Operators who need external access must
    // explicitly set RUSTARR_MCP_HOST=0.0.0.0 (and configure auth).
    "127.0.0.1".into()
}
fn default_mcp_port() -> u16 {
    40070
}
fn default_server_name() -> String {
    "rustarr-mcp".into()
}
fn default_auth_sqlite_path() -> String {
    "/data/auth.db".into()
}
fn default_auth_key_path() -> String {
    "/data/auth-jwt.pem".into()
}
fn default_access_token_ttl_secs() -> u64 {
    3600
}
fn default_refresh_token_ttl_secs() -> u64 {
    86400 * 30
}
fn default_auth_code_ttl_secs() -> u64 {
    300
}
fn default_register_rpm() -> u32 {
    10
}
fn default_authorize_rpm() -> u32 {
    60
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            host: default_mcp_host(),
            port: default_mcp_port(),
            server_name: default_server_name(),
            no_auth: false,
            trusted_gateway: false,
            api_token: None,
            allowed_hosts: Vec::new(),
            allowed_origins: Vec::new(),
            auth: AuthConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::default(),
            public_url: None,
            google_client_id: None,
            google_client_secret: None,
            admin_email: String::new(),
            allowed_emails: Vec::new(),
            sqlite_path: default_auth_sqlite_path(),
            key_path: default_auth_key_path(),
            access_token_ttl_secs: default_access_token_ttl_secs(),
            refresh_token_ttl_secs: default_refresh_token_ttl_secs(),
            auth_code_ttl_secs: default_auth_code_ttl_secs(),
            register_rpm: default_register_rpm(),
            authorize_rpm: default_authorize_rpm(),
            allowed_client_redirect_uris: Vec::new(),
        }
    }
}

// ── Appdata directory ─────────────────────────────────────────────────────────

/// Return the default local data directory for this service.
///
/// Pattern §25 + §28: The same `.env` and `config.toml` in `~/.<service>/`
/// work for both Docker and bare-metal deployment without modification.
///
/// | Environment   | Path                                |
/// |---------------|-------------------------------------|
/// | Container     | `/data` (bind-mounted from host)     |
/// | Bare-metal    | `~/.rustarr` (user home dir)        |
///
pub fn default_data_dir() -> anyhow::Result<std::path::PathBuf> {
    // Running inside a Docker container — /data is always the mount point.
    // Detection uses /.dockerenv (created by the Docker runtime) or an explicit
    // RUNNING_IN_CONTAINER env var (useful for testing or systemd-nspawn).
    if std::path::Path::new("/.dockerenv").exists()
        || std::env::var("RUNNING_IN_CONTAINER").is_ok()
        || std::env::var("container").is_ok()
    {
        return Ok(std::path::PathBuf::from("/data"));
    }

    // Bare-metal or local dev — use ~/.<service>/
    let home = dirs::home_dir().ok_or_else(|| {
        anyhow::anyhow!("cannot determine home directory — set HOME or RUNNING_IN_CONTAINER=1")
    })?;
    Ok(home.join(SERVICE_HOME_DIRNAME))
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
        if let Ok(v) = std::env::var("RUSTARR_MCP_AUTH_MODE") {
            if !v.is_empty() {
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
        }

        load_services_from_env(&mut config.rustarr)?;

        Ok(config)
    }
}

fn load_dotenv_defaults() -> anyhow::Result<()> {
    let data_dir = if let Some(value) = std::env::var_os("RUSTARR_HOME") {
        std::path::PathBuf::from(value)
    } else {
        default_data_dir()?
    };
    let path = data_dir.join(".env");
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(anyhow::anyhow!(
                "Failed to read {}: {error}",
                path.display()
            ))
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
        if key.is_empty() || key.contains(char::is_whitespace) {
            anyhow::bail!("{}:{}: invalid env key", path.display(), line_no + 1);
        }
        if std::env::var_os(key).is_some() {
            continue;
        }
        std::env::set_var(key, parse_dotenv_value(raw_value.trim())?);
    }
    Ok(())
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

fn load_services_from_env(config: &mut RustarrConfig) -> anyhow::Result<()> {
    let Ok(raw_names) = std::env::var("RUSTARR_SERVICES") else {
        return Ok(());
    };
    let mut services = Vec::new();
    for raw_name in raw_names
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let env_name = raw_name
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect::<String>();
        let kind = std::env::var(format!("RUSTARR_{env_name}_KIND"))
            .unwrap_or_else(|_| raw_name.to_owned())
            .parse::<ServiceKind>()?;
        let service = ServiceConfig {
            name: raw_name.to_ascii_lowercase(),
            kind,
            base_url: std::env::var(format!("RUSTARR_{env_name}_URL")).unwrap_or_default(),
            api_key: env_optional(&format!("RUSTARR_{env_name}_API_KEY")),
            username: env_optional(&format!("RUSTARR_{env_name}_USERNAME")),
            password: env_optional(&format!("RUSTARR_{env_name}_PASSWORD")),
            token: env_optional(&format!("RUSTARR_{env_name}_TOKEN")),
        };
        services.push(service);
    }
    if !services.is_empty() {
        config.services = services;
    }
    Ok(())
}

fn env_optional(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

// ── env helpers ───────────────────────────────────────────────────────────────

fn env_str(key: &str, target: &mut String) {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            *target = v;
        }
    }
}

fn env_opt_str(key: &str, target: &mut Option<String>) {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            *target = Some(v);
        }
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, target: &mut T) -> anyhow::Result<()> {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            *target = v
                .parse()
                .map_err(|_| anyhow::anyhow!("{key}: invalid value {v:?}"))?;
        }
    }
    Ok(())
}

fn env_bool(key: &str, target: &mut bool) -> anyhow::Result<()> {
    if let Ok(v) = std::env::var(key) {
        match v.to_lowercase().as_str() {
            "1" | "true" | "yes" => *target = true,
            "0" | "false" | "no" => *target = false,
            other => anyhow::bail!("{key}: expected bool, got {other:?}"),
        }
    }
    Ok(())
}

fn env_list(key: &str, target: &mut Vec<String>) {
    if let Ok(v) = std::env::var(key) {
        let items: Vec<String> = v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !items.is_empty() {
            *target = items;
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
