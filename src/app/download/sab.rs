//! SABnzbd DownloadClient impl (C5).
//!
//! SABnzbd is a `?mode=` query API: every action is a GET on `/api` with a
//! `mode` query param plus `output=json` and the `apikey` (both injected by
//! [`query_get`](crate::rustarr::query_get) — never `format!`'d into the path, so
//! a value like `"x&mode=delete"` cannot inject a second parameter, S6). All
//! requests therefore flow through `query_get` + `send_get`.
//!
//! Quirks (from the bead's FACT comment): `del_files=1` deletes the downloaded
//! data and defaults OFF; delete returns success even on PARTIAL failure, so the
//! removed `nzo_ids` are surfaced for the caller to verify.

use anyhow::Result;
use serde_json::{json, Value};

use crate::app::RustarrService;
use crate::config::ServiceConfig;
use crate::rustarr::slim;

/// Base path for SABnzbd's query API.
const SAB_API: &str = "/api";

/// Fields kept for a slimmed queue slot — enough to identify a job and reason
/// about progress without the full per-slot payload.
const QUEUE_FIELDS: &[&str] = &[
    "nzo_id",
    "filename",
    "status",
    "percentage",
    "mb",
    "mbleft",
    "timeleft",
    "cat",
    "priority",
];

/// GET `/api?mode=queue&output=json` → unwrap `queue.slots`, slimmed.
pub(super) async fn queue(svc: &RustarrService, config: &ServiceConfig) -> Result<Value> {
    let url = crate::rustarr::query_get(config, SAB_API, &[("mode", "queue")])?;
    let raw = svc.client_ref().send_get(config, url, None).await?;
    // SABnzbd wraps the active queue under `{ "queue": { "slots": [...] } }`.
    let slots = raw
        .get("queue")
        .and_then(|q| q.get("slots"))
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    Ok(json!({ "slots": slim(slots, QUEUE_FIELDS) }))
}

/// GET `/api?mode=addurl&name=URL` → queue a new download from a URL/magnet/NZB.
pub(super) async fn add(svc: &RustarrService, config: &ServiceConfig, url: &str) -> Result<Value> {
    let request = crate::rustarr::query_get(config, SAB_API, &[("mode", "addurl"), ("name", url)])?;
    svc.client_ref().send_get(config, request, None).await
}

/// GET `/api?mode=queue&name=pause[&value=NZO]` → pause one job, or pause-all.
pub(super) async fn pause(
    svc: &RustarrService,
    config: &ServiceConfig,
    id: Option<&str>,
) -> Result<Value> {
    let url = match id {
        Some(nzo) => crate::rustarr::query_get(
            config,
            SAB_API,
            &[("mode", "queue"), ("name", "pause"), ("value", nzo)],
        )?,
        None => crate::rustarr::query_get(config, SAB_API, &[("mode", "pause")])?,
    };
    svc.client_ref().send_get(config, url, None).await
}

/// GET `/api?mode=queue&name=resume[&value=NZO]` → resume one job, or resume-all.
pub(super) async fn resume(
    svc: &RustarrService,
    config: &ServiceConfig,
    id: Option<&str>,
) -> Result<Value> {
    let url = match id {
        Some(nzo) => crate::rustarr::query_get(
            config,
            SAB_API,
            &[("mode", "queue"), ("name", "resume"), ("value", nzo)],
        )?,
        None => crate::rustarr::query_get(config, SAB_API, &[("mode", "resume")])?,
    };
    svc.client_ref().send_get(config, url, None).await
}

/// GET `/api?mode=queue&name=delete&value=NZO[&del_files=1]` → remove a job.
///
/// `del_files=1` deletes the downloaded data and is only sent when the caller
/// opts in. The raw response (which carries the removed `nzo_ids`) is returned so
/// the caller can verify a PARTIAL-failure delete (SABnzbd reports success even
/// when some ids could not be removed).
pub(super) async fn remove(
    svc: &RustarrService,
    config: &ServiceConfig,
    id: &str,
    delete_files: bool,
) -> Result<Value> {
    let mut params: Vec<(&str, &str)> = vec![("mode", "queue"), ("name", "delete"), ("value", id)];
    if delete_files {
        params.push(("del_files", "1"));
    }
    let url = crate::rustarr::query_get(config, SAB_API, &params)?;
    svc.client_ref().send_get(config, url, None).await
}

#[cfg(test)]
#[path = "sab_tests.rs"]
mod tests;
