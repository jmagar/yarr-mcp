//! Radarr V3 models — `/api/v3` movie automation.
//!
//! Generated from the Radarr `develop` OpenAPI spec
//! (`src/Radarr.Api.V3/openapi.json`, openapi 3.0.4, info.version 3.0.0). Every
//! Radarr V3 resource serialises `camelCase`, so `rename_all = "camelCase"` is
//! applied throughout; the one reserved-word exception is `RatingChild.type`,
//! renamed to `kind`.
//!
//! Nullability quirk: the spec marks *nothing* as required (every target schema
//! declares `required: []`). To stay practical, value-type primitives that are
//! never `nullable` in the spec (e.g. `MovieResource.year`, `monitored`,
//! `qualityProfileId`) are typed as bare `i64` / `bool` / `f64` since they are
//! guaranteed present in responses; everything the spec flags `nullable:true`
//! (reference objects, strings, nullable ints) is `Option<…>`. Unknown upstream
//! fields deserialize-and-ignore via serde defaults.
//!
//! Date/time fields are ISO-8601 `date-time` strings (RFC3339), kept as
//! `String`. The `.NET` `date-span` fields (`QueueResource.timeleft`,
//! `CommandResource.duration`) are `TimeSpan` strings like `"00:10:00"`, also
//! kept as `String`. `HistoryResource.data` is an `additionalProperties: string`
//! map. `ReleaseResource.indexerFlags` is untyped in the spec (an indexer-flags
//! bitmask in practice) and is modelled as raw JSON.
//!
//! The two `*PagingResource` envelopes (`QueueResourcePagingResource`,
//! `HistoryResourcePagingResource`) share an identical paged shape and are
//! expressed as a single generic [`PagingResource`].

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A movie in Radarr. Produced by `GET /api/v3/movie`, `/api/v3/movie/{id}`,
/// calendar, and wanted endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieResource {
    /// Movie database id (int32).
    pub id: i64,
    /// Movie title. Nullable.
    pub title: Option<String>,
    /// Original-language title. Nullable.
    pub original_title: Option<String>,
    /// Original language object.
    pub original_language: Option<Language>,
    /// Alternate titles. Nullable array.
    #[serde(default)]
    pub alternate_titles: Vec<AlternativeTitleResource>,
    /// Secondary release year (int32, nullable).
    pub secondary_year: Option<i64>,
    /// Source id for secondary year (int32).
    pub secondary_year_source_id: i64,
    /// Sort-normalized title. Nullable.
    pub sort_title: Option<String>,
    /// Bytes on disk (int64, nullable).
    pub size_on_disk: Option<i64>,
    /// Movie status (tba/announced/inCinemas/released/deleted).
    pub status: Option<MovieStatusType>,
    /// Synopsis. Nullable.
    pub overview: Option<String>,
    /// Theatrical release date (RFC3339 date-time, nullable).
    pub in_cinemas: Option<String>,
    /// Physical release date (date-time, nullable).
    pub physical_release: Option<String>,
    /// Digital release date (date-time, nullable).
    pub digital_release: Option<String>,
    /// Effective release date (date-time, nullable).
    pub release_date: Option<String>,
    /// Note attached to physical release. Nullable.
    pub physical_release_note: Option<String>,
    /// Poster/fanart/etc images. Nullable array.
    #[serde(default)]
    pub images: Vec<MediaCover>,
    /// Official website URL. Nullable.
    pub website: Option<String>,
    /// Remote poster URL (lookup results). Nullable.
    pub remote_poster: Option<String>,
    /// Release year (int32).
    pub year: i64,
    /// YouTube trailer id. Nullable.
    pub you_tube_trailer_id: Option<String>,
    /// Production studio. Nullable.
    pub studio: Option<String>,
    /// On-disk folder path. Nullable.
    pub path: Option<String>,
    /// Assigned quality profile id (int32).
    pub quality_profile_id: i64,
    /// Whether a movie file exists (nullable).
    pub has_file: Option<bool>,
    /// Id of the associated movie file (int32; 0 if none).
    pub movie_file_id: i64,
    /// Whether the movie is monitored.
    pub monitored: bool,
    /// Minimum availability before searching (MovieStatusType enum).
    pub minimum_availability: Option<MovieStatusType>,
    /// Whether the movie has reached minimum availability.
    pub is_available: bool,
    /// Folder name. Nullable.
    pub folder_name: Option<String>,
    /// Runtime in minutes (int32).
    pub runtime: i64,
    /// Cleaned/normalized title. Nullable.
    pub clean_title: Option<String>,
    /// IMDb id string (e.g. tt1234567). Nullable.
    pub imdb_id: Option<String>,
    /// TMDb id (int32).
    pub tmdb_id: i64,
    /// URL slug. Nullable.
    pub title_slug: Option<String>,
    /// Root folder path. Nullable.
    pub root_folder_path: Option<String>,
    /// Folder name relative to root. Nullable.
    pub folder: Option<String>,
    /// Content rating/certification. Nullable.
    pub certification: Option<String>,
    /// Genre list. Nullable array.
    #[serde(default)]
    pub genres: Vec<String>,
    /// Keyword list. Nullable array.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Tag ids (int32). Nullable array.
    #[serde(default)]
    pub tags: Vec<i64>,
    /// When the movie was added (date-time).
    pub added: Option<String>,
    /// Options used at add time (write-mostly).
    pub add_options: Option<AddMovieOptions>,
    /// Aggregate ratings.
    pub ratings: Option<Ratings>,
    /// Associated movie file (when hasFile).
    pub movie_file: Option<MovieFileResource>,
    /// Owning collection summary (title + tmdbId).
    pub collection: Option<MovieCollectionResource>,
    /// TMDb popularity score (float).
    pub popularity: f64,
    /// Last automatic search time (date-time, nullable).
    pub last_search_time: Option<String>,
    /// File-count/size statistics.
    pub statistics: Option<MovieStatisticsResource>,
}

