//! Requests models — Overseerr (`/api/v1`).
//!
//! Field selection mirrors the slim keep-lists in `crate::app::requests`: the
//! paged request list and the multi-search results. Note Overseerr's request
//! `status` is an integer enum (`1`=pending, `2`=approved, `3`=declined).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `GET /api/v1/request?…` → `{ pageInfo, results: [...] }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestsPage {
    pub page_info: Option<PageInfo>,
    #[serde(default)]
    pub results: Vec<MediaRequest>,
}

/// The `pageInfo` pagination envelope on a paged Overseerr response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub pages: Option<i64>,
    pub page_size: Option<i64>,
    pub results: Option<i64>,
    pub page: Option<i64>,
}

/// A request row, slimmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaRequest {
    pub id: Option<i64>,
    /// `movie` or `tv`.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Integer enum: 1=pending, 2=approved, 3=declined.
    pub status: Option<i64>,
    pub media: Option<MediaInfo>,
    pub requested_by: Option<RequestUser>,
}

/// The `media` block on a request: the TMDB linkage and availability status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub id: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub media_type: Option<String>,
    /// Availability status integer (1=unknown … 5=available).
    pub status: Option<i64>,
}

/// The `requestedBy` user block on a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestUser {
    pub id: Option<i64>,
    pub display_name: Option<String>,
    pub email: Option<String>,
}

/// `GET /api/v1/search?query=` → `{ page, totalPages, totalResults, results }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub page: Option<i64>,
    pub total_pages: Option<i64>,
    pub total_results: Option<i64>,
    #[serde(default)]
    pub results: Vec<SearchResult>,
}

/// A multi-search hit, slimmed. The `id` is the TMDB id passed back into
/// `req_create`. `title`/`releaseDate` are populated for movies, `name`/
/// `firstAirDate` for TV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: Option<i64>,
    pub media_type: Option<String>,
    pub title: Option<String>,
    pub name: Option<String>,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
    pub overview: Option<String>,
}

#[cfg(test)]
#[path = "requests_tests.rs"]
mod tests;
