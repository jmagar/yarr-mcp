//! MediaServer models — Plex and Jellyfin.
//!
//! The two diverge completely. Plex wraps everything in a `MediaContainer`
//! envelope and mixes PascalCase child arrays (`Metadata`, `Directory`,
//! `SearchResult`, `User`, `Player`) with camelCase/lowercase scalar attributes
//! (`title`, `type`, `ratingKey`, `viewOffset`), so its structs use per-field
//! renames. Jellyfin is uniform PascalCase (`rename_all = "PascalCase"`) where
//! everything is a `BaseItemDto`. Field selection mirrors the slim keep-lists in
//! `crate::app::media_server`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Plex ─────────────────────────────────────────────────────────────────────

/// The `MediaContainer` envelope Plex wraps every response in. `/identity`
/// populates `machine_identifier`/`version`; `/status/sessions` populates
/// `metadata`; `/library/sections` populates `directory`; `/library/search`
/// populates `search_result`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlexResponse {
    #[serde(rename = "MediaContainer")]
    pub media_container: Option<MediaContainer>,
}

/// The inner container holding whichever child array the endpoint returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MediaContainer {
    #[serde(rename = "machineIdentifier")]
    pub machine_identifier: Option<String>,
    pub version: Option<String>,
    /// Active streams (`/status/sessions`).
    #[serde(rename = "Metadata", default)]
    pub metadata: Vec<PlexMetadata>,
    /// Library sections (`/library/sections`).
    #[serde(rename = "Directory", default)]
    pub directory: Vec<PlexDirectory>,
    /// Search hits (`/library/search`), each wrapping a `Metadata` entry.
    #[serde(rename = "SearchResult", default)]
    pub search_result: Vec<PlexSearchResult>,
}

/// A Plex media item — covers both session entries and search results (slimmed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlexMetadata {
    #[serde(rename = "ratingKey")]
    pub rating_key: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub year: Option<i64>,
    #[serde(rename = "librarySectionTitle")]
    pub library_section_title: Option<String>,
    /// Playback position in ms (session entries only).
    #[serde(rename = "viewOffset")]
    pub view_offset: Option<i64>,
    #[serde(rename = "User")]
    pub user: Option<PlexUser>,
    #[serde(rename = "Player")]
    pub player: Option<PlexPlayer>,
}

/// The `User` object on a Plex session entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlexUser {
    pub id: Option<String>,
    pub title: Option<String>,
}

/// The `Player` object on a Plex session entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlexPlayer {
    pub title: Option<String>,
    pub state: Option<String>,
    pub product: Option<String>,
}

/// A library section row (`MediaContainer.Directory`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlexDirectory {
    pub key: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

/// A Plex search result wrapping a `Metadata` entry
/// (`MediaContainer.SearchResult[].Metadata`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlexSearchResult {
    #[serde(rename = "Metadata")]
    pub metadata: Option<PlexMetadata>,
}

// ── Jellyfin ─────────────────────────────────────────────────────────────────

/// A `GET /Sessions` entry, slimmed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct JellyfinSession {
    pub user_name: Option<String>,
    pub now_playing_item: Option<JellyfinItem>,
    pub device_name: Option<String>,
    pub client: Option<String>,
    pub play_state: Option<JellyfinPlayState>,
}

/// The `PlayState` object on a Jellyfin session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct JellyfinPlayState {
    pub position_ticks: Option<i64>,
    pub is_paused: Option<bool>,
    pub play_method: Option<String>,
}

/// A `GET /Library/VirtualFolders` library row, slimmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct VirtualFolder {
    pub item_id: Option<String>,
    pub name: Option<String>,
    pub collection_type: Option<String>,
}

/// `GET /Items?…` → `{ Items: [...] }`. In Jellyfin everything is a
/// `BaseItemDto`, so item queries always wrap rows under `Items`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct JellyfinItemsResponse {
    #[serde(default)]
    pub items: Vec<JellyfinItem>,
}

/// A Jellyfin `BaseItemDto`, slimmed — covers both `NowPlayingItem` and search
/// hits. Ids are UUID strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct JellyfinItem {
    pub id: Option<String>,
    pub name: Option<String>,
    /// `Movie` / `Series` / `Episode` …
    #[serde(rename = "Type")]
    pub kind: Option<String>,
    pub production_year: Option<i64>,
    pub series_name: Option<String>,
}

#[cfg(test)]
#[path = "media_server_tests.rs"]
mod tests;
