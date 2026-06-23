//! Bazarr models — subtitle automation (`/api/...`, flask_restx).
//!
//! Bazarr has no clean OpenAPI spec; these shapes are read from the authoritative
//! flask_restx `fields.*` response models plus the `postprocess()` transformer in
//! `api/utils.py`. Two envelope conventions coexist:
//!
//! - [`SystemStatus`] and the providers list wrap the payload under `{ "data": … }`
//!   with no `total`; the episodes endpoint likewise returns `{ "data": [...] }`
//!   (a list, no `total`).
//! - [`MoviesGetResponse`], [`WantedEpisodesResponse`] and [`WantedMoviesResponse`]
//!   use the DataTables-style `{ "data": [...], "total": <int> }` paged envelope.
//! - `GET /api/system/languages` returns a *bare* JSON array of [`Language`] with
//!   no envelope.
//!
//! Casing is mixed: camelCase identifiers (`sonarrSeriesId`, `radarrId`) coexist
//! with snake_case (`missing_subtitles`, `audio_language`), so no single
//! `rename_all` fits — each non-snake field carries an explicit `#[serde(rename)]`.
//!
//! Postprocess quirks worth noting: `audio_language` is declared as a single
//! Nested in flask_restx but the transformer emits a *list*, so the wire shape is
//! `Vec<AudioLanguage>`; `monitored` is coerced *server-side* by `postprocess`
//! from the stored `'True'`/`'False'` string into a real JSON bool, so the WIRE
//! value is `true`/`false` (hence `Option<bool>`, not a string); `year` is a
//! string-encoded number (`"1999"`); `tags` /
//! `alternativeTitles` are `ast.literal_eval`'d into arrays (`[]` when empty);
//! `episode_number` on a wanted episode is a `'1x3'`-style label string.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response envelope for `GET /api/system/status` — wraps the payload under `data`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SystemStatus {
    /// Environment/version payload.
    pub data: Option<SystemStatusData>,
}

/// Environment information and versions (`system/status` dict). The handler always
/// sets every key, but they remain `Option` to tolerate older/newer Bazarr builds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SystemStatusData {
    /// Bazarr version, from the `BAZARR_VERSION` env.
    pub bazarr_version: Option<String>,
    /// Packaging version string; format `'<ver> by <author>'` when set, empty when
    /// not packaged.
    pub package_version: Option<String>,
    /// Detected Sonarr version (empty if not connected).
    pub sonarr_version: Option<String>,
    /// Detected Radarr version (empty if not connected).
    pub radarr_version: Option<String>,
    /// `platform.platform()` string.
    pub operating_system: Option<String>,
    /// `platform.python_version()`.
    pub python_version: Option<String>,
    /// `'<Dialect> <server_version>'` e.g. `'Sqlite'` or `'Postgresql 15.2'`; the
    /// version part may be empty.
    pub database_engine: Option<String>,
    /// Current alembic migration revision, or `'unknown'`.
    pub database_migration: Option<String>,
    /// Filesystem path to the Bazarr install dir.
    pub bazarr_directory: Option<String>,
    /// Filesystem path to the config dir (`args.config_dir`).
    pub bazarr_config_directory: Option<String>,
    /// Process start time as a Unix epoch float (`init.startTime = time.time()`).
    pub start_time: Option<f64>,
    /// Configured local timezone name, or `'Undefined'` / exception message on
    /// failure.
    pub timezone: Option<String>,
    /// `os.cpu_count()`; can be null when the OS cannot report it.
    pub cpu_cores: Option<i64>,
}

/// A language entry for a *missing* subtitle, from a `'code2[:forced|hi]'` string.
/// No `path`, no `file_size`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubtitleLanguage {
    /// Human language name (`language_from_alpha2`); may be null for unknown codes.
    pub name: Option<String>,
    /// ISO 639-1 alpha-2 code.
    pub code2: Option<String>,
    /// ISO 639-2 alpha-3 code (`alpha3_from_alpha2`); may be null.
    pub code3: Option<String>,
    /// Forced-subtitle flag (parsed from a `':forced'` suffix); default false.
    pub forced: Option<bool>,
    /// Hearing-impaired flag (parsed from a `':hi'` suffix); default false.
    pub hi: Option<bool>,
}

