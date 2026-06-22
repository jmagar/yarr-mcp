//! Status / version / health models — the `service_status` surface each
//! `ServiceKind` exposes via its `default_status_path`.
//!
//! Every supported kind has at least one model here, including the two
//! generic-passthrough-only kinds (Bazarr, Tracearr) whose curated command
//! surface is empty but whose status endpoint still has a knowable shape.
//!
//! Status payloads diverge completely between families, so there is no single
//! shared struct: the Servarr trio (Sonarr/Radarr/Prowlarr) share one
//! `/system/status` schema, while Overseerr, SABnzbd, Jellyfin, Bazarr, and
//! Tracearr each report differently. (qBittorrent's `/api/v2/app/version` returns
//! a bare `text/plain` version string, not JSON, so it has no struct here — read
//! it as a string.)

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `GET /api/v{1,3}/system/status` — shared by Sonarr, Radarr, and Prowlarr
/// (the Servarr family emits the same status schema across the trio). Slimmed to
/// the stable identity/version/runtime fields; the live endpoint returns ~30
/// more (paths, capability booleans) that deserialize-and-ignore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServarrSystemStatus {
    pub app_name: Option<String>,
    pub instance_name: Option<String>,
    pub version: Option<String>,
    pub build_time: Option<String>,
    pub branch: Option<String>,
    pub runtime_version: Option<String>,
    pub runtime_name: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub is_docker: Option<bool>,
    pub package_version: Option<String>,
    pub sqlite_version: Option<String>,
    pub migration_version: Option<i64>,
}

/// `GET /api/v1/status` — Overseerr's version/update report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverseerrStatus {
    pub version: Option<String>,
    pub commit_tag: Option<String>,
    pub update_available: Option<bool>,
    pub commits_behind: Option<i64>,
    pub restart_required: Option<bool>,
}

/// `GET /api?mode=version&output=json` — SABnzbd's version probe (`{version}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SabVersion {
    pub version: Option<String>,
}

/// `GET /System/Info/Public` — Jellyfin's unauthenticated public server info.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct JellyfinPublicInfo {
    pub server_name: Option<String>,
    pub version: Option<String>,
    pub product_name: Option<String>,
    pub operating_system: Option<String>,
    pub id: Option<String>,
    pub startup_wizard_completed: Option<bool>,
    pub local_address: Option<String>,
}

/// `GET /api/system/status` — Bazarr's status report. Bazarr wraps the payload in
/// a `{ "data": { … } }` envelope and reports the versions of the components it
/// bridges (itself, Sonarr, Radarr) plus host info. Generic-passthrough-only kind
/// (no curated commands); this models the one endpoint rustarr probes for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BazarrStatus {
    pub data: Option<BazarrStatusData>,
}

/// The `data` object inside [`BazarrStatus`]. Fields are snake_case on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BazarrStatusData {
    pub bazarr_version: Option<String>,
    pub sonarr_version: Option<String>,
    pub radarr_version: Option<String>,
    pub operating_system: Option<String>,
    pub python_version: Option<String>,
    pub bazarr_directory: Option<String>,
    pub bazarr_config_directory: Option<String>,
}

/// `GET /health` — Tracearr's health probe. Tracearr is a niche custom service
/// with no published OpenAPI; this models the conventional health-endpoint shape
/// (`{status, …}`) conservatively. Generic-passthrough-only kind — treat the
/// fields as best-effort and prefer the generic `api_get` passthrough for
/// anything richer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TracearrHealth {
    pub status: Option<String>,
    pub version: Option<String>,
    pub uptime: Option<String>,
}

#[cfg(test)]
#[path = "system_tests.rs"]
mod tests;
