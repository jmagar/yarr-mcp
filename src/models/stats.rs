//! Stats models — Tautulli (`/api/v2?cmd=…`).
//!
//! Tautulli wraps every command in a `{ response: { result, message, data } }`
//! envelope ([`TautulliEnvelope`]) and reports `data` shapes per command. The
//! envelope is generic over the command's payload so a caller can decode any
//! `cmd` into a typed `data`. Field selection mirrors the slim keep-lists in
//! `crate::app::stats`.
//!
//! Tautulli quirk: `get_activity` serialises `stream_count` as a *string*
//! (`"2"`), and history `date` is a Unix epoch integer — modelled accordingly.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The outer Tautulli envelope: `{ response: { … } }`. Generic over the `data`
/// payload `T` so any `cmd` can be decoded into a typed body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TautulliEnvelope<T> {
    pub response: Option<TautulliResponse<T>>,
}

/// The inner response: `result` is `"success"` on success, otherwise `message`
/// carries the human reason and `data` is absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TautulliResponse<T> {
    pub result: Option<String>,
    pub message: Option<String>,
    pub data: Option<T>,
}

/// `cmd=get_activity` data: current streams plus a per-stream session list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Activity {
    /// Serialised as a string by Tautulli (e.g. `"2"`).
    pub stream_count: Option<String>,
    #[serde(default)]
    pub sessions: Vec<StreamSession>,
}

/// A per-stream session (`get_activity.sessions[]`), slimmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StreamSession {
    pub user: Option<String>,
    pub full_title: Option<String>,
    pub title: Option<String>,
    pub state: Option<String>,
    pub progress_percent: Option<String>,
    pub media_type: Option<String>,
}

/// `cmd=get_history` data: a paged datatable wrapping history rows under `data`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HistoryPage {
    pub records_total: Option<i64>,
    pub records_filtered: Option<i64>,
    #[serde(default)]
    pub data: Vec<HistoryRow>,
}

/// A watch-history row (`get_history.data[]`), slimmed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HistoryRow {
    /// Unix epoch seconds.
    pub date: Option<i64>,
    pub user: Option<String>,
    pub full_title: Option<String>,
    pub title: Option<String>,
    pub media_type: Option<String>,
    /// 1 = watched, 0.5 = partial, 0 = unwatched.
    pub watched_status: Option<f64>,
    pub percent_complete: Option<i64>,
}

/// A `cmd=get_users` row, slimmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TautulliUser {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub plays: Option<i64>,
}

/// A `cmd=get_library_names` row, slimmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LibraryName {
    pub section_id: Option<i64>,
    pub section_name: Option<String>,
    pub section_type: Option<String>,
    pub agent: Option<String>,
    pub count: Option<i64>,
    pub parent_count: Option<i64>,
    pub child_count: Option<i64>,
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