/// Per-movie file statistics (`MovieResource.statistics`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieStatisticsResource {
    /// Number of files (int32).
    pub movie_file_count: i64,
    /// Total bytes on disk (int64).
    pub size_on_disk: i64,
    /// Distinct release groups. Nullable array.
    #[serde(default)]
    pub release_groups: Vec<String>,
}

/// An image (poster/fanart/etc). `MovieResource.images[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaCover {
    /// Image type enum.
    pub cover_type: Option<MediaCoverTypes>,
    /// Local URL. Nullable.
    pub url: Option<String>,
    /// Remote source URL. Nullable.
    pub remote_url: Option<String>,
}

/// Image type. Serialised as lowercase camelCase strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MediaCoverTypes {
    Unknown,
    Poster,
    Banner,
    Fanart,
    Screenshot,
    Headshot,
    Clearlogo,
}

/// Aggregate ratings from multiple sources (`MovieResource.ratings`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Ratings {
    /// IMDb rating.
    pub imdb: Option<RatingChild>,
    /// TMDb rating.
    pub tmdb: Option<RatingChild>,
    /// Metacritic rating.
    pub metacritic: Option<RatingChild>,
    /// Rotten Tomatoes rating.
    pub rotten_tomatoes: Option<RatingChild>,
    /// Trakt rating.
    pub trakt: Option<RatingChild>,
}

/// A single rating value/votes pair (the `type` key is reserved → renamed to
/// `kind`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RatingChild {
    /// Vote count (int32).
    pub votes: i64,
    /// Rating value (double).
    pub value: f64,
    /// Reserved-word key `type` → renamed to `kind`. RatingType enum
    /// (user/critic).
    #[serde(rename = "type")]
    pub kind: Option<RatingType>,
}

/// Whether the rating is user- or critic-sourced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RatingType {
    User,
    Critic,
}

