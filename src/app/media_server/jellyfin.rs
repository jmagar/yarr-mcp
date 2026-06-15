//! Jellyfin MediaServer impl (C6).
//!
//! Jellyfin is a JSON REST API authed by the `Authorization: MediaBrowser
//! Token="…"` header (with `X-Emby-Token` fallback) applied in
//! [`auth::apply_auth`](crate::rustarr::auth) for the `JellyfinToken` auth style
//! (F2) — no token handling lives here.
//!
//! Item-query FACT (bead, HIGH): in Jellyfin EVERYTHING is a `BaseItemDto`, so an
//! item query without `includeItemTypes` returns folders, collections, and noise.
//! Search therefore ALWAYS sends `includeItemTypes` (and `recursive=true`). Ids
//! are UUID strings.
//!
//! User-supplied search text reaches Jellyfin through
//! [`query_get`](crate::rustarr::query_get) (percent-encoded), never `format!`'d
//! into the path, so a value like `"x&IsFavorite=true"` cannot inject a second
//! query parameter (S6).

use anyhow::Result;
use serde_json::{json, Value};

use crate::app::RustarrService;
use crate::config::ServiceConfig;
use crate::rustarr::slim;

/// Item types search is restricted to — ALWAYS sent (bead FACT, HIGH): without it
/// Jellyfin returns every `BaseItemDto` (folders, collections, …), not media.
const SEARCH_ITEM_TYPES: &str = "Movie,Series,Episode";

/// Fields kept for a slimmed Jellyfin session (`/Sessions` entry).
const SESSION_FIELDS: &[&str] = &[
    "UserName",
    "NowPlayingItem",
    "DeviceName",
    "Client",
    "PlayState",
];

/// Fields kept for a slimmed Jellyfin library (`/Library/VirtualFolders` entry).
const LIBRARY_FIELDS: &[&str] = &["ItemId", "Name", "CollectionType"];

/// Fields kept for a slimmed Jellyfin search hit (`/Items.Items` entry).
const SEARCH_FIELDS: &[&str] = &["Id", "Name", "Type", "ProductionYear", "SeriesName"];

/// GET `/Sessions` → active streams, slimmed.
pub(super) async fn sessions(svc: &RustarrService, config: &ServiceConfig) -> Result<Value> {
    let url = crate::rustarr::build_url(config, "/Sessions")?;
    let raw = svc.client_ref().send_get(config, url, None).await?;
    Ok(json!({ "sessions": slim(raw, SESSION_FIELDS) }))
}

/// GET `/Library/VirtualFolders` → libraries, slimmed.
pub(super) async fn libraries(svc: &RustarrService, config: &ServiceConfig) -> Result<Value> {
    let url = crate::rustarr::build_url(config, "/Library/VirtualFolders")?;
    let raw = svc.client_ref().send_get(config, url, None).await?;
    Ok(json!({ "libraries": slim(raw, LIBRARY_FIELDS) }))
}

/// GET `/Items?searchTerm=&includeItemTypes=…&recursive=true` → hits, slimmed.
///
/// `includeItemTypes` is ALWAYS sent (bead FACT) so the result is media, not the
/// full `BaseItemDto` tree. `query` is percent-encoded by `query_get`.
pub(super) async fn search(
    svc: &RustarrService,
    config: &ServiceConfig,
    query: &str,
) -> Result<Value> {
    let url = crate::rustarr::query_get(
        config,
        "/Items",
        &[
            ("searchTerm", query),
            ("includeItemTypes", SEARCH_ITEM_TYPES),
            ("recursive", "true"),
        ],
    )?;
    let raw = svc.client_ref().send_get(config, url, None).await?;
    // Jellyfin wraps item queries under `{ "Items": [...] }`.
    let items = raw
        .get("Items")
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    Ok(json!({ "results": slim(items, SEARCH_FIELDS) }))
}

/// POST `/Library/Refresh` → trigger a server-wide library scan.
///
/// Jellyfin's refresh is server-wide, so no library id is sent; the empty JSON
/// body satisfies the endpoint.
pub(super) async fn scan(svc: &RustarrService, config: &ServiceConfig) -> Result<Value> {
    svc.client_ref()
        .post_json(config, "/Library/Refresh", json!({}))
        .await
}

#[cfg(test)]
#[path = "jellyfin_tests.rs"]
mod tests;
