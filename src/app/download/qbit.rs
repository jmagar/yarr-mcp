//! qBittorrent DownloadClient impl (C5).
//!
//! qBittorrent's WebUI API is a `/api/v2` REST surface authed by a username/
//! password SID cookie (established in [`crate::yarr::auth`], a dedicated
//! cookie-store client per the F2/S1 isolation fix). Reads are GETs; mutations
//! are `application/x-www-form-urlencoded` POSTs through
//! [`send_form_post`](crate::yarr::YarrClient::send_form_post), which
//! percent-encodes every field — callers never `format!` values into the body.
//!
//! API-VERSION FACT (bead, HIGH): qBittorrent **v5 renamed** the pause/resume
//! endpoints — `pause` → `POST /api/v2/torrents/stop`, `resume` →
//! `POST /api/v2/torrents/start`. The v4 `pause`/`resume` paths are GONE, so this
//! module targets the v5 `stop`/`start` names.

use anyhow::Result;
use serde_json::{Value, json};

use crate::app::YarrService;
use crate::config::ServiceConfig;
use crate::yarr::slim;

/// Envelope for a qBittorrent bulk mutation (`stop`/`start`/`delete`).
///
/// qBittorrent returns HTTP 200 with an EMPTY body even when the supplied hashes
/// match NO torrent, and the transport coerces an empty body to `{ok:true,...}`.
/// Returning that bare value would falsely imply the target existed. Instead we
/// return a `submitted` envelope that makes clear the action was accepted but NOT
/// confirmed against a real torrent, and points the caller at `queue` to verify.
fn qbit_submitted(status: u16) -> Value {
    json!({
        "submitted": true,
        "status": status,
        "note": "qBittorrent returns no confirmation body; verify with `queue`",
    })
}

/// Read the HTTP status the transport stamped onto a coerced empty-body response
/// (`{ok:true,status:200}`), defaulting to 200 when absent.
fn response_status(response: &Value) -> u16 {
    response
        .get("status")
        .and_then(Value::as_u64)
        .map(|s| s as u16)
        .unwrap_or(200)
}

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
pub(super) async fn queue(svc: &YarrService, config: &ServiceConfig) -> Result<Value> {
    let path = qbit_path(config, "/torrents/info");
    let url = crate::yarr::build_url(config, &path)?;
    let raw = svc.client_ref().send_get(config, url, None).await?;
    Ok(slim(raw, TORRENT_FIELDS))
}

/// POST `/api/v2/torrents/add` (form field `urls`) → add a download by URL/magnet.
pub(super) async fn add(svc: &YarrService, config: &ServiceConfig, url: &str) -> Result<Value> {
    let path = qbit_path(config, "/torrents/add");
    let request = crate::yarr::build_url(config, &path)?;
    svc.client_ref()
        .send_form_post(config, request, &[("urls", url)])
        .await
}

/// POST `/api/v2/torrents/stop` (v5 name — was `pause` in v4) with
/// `hashes=<hash>` or `hashes=all`.
pub(super) async fn pause(
    svc: &YarrService,
    config: &ServiceConfig,
    id: Option<&str>,
) -> Result<Value> {
    let path = qbit_path(config, "/torrents/stop");
    let url = crate::yarr::build_url(config, &path)?;
    let hashes = id.unwrap_or("all");
    let response = svc
        .client_ref()
        .send_form_post(config, url, &[("hashes", hashes)])
        .await?;
    Ok(qbit_submitted(response_status(&response)))
}

/// POST `/api/v2/torrents/start` (v5 name — was `resume` in v4) with
/// `hashes=<hash>` or `hashes=all`.
pub(super) async fn resume(
    svc: &YarrService,
    config: &ServiceConfig,
    id: Option<&str>,
) -> Result<Value> {
    let path = qbit_path(config, "/torrents/start");
    let url = crate::yarr::build_url(config, &path)?;
    let hashes = id.unwrap_or("all");
    let response = svc
        .client_ref()
        .send_form_post(config, url, &[("hashes", hashes)])
        .await?;
    Ok(qbit_submitted(response_status(&response)))
}

/// POST `/api/v2/torrents/delete` with `hashes=<hash>` and
/// `deleteFiles=true|false` (default false — opt-in via `delete_files`).
pub(super) async fn remove(
    svc: &YarrService,
    config: &ServiceConfig,
    id: &str,
    delete_files: bool,
) -> Result<Value> {
    let path = qbit_path(config, "/torrents/delete");
    let url = crate::yarr::build_url(config, &path)?;
    let delete = if delete_files { "true" } else { "false" };
    let response = svc
        .client_ref()
        .send_form_post(config, url, &[("hashes", id), ("deleteFiles", delete)])
        .await?;
    Ok(qbit_submitted(response_status(&response)))
}

#[cfg(test)]
#[path = "qbit_tests.rs"]
mod tests;
