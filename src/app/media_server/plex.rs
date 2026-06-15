//! Plex MediaServer impl (C6).
//!
//! Plex is an HTTP API that returns **XML by default**; per the bead's FACT
//! comment it requires `Accept: application/json` on EVERY call to negotiate a
//! JSON body. That negotiation is a TRANSPORT concern, so each request here just
//! passes [`ACCEPT_JSON`] as the `accept_mime` argument to
//! [`send_get`](crate::rustarr::RustarrClient::send_get) — no XML parsing lives in
//! this module (architecture decision C6-a).
//!
//! Auth is the `X-Plex-Token` query parameter, injected by
//! [`build_url`](crate::rustarr::build_url) /
//! [`query_get`](crate::rustarr::query_get) for the `PlexToken` auth style — never
//! `format!`'d into the path. User-supplied search text therefore reaches Plex
//! through `query_get` (percent-encoded), so a value like `"x&type=movie"` cannot
//! inject a second query parameter (S6).
//!
//! Plex wraps every response under a `MediaContainer` envelope; the slim helpers
//! unwrap the relevant child array (`Metadata` for sessions/search, `Directory`
//! for library sections) before field selection.

use anyhow::Result;
use serde_json::{Value, json};

use crate::app::RustarrService;
use crate::config::ServiceConfig;
use crate::rustarr::slim;

/// Plex returns XML unless this is sent on EVERY request (bead FACT, HIGH).
const ACCEPT_JSON: &str = "application/json";

/// Fields kept for a slimmed Plex session (`MediaContainer.Metadata` entry).
const SESSION_FIELDS: &[&str] = &["title", "type", "User", "Player", "Session", "viewOffset"];

/// Fields kept for a slimmed Plex library section (`MediaContainer.Directory`).
const LIBRARY_FIELDS: &[&str] = &["key", "title", "type"];

/// Fields kept for a slimmed Plex search hit (`MediaContainer.Metadata` entry).
const SEARCH_FIELDS: &[&str] = &["ratingKey", "title", "type", "year", "librarySectionTitle"];

/// Unwrap the `MediaContainer.<child>` array Plex wraps every payload in.
fn unwrap_container(raw: &Value, child: &str) -> Value {
    raw.get("MediaContainer")
        .and_then(|c| c.get(child))
        .cloned()
        .unwrap_or(Value::Array(Vec::new()))
}

/// GET `/status/sessions` (JSON-negotiated) → active streams, slimmed.
pub(super) async fn sessions(svc: &RustarrService, config: &ServiceConfig) -> Result<Value> {
    let url = crate::rustarr::build_url(config, "/status/sessions")?;
    let raw = svc
        .client_ref()
        .send_get(config, url, Some(ACCEPT_JSON))
        .await?;
    Ok(json!({ "sessions": slim(unwrap_container(&raw, "Metadata"), SESSION_FIELDS) }))
}

/// GET `/library/sections` (JSON-negotiated) → libraries, slimmed.
pub(super) async fn libraries(svc: &RustarrService, config: &ServiceConfig) -> Result<Value> {
    let url = crate::rustarr::build_url(config, "/library/sections")?;
    let raw = svc
        .client_ref()
        .send_get(config, url, Some(ACCEPT_JSON))
        .await?;
    Ok(json!({ "libraries": slim(unwrap_container(&raw, "Directory"), LIBRARY_FIELDS) }))
}

/// GET `/library/search?query=…` (JSON-negotiated) → hits, slimmed.
///
/// `query` is percent-encoded by `query_get`, never `format!`'d into the path.
pub(super) async fn search(
    svc: &RustarrService,
    config: &ServiceConfig,
    query: &str,
) -> Result<Value> {
    let url = crate::rustarr::query_get(config, "/library/search", &[("query", query)])?;
    let raw = svc
        .client_ref()
        .send_get(config, url, Some(ACCEPT_JSON))
        .await?;
    Ok(json!({ "results": slim(unwrap_container(&raw, "Metadata"), SEARCH_FIELDS) }))
}

/// GET `/library/sections/{library}/refresh` (JSON-negotiated) → trigger a scan.
///
/// `library` is a numeric Plex section id (parsed in `media_scan`), so the only
/// characters that can reach the path are digits — no traversal / injection value
/// is representable (S6).
pub(super) async fn scan(
    svc: &RustarrService,
    config: &ServiceConfig,
    library: u64,
) -> Result<Value> {
    let path = format!("/library/sections/{library}/refresh");
    let url = crate::rustarr::build_url(config, &path)?;
    svc.client_ref()
        .send_get(config, url, Some(ACCEPT_JSON))
        .await
}

#[cfg(test)]
#[path = "plex_tests.rs"]
mod tests;