/// Options sent when adding a movie (`MovieResource.addOptions`). Write-side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddMovieOptions {
    /// Legacy flag carried from *arr base; ignore items already having files.
    pub ignore_episodes_with_files: bool,
    /// Legacy flag; ignore items without files.
    pub ignore_episodes_without_files: bool,
    /// Monitoring policy (movieOnly/movieAndCollection/none).
    pub monitor: Option<MonitorTypes>,
    /// Trigger a search immediately after adding.
    pub search_for_movie: bool,
    /// How the movie was added (manual/list/collection).
    pub add_method: Option<AddMovieMethod>,
}

/// Monitoring scope when adding a movie.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MonitorTypes {
    MovieOnly,
    MovieAndCollection,
    None,
}

/// Provenance of an add operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AddMovieMethod {
    Manual,
    List,
    Collection,
}

/// Lightweight collection reference embedded in `MovieResource.collection`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieCollectionResource {
    /// Collection title. Nullable.
    pub title: Option<String>,
    /// TMDb collection id (int32).
    pub tmdb_id: i64,
}

/// Movie release status. Used for both `.status` and `.minimumAvailability`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MovieStatusType {
    Tba,
    Announced,
    InCinemas,
    Released,
    Deleted,
}

/// A language reference (id + name). Appears widely (`originalLanguage`,
/// `languages[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    /// Language id (int32).
    pub id: i64,
    /// Language name. Nullable.
    pub name: Option<String>,
}

/// An alternate movie title (`MovieResource.alternateTitles[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlternativeTitleResource {
    /// Row id (int32).
    pub id: i64,
    /// Where the title came from (tmdb/mappings/user/indexer).
    pub source_type: Option<SourceType>,
    /// Linked movie metadata id (int32).
    pub movie_metadata_id: i64,
    /// Alternate title text. Nullable.
    pub title: Option<String>,
    /// Normalized alternate title. Nullable.
    pub clean_title: Option<String>,
}

/// Source of an alternate title.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceType {
    Tmdb,
    Mappings,
    User,
    Indexer,
}

/// An imported movie file. `GET/PUT /api/v3/moviefile`; also embedded as
/// `MovieResource.movieFile`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MovieFileResource {
    /// File id (int32).
    pub id: i64,
    /// Owning movie id (int32).
    pub movie_id: i64,
    /// Path relative to the movie folder. Nullable.
    pub relative_path: Option<String>,
    /// Absolute file path. Nullable.
    pub path: Option<String>,
    /// File size in bytes (int64).
    pub size: i64,
    /// When the file was imported (date-time).
    pub date_added: Option<String>,
    /// Original scene release name. Nullable.
    pub scene_name: Option<String>,
    /// Release group. Nullable.
    pub release_group: Option<String>,
    /// Edition label (e.g. Director's Cut). Nullable.
    pub edition: Option<String>,
    /// Detected languages. Nullable array.
    #[serde(default)]
    pub languages: Vec<Language>,
    /// Quality/revision model.
    pub quality: Option<QualityModel>,
    /// Matched custom formats. Nullable array.
    #[serde(default)]
    pub custom_formats: Vec<CustomFormatResource>,
    /// Total custom-format score (int32, nullable).
    pub custom_format_score: Option<i64>,
    /// Indexer flag bitmask (int32, nullable).
    pub indexer_flags: Option<i64>,
    /// Probed media info (codecs, resolution, etc).
    pub media_info: Option<MediaInfoResource>,
    /// Pre-import source path. Nullable.
    pub original_file_path: Option<String>,
    /// True if the profile cutoff is not yet satisfied.
    pub quality_cutoff_not_met: bool,
}