/// An existing subtitle track. Same as [`SubtitleLanguage`] plus `path` and
/// `file_size`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Subtitle {
    /// Human language name; may be null for unknown codes.
    pub name: Option<String>,
    /// ISO 639-1 alpha-2 code.
    pub code2: Option<String>,
    /// ISO 639-2 alpha-3 code; may be null.
    pub code3: Option<String>,
    /// Path-mapped subtitle file path; null/empty for embedded subs without a file.
    pub path: Option<String>,
    /// Forced flag; default false.
    pub forced: Option<bool>,
    /// Hearing-impaired flag; default false.
    pub hi: Option<bool>,
    /// Subtitle file size in bytes; defaults to `0` when the stored tuple omits it.
    pub file_size: Option<i64>,
}

/// An audio-track language entry. Declared as a single Nested in flask_restx, but
/// `postprocess` emits a *list*, so episode/movie rows model `audio_language` as
/// `Vec<AudioLanguage>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AudioLanguage {
    /// Human language name; may be null.
    pub name: Option<String>,
    /// ISO 639-1 alpha-2 code; may be null.
    pub code2: Option<String>,
    /// ISO 639-2 alpha-3 code; may be null.
    pub code3: Option<String>,
}

/// Per-episode subtitle/metadata row. `GET /api/episodes` returns these inside a
/// `{ "data": [...] }` envelope with *no* `total`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EpisodeSubtitleStatus {
    /// Audio track languages (a list, despite the flask_restx single-Nested
    /// declaration).
    #[serde(default)]
    pub audio_language: Vec<AudioLanguage>,
    /// Episode number within the season.
    pub episode: Option<i64>,
    /// Languages still missing subtitles; `[]` when none.
    #[serde(default)]
    pub missing_subtitles: Vec<SubtitleLanguage>,
    /// Whether the episode is monitored (a real bool on the wire — Bazarr's
    /// `postprocess` coerces its stored `'True'`/`'False'` string server-side).
    pub monitored: Option<bool>,
    /// Path-mapped episode file path.
    pub path: Option<String>,
    /// Season number.
    pub season: Option<i64>,
    /// Sonarr episode id.
    #[serde(rename = "sonarrEpisodeId")]
    pub sonarr_episode_id: Option<i64>,
    /// Sonarr series id.
    #[serde(rename = "sonarrSeriesId")]
    pub sonarr_series_id: Option<i64>,
    /// Existing subtitle tracks; `[]` when none.
    #[serde(default)]
    pub subtitles: Vec<Subtitle>,
    /// Episode title.
    pub title: Option<String>,
    /// Scene/release name of the episode file.
    #[serde(rename = "sceneName")]
    pub scene_name: Option<String>,
}

/// Per-movie subtitle/metadata row, returned inside [`MoviesGetResponse`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MovieSubtitleStatus {
    /// Alternate titles; `ast.literal_eval`'d from a stored string; `[]` when none.
    #[serde(rename = "alternativeTitles", default)]
    pub alternative_titles: Vec<String>,
    /// Audio track languages (a list).
    #[serde(default)]
    pub audio_language: Vec<AudioLanguage>,
    /// Proxy fanart image URL; null when absent.
    pub fanart: Option<String>,
    /// IMDb id string.
    #[serde(rename = "imdbId")]
    pub imdb_id: Option<String>,
    /// Languages still missing subtitles; `[]` when none.
    #[serde(default)]
    pub missing_subtitles: Vec<SubtitleLanguage>,
    /// Whether monitored (a real bool on the wire — coerced server-side from a
    /// stored `'True'`/`'False'` string by Bazarr's `postprocess`).
    pub monitored: Option<bool>,
    /// Plot overview.
    pub overview: Option<String>,
    /// Path-mapped movie file path.
    pub path: Option<String>,
    /// Proxy poster image URL; null when absent.
    pub poster: Option<String>,
    /// Languages profile id; null when unset.
    #[serde(rename = "profileId")]
    pub profile_id: Option<i64>,
    /// Radarr movie id.
    #[serde(rename = "radarrId")]
    pub radarr_id: Option<i64>,
    /// Scene/release name.
    #[serde(rename = "sceneName")]
    pub scene_name: Option<String>,
    /// Existing subtitle tracks; `[]` when none.
    #[serde(default)]
    pub subtitles: Vec<Subtitle>,
    /// Tags; `ast.literal_eval`'d; `[]` when none.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Movie title.
    pub title: Option<String>,
    /// Release year as a *string*-encoded number, e.g. `"1999"`.
    pub year: Option<String>,
}

