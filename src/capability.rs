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
    /// Path prefixes the generic passthrough is allowed to reach for this kind.
    ///
    /// Drives [`crate::rustarr::helpers::validate_service_path`]. This keeps the
    /// allowlist next to the rest of the kind's topology (LD3) while preserving
    /// the strict v1/v3 separation that `api_prefix` alone cannot express for
    /// media servers (Plex/Jellyfin use resource-noun roots, not `/api/vN`).
    pub path_allowlist: &'static [&'static str],
    /// True when this ArrManager kind exposes a SEPARATE metadata-profile axis
    /// (`/metadataprofile`) in addition to the quality-profile axis — i.e. the
    /// music/book kinds Lidarr and Readarr. Sonarr/Radarr have quality profiles
    /// ONLY.
    ///
    /// This is the typed seam (bead rustarr-zha.8) for expressing command
    /// applicability differences between the v3 and v1 arr kinds WITHOUT a
    /// per-(action, kind) deny list. The curated `set_quality` command targets the
    /// quality-profile axis, which all four arr kinds share, so it is universally
    /// applicable; a future metadata-profile command would gate on this flag
    /// rather than enumerating which kinds it does/doesn't support.
    pub has_metadata_profiles: bool,
}

impl KindDescriptor {
    /// True when the API key/token travels in the query string rather than a
    /// header — i.e. the `QueryApiKey` (SABnzbd, Tautulli) and `PlexToken` (Plex)
    /// auth styles. Derived from `auth_style` so it can never drift out of sync
    /// with the auth topology (previously a hand-maintained `query_api` field).
    pub fn query_api(self) -> bool {
        matches!(
            self.auth_style,
            AuthStyle::QueryApiKey | AuthStyle::PlexToken
        )
    }
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
                path_allowlist: &["/api/v3"],
                has_metadata_profiles: false,
            },
            Self::Radarr => KindDescriptor {
                capability: Capability::ArrManager,
                api_prefix: "/api/v3",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("movie"),
                path_allowlist: &["/api/v3"],
                has_metadata_profiles: false,
            },
            Self::Lidarr => KindDescriptor {
                capability: Capability::ArrManager,
                api_prefix: "/api/v1",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("artist"),
                path_allowlist: &["/api/v1"],
                // Music: Lidarr has BOTH quality and metadata profiles.
                has_metadata_profiles: true,
            },
            Self::Readarr => KindDescriptor {
                capability: Capability::ArrManager,
                api_prefix: "/api/v1",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("author"),
                path_allowlist: &["/api/v1"],
                // Books: Readarr has BOTH quality and metadata profiles.
                has_metadata_profiles: true,
            },
            Self::Prowlarr => KindDescriptor {
                capability: Capability::Indexer,
                api_prefix: "/api/v1",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("indexer"),
                path_allowlist: &["/api/v1"],
                has_metadata_profiles: false,
            },
            Self::Overseerr => KindDescriptor {
                capability: Capability::Requests,
                api_prefix: "/api/v1",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: Some("request"),
                path_allowlist: &["/api/v1"],
                has_metadata_profiles: false,
            },
            Self::Tautulli => KindDescriptor {
                capability: Capability::Stats,
                api_prefix: "/api/v2",
                auth_style: AuthStyle::QueryApiKey,
                resource_noun: None,
                path_allowlist: &["/api", "/api/v2"],
                has_metadata_profiles: false,
            },
            Self::Sabnzbd => KindDescriptor {
                capability: Capability::DownloadClient,
                api_prefix: "/api",
                auth_style: AuthStyle::QueryApiKey,
                resource_noun: None,
                path_allowlist: &["/api", "/api/v2"],
                has_metadata_profiles: false,
            },
            Self::Qbittorrent => KindDescriptor {
                capability: Capability::DownloadClient,
                api_prefix: "/api/v2",
                auth_style: AuthStyle::CookieSession,
                resource_noun: None,
                path_allowlist: &["/api/v2"],
                has_metadata_profiles: false,
            },
            Self::Plex => KindDescriptor {
                capability: Capability::MediaServer,
                api_prefix: "",
                auth_style: AuthStyle::PlexToken,
                resource_noun: None,
                path_allowlist: &["/identity", "/library", "/status", "/servers"],
                has_metadata_profiles: false,
            },
            Self::Jellyfin => KindDescriptor {
                capability: Capability::MediaServer,
                api_prefix: "",
                auth_style: AuthStyle::JellyfinToken,
                resource_noun: None,
                path_allowlist: &["/System", "/Items", "/Users", "/Library", "/Sessions"],
                has_metadata_profiles: false,
            },
            Self::Tracearr => KindDescriptor {
                capability: Capability::GenericOnly,
                api_prefix: "/api",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: None,
                path_allowlist: &["/health", "/api", "/api/v2"],
                has_metadata_profiles: false,
            },
            Self::Bazarr | Self::Wizarr | Self::Notifiarr => KindDescriptor {
                capability: Capability::GenericOnly,
                api_prefix: "/api",
                auth_style: AuthStyle::ApiKeyHeader,
                resource_noun: None,
                path_allowlist: &["/api", "/api/v2"],
                has_metadata_profiles: false,
            },
        }
    }
}

#[cfg(test)]
#[path = "capability_tests.rs"]
mod tests;