/// Probed media-info for a file (`MovieFileResource.mediaInfo`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfoResource {
    /// Media-info id (int32).
    pub id: i64,
    /// Audio bitrate (int64).
    pub audio_bitrate: i64,
    /// Audio channel count (double, e.g. 5.1).
    pub audio_channels: f64,
    /// Audio codec. Nullable.
    pub audio_codec: Option<String>,
    /// Comma/slash-joined audio languages string. Nullable.
    pub audio_languages: Option<String>,
    /// Number of audio streams (int32).
    pub audio_stream_count: i64,
    /// Video bit depth (int32).
    pub video_bit_depth: i64,
    /// Video bitrate (int64).
    pub video_bitrate: i64,
    /// Video codec. Nullable.
    pub video_codec: Option<String>,
    /// Frames per second (double).
    pub video_fps: f64,
    /// Dynamic range label. Nullable.
    pub video_dynamic_range: Option<String>,
    /// HDR type (e.g. HDR10, DV). Nullable.
    pub video_dynamic_range_type: Option<String>,
    /// Resolution string (e.g. 1920x1080). Nullable.
    pub resolution: Option<String>,
    /// Runtime string (HH:MM:SS). Nullable.
    pub run_time: Option<String>,
    /// Progressive/Interlaced. Nullable.
    pub scan_type: Option<String>,
    /// Subtitle languages string. Nullable.
    pub subtitles: Option<String>,
}

/// Quality + revision pair attached to files, queue items, history and
/// releases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityModel {
    /// Quality definition.
    pub quality: Option<Quality>,
    /// Revision (proper/repack tracking).
    pub revision: Option<Revision>,
}

/// A quality definition (`QualityModel.quality`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Quality {
    /// Quality id (int32).
    pub id: i64,
    /// Quality name. Nullable.
    pub name: Option<String>,
    /// Source enum (bluray/webdl/dvd/...).
    pub source: Option<QualitySource>,
    /// Resolution in lines (int32, e.g. 1080).
    pub resolution: i64,
    /// Quality modifier (remux/brdisk/...).
    pub modifier: Option<Modifier>,
}

/// Origin/source of a quality.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QualitySource {
    Unknown,
    Cam,
    Telesync,
    Telecine,
    Workprint,
    Dvd,
    Tv,
    Webdl,
    Webrip,
    Bluray,
}

/// Quality modifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Modifier {
    None,
    Regional,
    Screener,
    Rawhd,
    Brdisk,
    Remux,
}

/// Quality revision metadata (proper/repack).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    /// Revision/proper version (int32).
    pub version: i64,
    /// REAL count (int32).
    pub real: i64,
    /// Whether this is a repack.
    pub is_repack: bool,
}

/// A custom format (embedded in files/queue/history/releases as
/// `customFormats[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomFormatResource {
    /// Custom format id (int32).
    pub id: i64,
    /// Format name. Nullable.
    pub name: Option<String>,
    /// Include this format token when renaming (nullable).
    pub include_custom_format_when_renaming: Option<bool>,
    /// `CustomFormatSpecificationSchema[]` (polymorphic; not expanded here).
    /// Nullable array.
    #[serde(default)]
    pub specifications: Vec<serde_json::Value>,
}

/// A quality profile. `GET/POST/PUT /api/v3/qualityprofile`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityProfileResource {
    /// Profile id (int32).
    pub id: i64,
    /// Profile name. Nullable.
    pub name: Option<String>,
    /// Whether upgrades are permitted.
    pub upgrade_allowed: bool,
    /// Cutoff quality id (int32).
    pub cutoff: i64,
    /// Ordered quality items (groups + qualities). Nullable array.
    #[serde(default)]
    pub items: Vec<QualityProfileQualityItemResource>,
    /// Minimum acceptable custom-format score (int32).
    pub min_format_score: i64,
    /// Format-score cutoff to stop upgrading (int32).
    pub cutoff_format_score: i64,
    /// Minimum score delta to trigger an upgrade (int32).
    pub min_upgrade_format_score: i64,
    /// Per-custom-format scoring entries. Nullable array.
    #[serde(default)]
    pub format_items: Vec<ProfileFormatItemResource>,
    /// Preferred language for the profile.
    pub language: Option<Language>,
}

/// A quality item or group within a profile (self-referential: groups nest
/// `items[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityProfileQualityItemResource {
    /// Item/group id (int32). 0 for raw qualities.
    pub id: i64,
    /// Group name (null for a bare quality).
    pub name: Option<String>,
    /// Quality (null when this is a group).
    pub quality: Option<Quality>,
    /// Nested items (groups only). Self-referential. Nullable array.
    #[serde(default)]
    pub items: Vec<QualityProfileQualityItemResource>,
    /// Whether this quality/group is allowed.
    pub allowed: bool,
}