/// Paged envelope for `GET /api/movies`: `{ "data": [...], "total": <int> }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MoviesGetResponse {
    /// Movie rows.
    #[serde(default)]
    pub data: Vec<MovieSubtitleStatus>,
    /// Total row count (ignores paging).
    pub total: Option<i64>,
}

/// A series episode awaiting subtitles, returned inside [`WantedEpisodesResponse`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WantedEpisode {
    /// Series title.
    #[serde(rename = "seriesTitle")]
    pub series_title: Option<String>,
    /// Season/episode label as a string like `'1x3'` (season concat `'x'` concat
    /// episode).
    pub episode_number: Option<String>,
    /// Episode title.
    #[serde(rename = "episodeTitle")]
    pub episode_title: Option<String>,
    /// Languages still missing; `[]` when none.
    #[serde(default)]
    pub missing_subtitles: Vec<SubtitleLanguage>,
    /// Sonarr series id.
    #[serde(rename = "sonarrSeriesId")]
    pub sonarr_series_id: Option<i64>,
    /// Sonarr episode id.
    #[serde(rename = "sonarrEpisodeId")]
    pub sonarr_episode_id: Option<i64>,
    /// Scene/release name.
    #[serde(rename = "sceneName")]
    pub scene_name: Option<String>,
    /// Series tags; `[]` when none.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Series type (e.g. standard/anime/daily).
    #[serde(rename = "seriesType")]
    pub series_type: Option<String>,
}

/// Paged envelope for `GET /api/episodes/wanted`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WantedEpisodesResponse {
    /// Wanted episode rows.
    #[serde(default)]
    pub data: Vec<WantedEpisode>,
    /// Total wanted count.
    pub total: Option<i64>,
}

/// A movie awaiting subtitles, returned inside [`WantedMoviesResponse`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WantedMovie {
    /// Movie title.
    pub title: Option<String>,
    /// Languages still missing; `[]` when none.
    #[serde(default)]
    pub missing_subtitles: Vec<SubtitleLanguage>,
    /// Radarr movie id.
    #[serde(rename = "radarrId")]
    pub radarr_id: Option<i64>,
    /// Scene/release name.
    #[serde(rename = "sceneName")]
    pub scene_name: Option<String>,
    /// Movie tags; `[]` when none.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Paged envelope for `GET /api/movies/wanted`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WantedMoviesResponse {
    /// Wanted movie rows.
    #[serde(default)]
    pub data: Vec<WantedMovie>,
    /// Total wanted count.
    pub total: Option<i64>,
}

/// A provider throttling/status row. `GET /api/providers` returns these inside a
/// `{ "data": [...] }` envelope (the flask_restx model id is misnamed
/// `MovieBlacklistGetResponse` in the source — not a field).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProviderStatus {
    /// Provider name.
    pub name: Option<String>,
    /// Status text: `'Good'` when not throttled, `'History'` for history-filter
    /// mode, otherwise the throttle reason.
    pub status: Option<String>,
    /// Retry time as a string; `'-'` when not throttling/now, otherwise a human
    /// time-to-retry.
    pub retry: Option<String>,
}

/// A language available for filters/profiles. `GET /api/system/languages` returns a
/// *bare* sorted array of these (no envelope).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Language {
    /// Human language name (`language_from_alpha2`).
    pub name: Option<String>,
    /// ISO 639-1 alpha-2 code.
    pub code2: Option<String>,
    /// ISO 639-2 alpha-3 code.
    pub code3: Option<String>,
    /// Whether the language is enabled in settings. Always false in the `?history`
    /// filter branch (a compatibility stub).
    pub enabled: Option<bool>,
}

#[cfg(test)]
#[path = "bazarr_tests.rs"]
mod tests;
