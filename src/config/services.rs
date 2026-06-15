//! Service configuration: per-service [`ServiceConfig`], the [`ServiceKind`]
//! enum, env-based service loading, and the appdata directory resolver.

use serde::{Deserialize, Serialize};

pub(super) const SERVICE_HOME_DIRNAME: &str = ".rustarr";

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

// ── Service loading from env ────────────────────────────────────────────────────

pub(super) fn load_services_from_env(config: &mut super::RustarrConfig) -> anyhow::Result<()> {
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

#[cfg(test)]
#[path = "services_tests.rs"]
mod tests;
