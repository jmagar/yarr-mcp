//! ArrManager models — Sonarr (`series`) and Radarr (`movie`).
//!
//! Both share the `/api/v3` schema and diverge only in the resource noun, so a
//! single [`ArrResource`] covers a slimmed `series`/`movie` library row. Fields
//! mirror the proven `slim()` keep-lists in `crate::app::arr::read` plus the
//! `statistics` block the list summary reads.
//!
//! Note the queue/history records use the *arr API's lowercase `sizeleft` /
//! `timeleft` keys (not `sizeLeft`); the field names are single words so
//! `rename_all = "camelCase"` leaves them untouched.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A slimmed library row from the primary resource collection
/// (`GET /api/v3/series` for Sonarr, `/api/v3/movie` for Radarr).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArrResource {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub quality_profile_id: Option<i64>,
    pub monitored: Option<bool>,
    /// Bytes on disk. Radarr reports `0` for an unacquired movie.
    pub size_on_disk: Option<u64>,
    pub status: Option<String>,
    /// ISO-8601 timestamp the item was added.
    pub added: Option<String>,
    /// Present on Sonarr series rows; drives the missing-episode summary.
    pub statistics: Option<ArrStatistics>,
}

/// The `statistics` block on a Sonarr series row (used to compute missing
/// episodes in the library summary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArrStatistics {
    pub episode_count: Option<u64>,
    pub episode_file_count: Option<u64>,
    pub size_on_disk: Option<u64>,
}

/// `GET /api/v3/qualityprofile` row, slimmed to the stable identity + cutoff /
/// upgrade settings (the live row also carries the full nested quality tree).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityProfile {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub cutoff: Option<i64>,
    pub cutoff_format_score: Option<i64>,
    pub min_format_score: Option<i64>,
    pub min_upgrade_format_score: Option<i64>,
    pub upgrade_allowed: Option<bool>,
}

/// The paged envelope the *arr `queue` / `history` / `wanted/missing` endpoints
/// return: `{ page, pageSize, totalRecords, records: [...] }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedRecords {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub total_records: Option<i64>,
    #[serde(default)]
    pub records: Vec<QueueRecord>,
}

/// A queue / history / wanted record. The same shape covers all three paged
/// endpoints; not every field is populated on every endpoint (e.g. `eventType`
/// is history-only, `timeleft` is queue-only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueRecord {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub monitored: Option<bool>,
    pub size: Option<u64>,
    /// Lowercase on the wire (`sizeleft`), not `sizeLeft`.
    pub sizeleft: Option<u64>,
    /// Lowercase on the wire (`timeleft`), not `timeLeft`.
    pub timeleft: Option<String>,
    pub tracked_download_status: Option<String>,
    pub tracked_download_state: Option<String>,
    pub error_message: Option<String>,
    pub download_client: Option<String>,
    pub indexer: Option<String>,
    /// History-only: `grabbed` / `downloadFolderImported` / `episodeFileDeleted` …
    pub event_type: Option<String>,
    pub date: Option<String>,
    pub quality: Option<QualityRevision>,
    pub series: Option<RelatedTitle>,
    pub movie: Option<RelatedTitle>,
    pub episode: Option<EpisodeRef>,
    #[serde(default)]
    pub status_messages: Vec<StatusMessage>,
}

/// The `quality` wrapper on a record: `{ quality: { id, name }, revision: … }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct QualityRevision {
    pub quality: Option<QualityItem>,
}

/// The inner `quality` object (`{ id, name, source, resolution }`), slimmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct QualityItem {
    pub id: Option<i64>,
    pub name: Option<String>,
}

/// A minimal nested `series` / `movie` reference on a queue record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RelatedTitle {
    pub id: Option<i64>,
    pub title: Option<String>,
}

/// A nested `episode` reference on a Sonarr queue/history record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeRef {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub season_number: Option<i64>,
    pub episode_number: Option<i64>,
}

/// A per-record import/health note (`{ title, messages: [...] }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StatusMessage {
    pub title: Option<String>,
    #[serde(default)]
    pub messages: Vec<String>,
}

/// `GET /api/v3/rootfolder` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RootFolder {
    pub id: Option<i64>,
    pub path: Option<String>,
    pub accessible: Option<bool>,
    pub free_space: Option<u64>,
    pub total_space: Option<u64>,
}

/// `GET /api/v3/health` message. An empty array means healthy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthMessage {
    pub source: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub message: Option<String>,
    pub wiki_url: Option<String>,
}

#[cfg(test)]
#[path = "arr_tests.rs"]
mod tests;
