//! Overseerr models — request management (`/api/v1`).
//!
//! Sourced from the Overseerr OpenAPI spec (`overseerr-api.yml`, `develop`
//! branch: `MediaRequest`, `MediaInfo`, `User`, `PageInfo`, `MovieResult`,
//! `TvResult`, `Season`, `Episode`; `/status` and the `GET /request` list
//! envelope are inline schemas, not named components). Field selection mirrors
//! the keep-lists in `crate::app::requests` but carries the fuller spec shape so
//! a machine-readable schema can be emitted for the whole request surface.
//!
//! Overseerr quirks worth flagging:
//!   * **Casing is `camelCase`** across every schema here. (The only snake_case
//!     keys in the spec — `iso_3166_1` on a nested `ProductionCountry` — are out
//!     of scope.)
//!   * **Timestamps are ISO-8601 strings** (e.g. `"2020-09-12T10:00:27.000Z"`),
//!     *not* epoch ints, so `createdAt` / `updatedAt` are modelled as `String`.
//!   * **Several state fields are integer enums.** `MediaRequest.status` is
//!     `1`=pending / `2`=approved / `3`=declined (the spec example shows `0`, but
//!     valid values start at `1`). `MediaInfo.status` / `status4k` are
//!     `1`=unknown … `6`=deleted. `User.userType` is `1`=Plex / `2`=local, and
//!     `User.permissions` is a numeric bitfield.
//!   * **`MediaRequest.modifiedBy`** is `anyOf [User, nullable string]` in the
//!     spec; the string branch is an artifact, so it is modelled as
//!     [`Option<User>`] (`null` when never modified).
//!   * **`type` / `seasons` on [`MediaRequest`]** and `pageSize` on [`PageInfo`]
//!     are *not* in the OpenAPI schemas but are present on the runtime TypeORM
//!     entities — included defensively (and `type` renamed to `kind`). Likewise
//!     `User.displayName` is a runtime-derived field absent from the spec User.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `GET /request` `200` — the paged request-list envelope. This is an inline
/// schema (`{ pageInfo, results }`), not a named component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaRequestPage {
    /// Pagination metadata (`$ref` PageInfo).
    pub page_info: Option<PageInfo>,
    /// The `MediaRequest` items for this page.
    #[serde(default)]
    pub results: Vec<MediaRequest>,
}

/// Pagination metadata for paged list envelopes. The spec documents only
/// `page` / `pages` / `results`; `pageSize` is on the runtime entity and is
/// included defensively.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    /// Current page number (example `1`).
    pub page: Option<i64>,
    /// Total number of pages (example `10`).
    pub pages: Option<i64>,
    /// Total result count across all pages (example `100`).
    pub results: Option<i64>,
    /// Page size. NOT in the OpenAPI PageInfo schema (only `page`/`pages`/
    /// `results` are documented) but present on the runtime PageInfo entity.
    pub page_size: Option<i64>,
}

/// A media request submitted by a user (movie or TV). `GET /request`.
///
/// Spec `required` is `[id, status]`; everything else is optional. `readOnly`
/// fields are still present in responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaRequest {
    /// Request ID. `readOnly`.
    pub id: Option<i64>,
    /// Request status integer: `1`=pending approval, `2`=approved,
    /// `3`=declined. `readOnly`. (The spec example shows `0`, but valid values
    /// are `1`–`3`.)
    pub status: Option<i64>,
    /// The associated media-info object (renamed from the `media` JSON key).
    pub media: Option<MediaInfo>,
    /// ISO-8601 timestamp, e.g. `"2020-09-12T10:00:27.000Z"`. `readOnly`.
    /// String, not epoch.
    pub created_at: Option<String>,
    /// ISO-8601 timestamp. `readOnly`.
    pub updated_at: Option<String>,
    /// User who submitted the request.
    pub requested_by: Option<User>,
    /// User who last modified the request. The spec types this as
    /// `anyOf [User, nullable string]`; modelled as `Option<User>` (`null` when
    /// never modified).
    pub modified_by: Option<User>,
    /// Whether this is a 4K request (example `false`).
    pub is_4k: Option<bool>,
    /// Radarr/Sonarr server ID.
    pub server_id: Option<i64>,
    /// Quality profile ID.
    pub profile_id: Option<i64>,
    /// Root folder path.
    pub root_folder: Option<String>,
    /// Media type (`movie` | `tv`). NOT in the OpenAPI `MediaRequest` schema but
    /// present on the runtime entity; renamed from the reserved `type` key.
    /// Included defensively.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Requested seasons. NOT in the OpenAPI `MediaRequest` schema but present on
    /// the runtime entity (`SeasonRequest[]`). Included defensively.
    #[serde(default)]
    pub seasons: Vec<Season>,
}