/// Custom-format scoring entry within a quality profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFormatItemResource {
    /// Row id (int32).
    pub id: i64,
    /// Custom format id (int32).
    pub format: i64,
    /// Custom format name. Nullable.
    pub name: Option<String>,
    /// Score assigned to this format (int32, may be negative).
    pub score: i64,
}

/// A download-queue item. Records of the queue paging envelope
/// (`GET /api/v3/queue`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueResource {
    /// Queue item id (int32).
    pub id: i64,
    /// Associated movie id (int32, nullable).
    pub movie_id: Option<i64>,
    /// Embedded movie (when includeMovie=true).
    pub movie: Option<MovieResource>,
    /// Languages. Nullable array.
    #[serde(default)]
    pub languages: Vec<Language>,
    /// Quality model.
    pub quality: Option<QualityModel>,
    /// Matched custom formats. Nullable array.
    #[serde(default)]
    pub custom_formats: Vec<CustomFormatResource>,
    /// Total custom-format score (int32).
    pub custom_format_score: i64,
    /// Total size in bytes (double).
    pub size: f64,
    /// Release/download title. Nullable.
    pub title: Option<String>,
    /// ETA (date-time, nullable).
    pub estimated_completion_time: Option<String>,
    /// When added to queue (date-time, nullable).
    pub added: Option<String>,
    /// Queue status enum.
    pub status: Option<QueueStatus>,
    /// Tracked download status (ok/warning/error).
    pub tracked_download_status: Option<TrackedDownloadStatus>,
    /// Tracked download state (downloading/importing/...).
    pub tracked_download_state: Option<TrackedDownloadState>,
    /// Per-item status messages. Nullable array.
    #[serde(default)]
    pub status_messages: Vec<TrackedDownloadStatusMessage>,
    /// Top-level error message. Nullable.
    pub error_message: Option<String>,
    /// Download-client item id/hash. Nullable.
    pub download_id: Option<String>,
    /// usenet/torrent/unknown.
    pub protocol: Option<DownloadProtocol>,
    /// Download client name. Nullable.
    pub download_client: Option<String>,
    /// Whether the client sets a post-import category.
    pub download_client_has_post_import_category: bool,
    /// Source indexer name. Nullable.
    pub indexer: Option<String>,
    /// Download output path. Nullable.
    pub output_path: Option<String>,
    /// Bytes remaining (double). Lowercase `sizeleft` on the wire.
    pub sizeleft: f64,
    /// Time remaining as a `.NET` TimeSpan string (format date-span, e.g.
    /// `"00:10:00"`). Nullable. Lowercase `timeleft` on the wire.
    pub timeleft: Option<String>,
}

/// Paged envelope shared by the queue and history endpoints:
/// `{ page, pageSize, sortKey, sortDirection, totalRecords, records: [...] }`.
/// Generic over the record type `T` so both `QueueResourcePagingResource` and
/// `HistoryResourcePagingResource` decode through one shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagingResource<T> {
    /// Current page (1-based, int32).
    pub page: i64,
    /// Page size (int32).
    pub page_size: i64,
    /// Sort field. Nullable.
    pub sort_key: Option<String>,
    /// Sort direction enum.
    pub sort_direction: Option<SortDirection>,
    /// Total matching records (int32).
    pub total_records: i64,
    /// Page of records. Always present in a paged response — no `#[serde(default)]`
    /// because a default on a generic `Vec<T>` field can't infer `T` here.
    pub records: Vec<T>,
}

/// Paged envelope for queue (`GET /api/v3/queue`).
pub type QueueResourcePagingResource = PagingResource<QueueResource>;

/// Paged envelope for history (`GET /api/v3/history`).
pub type HistoryResourcePagingResource = PagingResource<HistoryResource>;

/// A grouped status message for a tracked download.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackedDownloadStatusMessage {
    /// Message subject. Nullable.
    pub title: Option<String>,
    /// Message lines. Nullable array.
    #[serde(default)]
    pub messages: Vec<String>,
}

