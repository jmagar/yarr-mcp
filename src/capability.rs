//! Capability model: the single source of truth for what each `ServiceKind`
//! can do and how it is addressed.
//!
//! Three previously-independent `ServiceKind` match sites (auth, query-injection,
//! path validation) re-expressed the same capability topology three different
//! ways. This module collapses that into one [`KindDescriptor`] table so adding a
//! kind is a single-line change rather than shotgun surgery.
//!
//! The inherent `impl ServiceKind` lives here (not in `config.rs`) to keep the
//! config module focused on config and this module the SSOT for capabilities.

use crate::config::ServiceKind;

/// Broad behavioural class a service belongs to. Curated commands target a
/// `Capability`, not a specific kind, so e.g. an `ArrManager` command works for
/// both Sonarr and Radarr without per-kind lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Sonarr / Radarr / Lidarr / Readarr — `/api/vN` resource managers.
    ArrManager,
    /// Prowlarr — indexer manager.
    Indexer,
    /// SABnzbd / qBittorrent — download clients.
    DownloadClient,
    /// Plex / Jellyfin — media servers.
    MediaServer,
    /// Overseerr — request manager.
    Requests,
    /// Tautulli — stats/analytics.
    Stats,
    /// Kinds with no curated command surface yet — generic passthrough only.
    GenericOnly,
}

/// How a service authenticates an HTTP request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStyle {
    /// `X-Api-Key` header (the *arr family, Overseerr, Bazarr, …).
    ApiKeyHeader,
    /// API key appended to the query string (SABnzbd, Tautulli).
    QueryApiKey,
    /// Username/password cookie session (qBittorrent).
    CookieSession,
    /// `X-Plex-Token` query parameter (Plex).
    PlexToken,
    /// `X-Emby-Token` header (Jellyfin).
    JellyfinToken,
}

/// Static, per-kind description of api versioning, auth, and resource shape.
/// Looked up once via [`ServiceKind::descriptor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindDescriptor {
    pub capability: Capability,
    /// Canonical API prefix, e.g. `/api/v3`. Empty for kinds with no fixed prefix.
    pub api_prefix: &'static str,
    pub auth_style: AuthStyle,
    /// Primary resource noun for *arr managers (`series`, `movie`, `artist`, …).
    pub resource_noun: Option<&'static str>,
    /// True when the API key travels in the query string rather than a header.
    pub query_api: bool,
}

impl ServiceKind {
    /// Broad capability class for this kind.
    pub fn capability(self) -> Capability {
        self.descriptor().capability
    }

    /// Full static descriptor for this kind. Single lookup site for auth style,
    /// api prefix, resource noun, and query-api topology.
    pub fn descriptor(self) -> KindDescriptor {
        match self {
            Self::Sonarr => KindDescriptor {
                capability: Capability::ArrManager,
                api_prefix: "/api/v3",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("series"),
                query_api: false,
            },
            Self::Radarr => KindDescriptor {
                capability: Capability::ArrManager,
                api_prefix: "/api/v3",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("movie"),
                query_api: false,
            },
            Self::Lidarr => KindDescriptor {
                capability: Capability::ArrManager,
                api_prefix: "/api/v1",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("artist"),
                query_api: false,
            },
            Self::Readarr => KindDescriptor {
                capability: Capability::ArrManager,
                api_prefix: "/api/v1",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("author"),
                query_api: false,
            },
            Self::Prowlarr => KindDescriptor {
                capability: Capability::Indexer,
                api_prefix: "/api/v1",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("indexer"),
                query_api: false,
            },
            Self::Overseerr => KindDescriptor {
                capability: Capability::Requests,
                api_prefix: "/api/v1",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("request"),
                query_api: false,
            },
            Self::Tautulli => KindDescriptor {
                capability: Capability::Stats,
                api_prefix: "/api/v2",
                auth_style: AuthStyle::QueryApiKey,
                resource_noun: None,
                query_api: true,
            },
            Self::Sabnzbd => KindDescriptor {
                capability: Capability::DownloadClient,
                api_prefix: "/api",
                auth_style: AuthStyle::QueryApiKey,
                resource_noun: None,
                query_api: true,
            },
            Self::Qbittorrent => KindDescriptor {
                capability: Capability::DownloadClient,
                api_prefix: "/api/v2",
                auth_style: AuthStyle::CookieSession,
                resource_noun: None,
                query_api: false,
            },
            Self::Plex => KindDescriptor {
                capability: Capability::MediaServer,
                api_prefix: "",
                auth_style: AuthStyle::PlexToken,
                resource_noun: None,
                query_api: true,
            },
            Self::Jellyfin => KindDescriptor {
                capability: Capability::MediaServer,
                api_prefix: "",
                auth_style: AuthStyle::JellyfinToken,
                resource_noun: None,
                query_api: false,
            },
            Self::Bazarr | Self::Tracearr | Self::Wizarr | Self::Notifiarr => KindDescriptor {
                capability: Capability::GenericOnly,
                api_prefix: "/api",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: None,
                query_api: false,
            },
        }
    }
}

#[cfg(test)]
#[path = "capability_tests.rs"]
mod tests;