/// Media availability/identity record linking TMDB/TVDB IDs to Overseerr
/// availability state. Nested on `MediaRequest.media` and returned by
/// `GET /media`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    /// Internal media ID. `readOnly`.
    pub id: Option<i64>,
    /// TMDB ID. `readOnly`.
    pub tmdb_id: Option<i64>,
    /// TVDB ID. `readOnly`, nullable.
    pub tvdb_id: Option<i64>,
    /// Availability integer: `1`=unknown, `2`=pending, `3`=processing,
    /// `4`=partially available, `5`=available, `6`=deleted.
    pub status: Option<i64>,
    /// 4K availability integer, same enum as `status`.
    pub status_4k: Option<i64>,
    /// Media type (`movie` | `tv`).
    pub media_type: Option<String>,
    /// Associated requests. `readOnly`.
    #[serde(default)]
    pub requests: Vec<MediaRequest>,
    /// ISO-8601 timestamp. `readOnly`. String, not epoch.
    pub created_at: Option<String>,
    /// ISO-8601 timestamp. `readOnly`.
    pub updated_at: Option<String>,
}

/// An Overseerr user (Plex or local). `GET /user`, `GET /auth/me`.
///
/// Spec `required` is `[id, email, createdAt, updatedAt]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// User ID (integer). `readOnly`.
    pub id: Option<i64>,
    /// Email address, e.g. `"hey@itsme.com"`. `readOnly`.
    pub email: Option<String>,
    /// Display username (writable). The spec field is `username`; the runtime
    /// `displayName` is a separate derived field (see [`User::display_name`]).
    pub username: Option<String>,
    /// Plex auth token. `readOnly`.
    pub plex_token: Option<String>,
    /// Plex username. `readOnly`.
    pub plex_username: Option<String>,
    /// User type integer (`1`=Plex, `2`=local). `readOnly` (example `1`).
    pub user_type: Option<i64>,
    /// Permissions bitfield (numeric; example `0`).
    pub permissions: Option<i64>,
    /// Avatar URL. `readOnly`.
    pub avatar: Option<String>,
    /// ISO-8601 timestamp. `readOnly`. String, not epoch.
    pub created_at: Option<String>,
    /// ISO-8601 timestamp. `readOnly`.
    pub updated_at: Option<String>,
    /// Number of requests by this user. `readOnly` (example `5`).
    pub request_count: Option<i64>,
    /// Friendly display name. NOT in the OpenAPI User schema but commonly
    /// returned by the runtime API (falls back to `username`/`plexUsername`).
    /// Included defensively.
    pub display_name: Option<String>,
}

/// A movie search/discover result. `GET /search`, `GET /discover/movies`.
///
/// Spec `required` is `[id, mediaType, title]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieResult {
    /// TMDB movie ID (example `1234`).
    pub id: Option<i64>,
    /// Result-type discriminator (`movie`).
    pub media_type: Option<String>,
    /// TMDB popularity score (example `10`).
    pub popularity: Option<f64>,
    /// Poster image path.
    pub poster_path: Option<String>,
    /// Backdrop image path.
    pub backdrop_path: Option<String>,
    /// TMDB vote count.
    pub vote_count: Option<i64>,
    /// TMDB average vote (float).
    pub vote_average: Option<f64>,
    /// TMDB genre IDs.
    #[serde(default)]
    pub genre_ids: Vec<i64>,
    /// Plot overview text.
    pub overview: Option<String>,
    /// ISO language code, e.g. `"en"`.
    pub original_language: Option<String>,
    /// Movie title.
    pub title: Option<String>,
    /// Original-language title.
    pub original_title: Option<String>,
    /// Release date string (`YYYY-MM-DD`).
    pub release_date: Option<String>,
    /// Adult content flag (example `false`).
    pub adult: Option<bool>,
    /// Whether the entry is a video (example `false`).
    pub video: Option<bool>,
    /// Overseerr media availability info, if tracked.
    pub media_info: Option<MediaInfo>,
}