/// Queue item status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QueueStatus {
    Unknown,
    Queued,
    Paused,
    Downloading,
    Completed,
    Failed,
    Warning,
    Delay,
    DownloadClientUnavailable,
    Fallback,
}

/// Overall tracked-download health.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrackedDownloadStatus {
    Ok,
    Warning,
    Error,
}

/// Lifecycle state of a tracked download.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrackedDownloadState {
    Downloading,
    ImportBlocked,
    ImportPending,
    Importing,
    Imported,
    FailedPending,
    Failed,
    Ignored,
}

/// Download protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DownloadProtocol {
    Unknown,
    Usenet,
    Torrent,
}

/// Paging sort direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    Default,
    Ascending,
    Descending,
}

/// A history event. Records of the history paging envelope
/// (`GET /api/v3/history`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResource {
    /// History row id (int32).
    pub id: i64,
    /// Associated movie id (int32).
    pub movie_id: i64,
    /// Release/source title. Nullable.
    pub source_title: Option<String>,
    /// Languages. Nullable array.
    #[serde(default)]
    pub languages: Vec<Language>,
    /// Quality model.
    pub quality: Option<QualityModel>,
    /// Matched custom formats. Nullable array.
    #[serde(default)]
    pub custom_formats: Vec<CustomFormatResource>,
    /// Total custom-format score (int32).
    pub custom_format_score: i64,
    /// Whether the cutoff was unmet at event time.
    pub quality_cutoff_not_met: bool,
    /// Event timestamp (date-time).
    pub date: Option<String>,
    /// Download id/hash. Nullable.
    pub download_id: Option<String>,
    /// Event type enum (grabbed/downloadFolderImported/...).
    pub event_type: Option<MovieHistoryEventType>,
    /// Free-form key/value metadata (additionalProperties: string). Nullable
    /// map.
    pub data: Option<HashMap<String, String>>,
    /// Embedded movie (when includeMovie=true).
    pub movie: Option<MovieResource>,
}

/// Type of a history event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MovieHistoryEventType {
    Unknown,
    Grabbed,
    DownloadFolderImported,
    DownloadFailed,
    MovieFileDeleted,
    MovieFolderImported,
    MovieFileRenamed,
    DownloadIgnored,
}

/// A configured root folder. `GET/POST /api/v3/rootfolder`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RootFolderResource {
    /// Root folder id (int32).
    pub id: i64,
    /// Filesystem path. Nullable.
    pub path: Option<String>,
    /// Whether Radarr can access the path.
    pub accessible: bool,
    /// Free bytes on the volume (int64, nullable).
    pub free_space: Option<i64>,
    /// Folders under the root not linked to a movie. Nullable array.
    #[serde(default)]
    pub unmapped_folders: Vec<UnmappedFolder>,
}

/// An unmapped subfolder under a root folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnmappedFolder {
    /// Folder name. Nullable.
    pub name: Option<String>,
    /// Absolute path. Nullable.
    pub path: Option<String>,
    /// Path relative to the root. Nullable.
    pub relative_path: Option<String>,
}

/// A queued/running/completed command task. `GET/POST /api/v3/command`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandResource {
    /// Command id (int32).
    pub id: i64,
    /// Command name (e.g. RefreshMovie). Nullable.
    pub name: Option<String>,
    /// Human-readable command name. Nullable.
    pub command_name: Option<String>,
    /// Latest progress message. Nullable.
    pub message: Option<String>,
    /// Polymorphic Command body (base Command; concrete shape varies per
    /// command). Modeled as raw JSON.
    pub body: Option<serde_json::Value>,
    /// Scheduling priority (normal/high/low).
    pub priority: Option<CommandPriority>,
    /// Execution status enum.
    pub status: Option<CommandStatus>,
    /// Result enum (unknown/successful/unsuccessful).
    pub result: Option<CommandResult>,
    /// When queued (date-time).
    pub queued: Option<String>,
    /// When started (date-time, nullable).
    pub started: Option<String>,
    /// When ended (date-time, nullable).
    pub ended: Option<String>,
    /// Elapsed time as a TimeSpan string (date-span format). Nullable.
    pub duration: Option<String>,
    /// Exception text on failure. Nullable.
    pub exception: Option<String>,
    /// What triggered the command (manual/scheduled/unspecified).
    pub trigger: Option<CommandTrigger>,
    /// User-agent of the requesting client. Nullable.
    pub client_user_agent: Option<String>,
    /// Last state-change time (date-time, nullable).
    pub state_change_time: Option<String>,
    /// Whether progress is pushed to clients via SignalR.
    pub send_updates_to_client: bool,
    /// Whether this updates the scheduled task record.
    pub update_scheduled_task: bool,
    /// Previous execution time (date-time, nullable).
    pub last_execution_time: Option<String>,
}

