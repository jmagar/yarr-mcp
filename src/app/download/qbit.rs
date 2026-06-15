//! qBittorrent DownloadClient impl (C5).
//!
//! qBittorrent's WebUI API is a `/api/v2` REST surface authed by a username/
//! password SID cookie (established in [`crate::rustarr::auth`], a dedicated
//! cookie-store client per the F2/S1 isolation fix). Reads are GETs; mutations
//! are `application/x-www-form-urlencoded` POSTs through
//! [`send_form_post`](crate::rustarr::RustarrClient::send_form_post), which
//! percent-encodes every field — callers never `format!` values into the body.
//!
//! API-VERSION FACT (bead, HIGH): qBittorrent **v5 renamed** the pause/resume
//! endpoints — `pause` → `POST /api/v2/torrents/stop`, `resume` →
//! `POST /api/v2/torrents/start`. The v4 `pause`/`resume` paths are GONE, so this
//! module targets the v5 `stop`/`start` names.

use anyhow::Result;
use serde_json::Value;

use crate::app::RustarrService;
use crate::config::ServiceConfig;
use crate::rustarr::slim;

/// Fields kept for a slimmed `/torrents/info` row — identify a torrent and reason
/// about its progress/throughput without the (large) full payload.
const TORRENT_FIELDS: &[&str] = &[
    "hash", "name", "state", "progress", "dlspeed", "size", "category",
];

/// Build `{api_prefix}{suffix}` for the qBittorrent service (descriptor-driven —
/// `/api/v2`, no hardcoded version). Pure for testability.
pub(super) fn qbit_path(config: &ServiceConfig, suffix: &str) -> String {
    format!("{}{}", config.kind.descriptor().api_prefix, suffix)
}

/// GET `/api/v2/torrents/info` → active torrents, slimmed to [`TORRENT_FIELDS`].
pub(super) async fn queue(svc: &RustarrService, config: &ServiceConfig) -> Result<Value> {
    let path = qbit_path(config, "/torrents/info");
    let url = crate::rustarr::build_url(config, &path)?;
    let raw = svc.client_ref().send_get(config, url, None).await?;
    Ok(slim(raw, TORRENT_FIELDS))
}

/// POST `/api/v2/torrents/add` (form field `urls`) → add a download by URL/magnet.
pub(super) async fn add(svc: &RustarrService, config: &ServiceConfig, url: &str) -> Result<Value> {
    let path = qbit_path(config, "/torrents/add");
    let request = crate::rustarr::build_url(config, &path)?;
    svc.client_ref()
        .send_form_post(config, request, &[("urls", url)])
        .await
}

/// POST `/api/v2/torrents/stop` (v5 name — was `pause` in v4) with
/// `hashes=<hash>` or `hashes=all`.
pub(super) async fn pause(
    svc: &RustarrService,
    config: &ServiceConfig,
    id: Option<&str>,
) -> Result<Value> {
    let path = qbit_path(config, "/torrents/stop");
    let url = crate::rustarr::build_url(config, &path)?;
    let hashes = id.unwrap_or("all");
    svc.client_ref()
        .send_form_post(config, url, &[("hashes", hashes)])
        .await
}

/// POST `/api/v2/torrents/start` (v5 name — was `resume` in v4) with
/// `hashes=<hash>` or `hashes=all`.
pub(super) async fn resume(
    svc: &RustarrService,
    config: &ServiceConfig,
    id: Option<&str>,
) -> Result<Value> {
    let path = qbit_path(config, "/torrents/start");
    let url = crate::rustarr::build_url(config, &path)?;
    let hashes = id.unwrap_or("all");
    svc.client_ref()
        .send_form_post(config, url, &[("hashes", hashes)])
        .await
}

/// POST `/api/v2/torrents/delete` with `hashes=<hash>` and
/// `deleteFiles=true|false` (default false — opt-in via `delete_files`).
pub(super) async fn remove(
    svc: &RustarrService,
    config: &ServiceConfig,
    id: &str,
    delete_files: bool,
) -> Result<Value> {
    let path = qbit_path(config, "/torrents/delete");
    let url = crate::rustarr::build_url(config, &path)?;
    let delete = if delete_files { "true" } else { "false" };
    svc.client_ref()
        .send_form_post(config, url, &[("hashes", id), ("deleteFiles", delete)])
        .await
}

#[cfg(test)]
#[path = "qbit_tests.rs"]
mod tests;