/// A TV series search/discover result. `GET /search`, `GET /discover/tv`.
///
/// The spec gives this type no `required` list — every field is optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TvResult {
    /// TMDB series ID (example `1234`).
    pub id: Option<i64>,
    /// Result-type discriminator (`tv`).
    pub media_type: Option<String>,
    /// TMDB popularity score (example `10`).
    pub popularity: Option<f64>,
    /// Poster image path.
    pub poster_path: Option<String>,
    /// Backdrop image path.
    pub backdrop_path: Option<String>,
    /// TMDB vote count.
    pub vote_count: Option<i64>,
    /// TMDB average vote (float).
    pub vote_average: Option<f64>,
    /// TMDB genre IDs.
    #[serde(default)]
    pub genre_ids: Vec<i64>,
    /// Plot overview text.
    pub overview: Option<String>,
    /// ISO language code, e.g. `"en"`.
    pub original_language: Option<String>,
    /// Series name (TV uses `name`, not `title`).
    pub name: Option<String>,
    /// Original-language series name.
    pub original_name: Option<String>,
    /// ISO country codes of origin.
    #[serde(default)]
    pub origin_country: Vec<String>,
    /// First air date string (`YYYY-MM-DD`).
    pub first_air_date: Option<String>,
    /// Overseerr media availability info, if tracked.
    pub media_info: Option<MediaInfo>,
}

/// Overseerr server status/version info. `GET /status` — an inline response
/// schema (not a named component) and a public, no-auth endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Running version string, e.g. `"1.0.0"`.
    pub version: Option<String>,
    /// Git commit tag of the build.
    pub commit_tag: Option<String>,
    /// Whether a newer version is available.
    pub update_available: Option<bool>,
    /// Number of commits behind the latest release.
    pub commits_behind: Option<i64>,
    /// Whether a restart is required to apply changes.
    pub restart_required: Option<bool>,
}

/// A TV season summary. Nested in `TvDetails.seasons` (`GET /tv/{tvId}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    /// TMDB season ID.
    pub id: Option<i64>,
    /// Air date string (`YYYY-MM-DD`), nullable.
    pub air_date: Option<String>,
    /// Number of episodes in the season.
    pub episode_count: Option<i64>,
    /// Season name.
    pub name: Option<String>,
    /// Season overview text.
    pub overview: Option<String>,
    /// Season poster image path.
    pub poster_path: Option<String>,
    /// Season number (`0` = specials).
    pub season_number: Option<i64>,
    /// Episode list (only populated on season-detail responses).
    #[serde(default)]
    pub episodes: Vec<Episode>,
}

/// A single TV episode. Nested in `Season.episodes`
/// (`GET /tv/{tvId}/season/{seasonId}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    /// TMDB episode ID.
    pub id: Option<i64>,
    /// Episode title.
    pub name: Option<String>,
    /// Air date string, nullable.
    pub air_date: Option<String>,
    /// Episode number within the season.
    pub episode_number: Option<i64>,
    /// Episode overview text.
    pub overview: Option<String>,
    /// Production code.
    pub production_code: Option<String>,
    /// Season number this episode belongs to.
    pub season_number: Option<i64>,
    /// Parent show TMDB ID.
    pub show_id: Option<i64>,
    /// Still-frame image path, nullable.
    pub still_path: Option<String>,
    /// TMDB average vote.
    pub vote_average: Option<f64>,
    /// TMDB vote count.
    pub vote_count: Option<i64>,
}

#[cfg(test)]
#[path = "overseerr_tests.rs"]
mod tests;