/// Command scheduling priority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CommandPriority {
    Normal,
    High,
    Low,
}

/// Command execution status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CommandStatus {
    Queued,
    Started,
    Completed,
    Failed,
    Aborted,
    Cancelled,
    Orphaned,
}

/// Command outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CommandResult {
    Unknown,
    Successful,
    Unsuccessful,
}

/// What initiated a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CommandTrigger {
    Unspecified,
    Manual,
    Scheduled,
}

/// System/instance status. `GET /api/v3/system/status`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SystemResource {
    /// Application name (Radarr). Nullable.
    pub app_name: Option<String>,
    /// Configured instance name. Nullable.
    pub instance_name: Option<String>,
    /// Radarr version string. Nullable.
    pub version: Option<String>,
    /// Build timestamp (date-time).
    pub build_time: Option<String>,
    /// Debug build flag.
    pub is_debug: bool,
    /// Production build flag.
    pub is_production: bool,
    /// Running with admin privileges.
    pub is_admin: bool,
    /// Interactive session flag.
    pub is_user_interactive: bool,
    /// Process startup path. Nullable.
    pub startup_path: Option<String>,
    /// App data directory. Nullable.
    pub app_data: Option<String>,
    /// Operating system name. Nullable.
    pub os_name: Option<String>,
    /// OS version. Nullable.
    pub os_version: Option<String>,
    /// `.NET` Core runtime flag.
    pub is_net_core: bool,
    /// Running on Linux.
    pub is_linux: bool,
    /// Running on macOS.
    pub is_osx: bool,
    /// Running on Windows.
    pub is_windows: bool,
    /// Running inside Docker.
    pub is_docker: bool,
    /// Runtime mode (console/service/tray).
    pub mode: Option<RuntimeMode>,
    /// Update branch (e.g. master/develop). Nullable.
    pub branch: Option<String>,
    /// Database backend (sqLite/postgreSQL).
    pub database_type: Option<DatabaseType>,
    /// DB engine version. Nullable.
    pub database_version: Option<String>,
    /// Auth method (none/basic/forms/external).
    pub authentication: Option<AuthenticationType>,
    /// Applied DB migration number (int32).
    pub migration_version: i64,
    /// Configured URL base path. Nullable.
    pub url_base: Option<String>,
    /// `.NET` runtime version. Nullable.
    pub runtime_version: Option<String>,
    /// Runtime name. Nullable.
    pub runtime_name: Option<String>,
    /// Process start time (date-time).
    pub start_time: Option<String>,
    /// Distribution package version. Nullable.
    pub package_version: Option<String>,
    /// Package author/maintainer. Nullable.
    pub package_author: Option<String>,
    /// How updates are applied (builtIn/script/external/apt/docker).
    pub package_update_mechanism: Option<UpdateMechanism>,
    /// Notes about the update mechanism. Nullable.
    pub package_update_mechanism_message: Option<String>,
}

/// Process runtime mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeMode {
    Console,
    Service,
    Tray,
}

/// Database backend. Note camelCase quirk: `sqLite` (not sqlite/SQLite).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseType {
    SqLite,
    PostgreSQL,
}

/// Configured authentication method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AuthenticationType {
    None,
    Basic,
    Forms,
    External,
}

/// How the app receives updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UpdateMechanism {
    BuiltIn,
    Script,
    External,
    Apt,
    Docker,
}

/// A release from indexer search/lookup. `GET /api/v3/release` (release/lookup),
/// `POST /api/v3/release/push`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseResource {
    /// Synthetic release id (int32).
    pub id: i64,
    /// Indexer GUID. Nullable.
    pub guid: Option<String>,
    /// Parsed quality model.
    pub quality: Option<QualityModel>,
    /// Matched custom formats. Nullable array.
    #[serde(default)]
    pub custom_formats: Vec<CustomFormatResource>,
    /// Custom-format score (int32).
    pub custom_format_score: i64,
    /// Internal quality weight for sorting (int32).
    pub quality_weight: i64,
    /// Age in days (int32).
    pub age: i64,
    /// Age in hours (double).
    pub age_hours: f64,
    /// Age in minutes (double).
    pub age_minutes: f64,
    /// Release size in bytes (int64).
    pub size: i64,
    /// Source indexer id (int32).
    pub indexer_id: i64,
    /// Indexer name. Nullable.
    pub indexer: Option<String>,
    /// Release group. Nullable.
    pub release_group: Option<String>,
    /// Sub-group. Nullable.
    pub sub_group: Option<String>,
    /// Release hash. Nullable.
    pub release_hash: Option<String>,
    /// Release title. Nullable.
    pub title: Option<String>,
    /// Whether the release is scene-sourced.
    pub scene_source: bool,
    /// Parsed movie titles. Nullable array.
    #[serde(default)]
    pub movie_titles: Vec<String>,
    /// Parsed languages. Nullable array.
    #[serde(default)]
    pub languages: Vec<Language>,
    /// Mapped movie id (int32, nullable).
    pub mapped_movie_id: Option<i64>,
    /// Whether the release passed all checks.
    pub approved: bool,
    /// Rejected but retryable (e.g. delay).
    pub temporarily_rejected: bool,
    /// Permanently rejected.
    pub rejected: bool,
    /// Parsed TMDb id (int32).
    pub tmdb_id: i64,
    /// Parsed IMDb id as INTEGER (int32) — note: here `imdbId` is numeric,
    /// unlike `MovieResource.imdbId` which is a tt-string.
    pub imdb_id: i64,
    /// Rejection reasons. Nullable array.
    #[serde(default)]
    pub rejections: Vec<String>,
    /// Release publish date (date-time).
    pub publish_date: Option<String>,
    /// Indexer comment/details URL. Nullable.
    pub comment_url: Option<String>,
    /// NZB/torrent download URL. Nullable.
    pub download_url: Option<String>,
    /// Info page URL. Nullable.
    pub info_url: Option<String>,
    /// Whether the movie is monitored/requested.
    pub movie_requested: bool,
    /// Whether the grab is permitted.
    pub download_allowed: bool,
    /// Overall sort weight (int32).
    pub release_weight: i64,
    /// Edition label. Nullable.
    pub edition: Option<String>,
    /// Torrent magnet URL. Nullable.
    pub magnet_url: Option<String>,
    /// Torrent info hash. Nullable.
    pub info_hash: Option<String>,
    /// Torrent seeders (int32, nullable).
    pub seeders: Option<i64>,
    /// Torrent leechers (int32, nullable).
    pub leechers: Option<i64>,
    /// usenet/torrent/unknown.
    pub protocol: Option<DownloadProtocol>,
    /// Spec gives no type, only `nullable:true` — an untyped value. In practice
    /// an indexer-flags bitmask/enum. Modeled as raw JSON to be safe.
    pub indexer_flags: Option<serde_json::Value>,
    /// Movie id for push (int32, nullable).
    pub movie_id: Option<i64>,
    /// Target download client id (int32, nullable).
    pub download_client_id: Option<i64>,
    /// Target download client name. Nullable.
    pub download_client: Option<String>,
    /// Override checks when pushing (nullable).
    pub should_override: Option<bool>,
}

#[cfg(test)]
#[path = "radarr_tests.rs"]
mod tests;
