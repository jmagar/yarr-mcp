//! Sonarr models — the full `/api/v3` resource surface.
//!
//! This module mirrors Sonarr's complete OpenAPI schema as published in
//! `Sonarr/Sonarr` (`src/Sonarr.Api.V3/openapi.json`) — the full resource surface,
//! not a slim subset. Every body is
//! camelCase, so `rename_all = "camelCase"` is the default; explicit renames cover
//! the exceptions.
//!
//! Decoding quirks worth knowing (all encoded below):
//! - [`HealthResource`]'s `type` key is a reserved word → renamed to `kind`.
//! - [`HistoryResource::data`] is a free-form map whose values are *always*
//!   strings even when they encode numbers (`"size"`, `"tvdbId"`, …); the keys
//!   vary by [`EpisodeHistoryEventType`].
//! - The paged wrappers (`*PagingResource`) share one shape and carry
//!   `sortDirection` as a [`SortDirection`] *enum*, not a string, even though
//!   `sortKey` is a nullable string.
//! - Byte counts (`sizeOnDisk`, `size`, `freeSpace`) are `int64` → `i64`; large
//!   libraries overflow `i32`.
//! - [`QueueResource::size`] and [`ReleaseResource::age_hours`]/`age_minutes` are
//!   JSON doubles → `f64`; but [`ReleaseResource::size`] is `int64` → `i64`.
//! - `timeleft` / `duration` are .NET `TimeSpan` strings (e.g. `"00:13:37"`), not
//!   numbers → `String`. `sizeleft`/`timeleft` on the queue are DEPRECATED.
//! - Date-time fields are ISO-8601 strings (not epoch ints) → `String`.
//! - The quality tree ([`QualityProfileQualityItemResource`]) is recursive: a
//!   group has nested `items` and a null `quality`; `cutoff` is the quality id of
//!   the cutoff item (an `i32`), `upgradeAllowed` is the bool.
//!
//! Every non-guaranteed field is `Option<T>` and every array carries
//! `#[serde(default)]`, so unknown upstream fields deserialize-and-ignore and a
//! sparse object decodes to all-`None` / empty `Vec`. `Eq` is deliberately *not*
//! derived on types reachable from a float field.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ───────────────────────────── Series ─────────────────────────────

/// A TV series — the central Sonarr entity (`GET /api/v3/series`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeriesResource {
    /// Series id.
    pub id: Option<i32>,
    /// Series title.
    pub title: Option<String>,
    /// Alternate/scene titles.
    #[serde(default)]
    pub alternate_titles: Vec<AlternateTitleResource>,
    /// Normalized title for sorting.
    pub sort_title: Option<String>,
    /// Series status enum (continuing/ended/upcoming/deleted).
    pub status: Option<SeriesStatusType>,
    /// Whether the series has ended. readOnly.
    pub ended: Option<bool>,
    /// Resolved quality profile name.
    pub profile_name: Option<String>,
    /// Plot overview.
    pub overview: Option<String>,
    /// Next airing date-time (ISO-8601).
    pub next_airing: Option<String>,
    /// Previous airing date-time (ISO-8601).
    pub previous_airing: Option<String>,
    /// Originating network.
    pub network: Option<String>,
    /// Scheduled air time of day.
    pub air_time: Option<String>,
    /// Cover images.
    #[serde(default)]
    pub images: Vec<MediaCover>,
    /// Original language object `{id,name}`.
    pub original_language: Option<Language>,
    /// Remote poster URL (lookup results).
    pub remote_poster: Option<String>,
    /// Seasons.
    #[serde(default)]
    pub seasons: Vec<SeasonResource>,
    /// Release year.
    pub year: Option<i32>,
    /// Filesystem path on disk.
    pub path: Option<String>,
    /// Quality profile id.
    pub quality_profile_id: Option<i32>,
    /// Use season subfolders.
    pub season_folder: Option<bool>,
    /// Monitored for downloads.
    pub monitored: Option<bool>,
    /// New-item monitor policy enum.
    pub monitor_new_items: Option<NewItemMonitorTypes>,
    /// Use scene episode numbering.
    pub use_scene_numbering: Option<bool>,
    /// Episode runtime in minutes.
    pub runtime: Option<i32>,
    /// TheTVDB id.
    pub tvdb_id: Option<i32>,
    /// TVRage id.
    pub tv_rage_id: Option<i32>,
    /// TVmaze id.
    pub tv_maze_id: Option<i32>,
    /// TMDB id.
    pub tmdb_id: Option<i32>,
    /// First aired date-time (ISO-8601).
    pub first_aired: Option<String>,
    /// Last aired date-time (ISO-8601).
    pub last_aired: Option<String>,
    /// Series type enum (standard/daily/anime).
    pub series_type: Option<SeriesTypes>,
    /// Cleaned title for matching.
    pub clean_title: Option<String>,
    /// IMDb id.
    pub imdb_id: Option<String>,
    /// URL slug.
    pub title_slug: Option<String>,
    /// Root folder path.
    pub root_folder_path: Option<String>,
    /// Folder name.
    pub folder: Option<String>,
    /// Content rating/certification.
    pub certification: Option<String>,
    /// Genres.
    #[serde(default)]
    pub genres: Vec<String>,
    /// Tag ids (uniqueItems).
    #[serde(default)]
    pub tags: Vec<i32>,
    /// Date added to Sonarr (ISO-8601).
    pub added: Option<String>,
    /// Add-time options (write-only on POST).
    pub add_options: Option<AddSeriesOptions>,
    /// Aggregate ratings.
    pub ratings: Option<Ratings>,
    /// Series-level statistics.
    pub statistics: Option<SeriesStatisticsResource>,
    /// Whether episode list changed.
    pub episodes_changed: Option<bool>,
    /// DEPRECATED, readOnly legacy language profile id.
    pub language_profile_id: Option<i32>,
}

/// Series-level aggregate statistics (nested in [`SeriesResource::statistics`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeriesStatisticsResource {
    /// Number of seasons.
    pub season_count: Option<i32>,
    /// Episodes that have a file.
    pub episode_file_count: Option<i32>,
    /// Monitored/aired episode count.
    pub episode_count: Option<i32>,
    /// Total episodes including unaired/specials.
    pub total_episode_count: Option<i32>,
    /// Total bytes on disk (int64).
    pub size_on_disk: Option<i64>,
    /// Distinct release groups present.
    #[serde(default)]
    pub release_groups: Vec<String>,
    /// Percentage of episodes downloaded. readOnly (double).
    pub percent_of_episodes: Option<f64>,
}

/// A single season within a series (nested in `GET /api/v3/series`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeasonResource {
    /// Season number (0 = specials).
    pub season_number: Option<i32>,
    /// Whether the season is monitored.
    pub monitored: Option<bool>,
    /// Season-level statistics.
    pub statistics: Option<SeasonStatisticsResource>,
    /// Season cover images.
    #[serde(default)]
    pub images: Vec<MediaCover>,
}

/// Per-season aggregate statistics (nested in `GET /api/v3/series`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeasonStatisticsResource {
    /// Next airing date-time (ISO-8601).
    pub next_airing: Option<String>,
    /// Previous airing date-time (ISO-8601).
    pub previous_airing: Option<String>,
    /// Episodes with files in this season.
    pub episode_file_count: Option<i32>,
    /// Aired/monitored episode count.
    pub episode_count: Option<i32>,
    /// Total episodes in the season.
    pub total_episode_count: Option<i32>,
    /// Bytes on disk (int64).
    pub size_on_disk: Option<i64>,
    /// Release groups present.
    #[serde(default)]
    pub release_groups: Vec<String>,
    /// Percent of episodes downloaded. readOnly (double).
    pub percent_of_episodes: Option<f64>,
}

/// An image/cover reference (nested `images[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaCover {
    /// Image kind enum (unknown/poster/banner/fanart/screenshot/headshot/clearlogo).
    pub cover_type: Option<MediaCoverTypes>,
    /// Local Sonarr-served URL.
    pub url: Option<String>,
    /// Original remote URL.
    pub remote_url: Option<String>,
}

/// Aggregate rating (nested).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Ratings {
    /// Number of votes.
    pub votes: Option<i32>,
    /// Average rating value (double).
    pub value: Option<f64>,
}

/// Options applied when adding a series (`POST /api/v3/series`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddSeriesOptions {
    /// Skip monitoring episodes that already have files.
    pub ignore_episodes_with_files: Option<bool>,
    /// Skip monitoring episodes lacking files.
    pub ignore_episodes_without_files: Option<bool>,
    /// Which episodes to monitor (enum).
    pub monitor: Option<MonitorTypes>,
    /// Kick off search for missing on add.
    pub search_for_missing_episodes: Option<bool>,
    /// Search cutoff-unmet on add.
    pub search_for_cutoff_unmet_episodes: Option<bool>,
}

/// Alternate/scene title mapping (nested `alternateTitles[]`; also
/// [`ReleaseResource::scene_mapping`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlternateTitleResource {
    /// Alternate title.
    pub title: Option<String>,
    /// Season number this title applies to.
    pub season_number: Option<i32>,
    /// Scene season number mapping.
    pub scene_season_number: Option<i32>,
    /// Origin of the scene mapping.
    pub scene_origin: Option<String>,
    /// Free-text comment.
    pub comment: Option<String>,
}

// ───────────────────────────── Episodes ─────────────────────────────

/// A single episode (`GET /api/v3/episode`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeResource {
    /// Episode id.
    pub id: Option<i32>,
    /// Owning series id.
    pub series_id: Option<i32>,
    /// TheTVDB episode id.
    pub tvdb_id: Option<i32>,
    /// Linked episode file id (0 if none).
    pub episode_file_id: Option<i32>,
    /// Season number.
    pub season_number: Option<i32>,
    /// Episode number within season.
    pub episode_number: Option<i32>,
    /// Episode title.
    pub title: Option<String>,
    /// Air date (date-only string, e.g. `2023-01-15`).
    pub air_date: Option<String>,
    /// Air date-time UTC (ISO-8601).
    pub air_date_utc: Option<String>,
    /// Last automatic search time (ISO-8601).
    pub last_search_time: Option<String>,
    /// Runtime in minutes.
    pub runtime: Option<i32>,
    /// Finale designation (series/season/midseason) if any.
    pub finale_type: Option<String>,
    /// Episode synopsis.
    pub overview: Option<String>,
    /// Embedded file (when `includeEpisodeFile=true`).
    pub episode_file: Option<EpisodeFileResource>,
    /// Whether a file exists.
    pub has_file: Option<bool>,
    /// Whether episode is monitored.
    pub monitored: Option<bool>,
    /// Absolute episode number (anime).
    pub absolute_episode_number: Option<i32>,
    /// Scene absolute episode number.
    pub scene_absolute_episode_number: Option<i32>,
    /// Scene episode number.
    pub scene_episode_number: Option<i32>,
    /// Scene season number.
    pub scene_season_number: Option<i32>,
    /// Scene numbering not yet verified.
    pub unverified_scene_numbering: Option<bool>,
    /// Computed end time (ISO-8601).
    pub end_time: Option<String>,
    /// When release was grabbed (ISO-8601).
    pub grab_date: Option<String>,
    /// Embedded series (when `includeSeries=true`).
    pub series: Option<SeriesResource>,
    /// Episode images (screenshots).
    #[serde(default)]
    pub images: Vec<MediaCover>,
}

/// A media file backing one or more episodes (`GET /api/v3/episodefile`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeFileResource {
    /// File id.
    pub id: Option<i32>,
    /// Owning series id.
    pub series_id: Option<i32>,
    /// Season number.
    pub season_number: Option<i32>,
    /// Path relative to series folder.
    pub relative_path: Option<String>,
    /// Absolute file path.
    pub path: Option<String>,
    /// File size in bytes (int64).
    pub size: Option<i64>,
    /// Import date-time (ISO-8601).
    pub date_added: Option<String>,
    /// Original scene release name.
    pub scene_name: Option<String>,
    /// Release group.
    pub release_group: Option<String>,
    /// Detected languages.
    #[serde(default)]
    pub languages: Vec<Language>,
    /// Quality model `{quality,revision}`.
    pub quality: Option<QualityModel>,
    /// Matched custom formats.
    #[serde(default)]
    pub custom_formats: Vec<CustomFormatResource>,
    /// Aggregate custom format score.
    pub custom_format_score: Option<i32>,
    /// Bitmask of indexer flags.
    pub indexer_flags: Option<i32>,
    /// Release type enum (singleEpisode/multiEpisode/seasonPack).
    pub release_type: Option<ReleaseType>,
    /// Embedded media-info (codecs, resolution, etc). Untyped — free-form object.
    pub media_info: Option<serde_json::Value>,
    /// True if below quality cutoff.
    pub quality_cutoff_not_met: Option<bool>,
}

// ───────────────────────────── Quality ─────────────────────────────

/// A quality profile defining allowed qualities, cutoff and custom-format
/// scoring (`GET /api/v3/qualityprofile`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityProfileResource {
    /// Profile id.
    pub id: Option<i32>,
    /// Profile name.
    pub name: Option<String>,
    /// Whether upgrades are allowed.
    pub upgrade_allowed: Option<bool>,
    /// Quality (item) id at which upgrading stops.
    pub cutoff: Option<i32>,
    /// Ordered quality items / groups (recursive tree).
    #[serde(default)]
    pub items: Vec<QualityProfileQualityItemResource>,
    /// Minimum acceptable custom-format score.
    pub min_format_score: Option<i32>,
    /// Custom-format score at which upgrading stops.
    pub cutoff_format_score: Option<i32>,
    /// Minimum score delta to trigger an upgrade.
    pub min_upgrade_format_score: Option<i32>,
    /// Per-custom-format score assignments. Untyped — free-form objects.
    #[serde(default)]
    pub format_items: Vec<serde_json::Value>,
}

/// A quality item or quality group in a profile. Recursive: a group has nested
/// `items` and a null `quality`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityProfileQualityItemResource {
    /// Group id (set for groups; individual qualities use `quality.id`).
    pub id: Option<i32>,
    /// Group name (null for leaf quality items).
    pub name: Option<String>,
    /// The leaf quality (null when this is a group).
    pub quality: Option<Quality>,
    /// Nested items (recursive grouping).
    #[serde(default)]
    pub items: Vec<QualityProfileQualityItemResource>,
    /// Whether this quality/group is allowed.
    pub allowed: Option<bool>,
    /// Min size MB/min override (double).
    pub min_size: Option<f64>,
    /// Max size MB/min override (double).
    pub max_size: Option<f64>,
    /// Preferred size MB/min (double).
    pub preferred_size: Option<f64>,
}

/// Quality + revision wrapper (nested).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityModel {
    /// The quality.
    pub quality: Option<Quality>,
    /// Revision (version/real/repack).
    pub revision: Option<Revision>,
}

/// A concrete quality definition (nested).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Quality {
    /// Quality id.
    pub id: Option<i32>,
    /// Quality name (e.g. WEBDL-1080p).
    pub name: Option<String>,
    /// Source enum (television/web/bluray/etc).
    pub source: Option<QualitySource>,
    /// Vertical resolution (e.g. 1080).
    pub resolution: Option<i32>,
}

/// Quality revision/proper info (nested).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    /// Proper version number (1=base).
    pub version: Option<i32>,
    /// REAL count.
    pub real: Option<i32>,
    /// Whether release is a repack.
    pub is_repack: Option<bool>,
}

// ───────────────────────────── Queue ─────────────────────────────

/// An item in the download queue (`GET /api/v3/queue` records).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueResource {
    /// Queue item id.
    pub id: Option<i32>,
    /// Series id (null if unmapped).
    pub series_id: Option<i32>,
    /// Episode id (null if unmapped).
    pub episode_id: Option<i32>,
    /// Season number.
    pub season_number: Option<i32>,
    /// Embedded series (when `includeSeries=true`).
    pub series: Option<SeriesResource>,
    /// Embedded episode (when `includeEpisode=true`).
    pub episode: Option<EpisodeResource>,
    /// Languages of the release.
    #[serde(default)]
    pub languages: Vec<Language>,
    /// Quality model.
    pub quality: Option<QualityModel>,
    /// Matched custom formats.
    #[serde(default)]
    pub custom_formats: Vec<CustomFormatResource>,
    /// Custom format score.
    pub custom_format_score: Option<i32>,
    /// Total size in bytes (double — NOT int).
    pub size: Option<f64>,
    /// Release title.
    pub title: Option<String>,
    /// ETA date-time (ISO-8601).
    pub estimated_completion_time: Option<String>,
    /// When added to queue (ISO-8601).
    pub added: Option<String>,
    /// Queue status enum.
    pub status: Option<QueueStatus>,
    /// Tracked download status enum (ok/warning/error).
    pub tracked_download_status: Option<TrackedDownloadStatus>,
    /// Tracked download state enum (downloading/importing/etc).
    pub tracked_download_state: Option<TrackedDownloadState>,
    /// Per-item status messages.
    #[serde(default)]
    pub status_messages: Vec<TrackedDownloadStatusMessage>,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// Download client id/hash.
    pub download_id: Option<String>,
    /// Protocol enum (usenet/torrent/unknown).
    pub protocol: Option<DownloadProtocol>,
    /// Download client name.
    pub download_client: Option<String>,
    /// Client has a post-import category set.
    pub download_client_has_post_import_category: Option<bool>,
    /// Source indexer name.
    pub indexer: Option<String>,
    /// Download output path.
    pub output_path: Option<String>,
    /// Whether target episode already has a file.
    pub episode_has_file: Option<bool>,
    /// DEPRECATED. Bytes remaining (double).
    pub sizeleft: Option<f64>,
    /// DEPRECATED. Time remaining as a .NET `TimeSpan` string (date-span), e.g.
    /// `00:13:37` — NOT a number.
    pub timeleft: Option<String>,
}

/// A grouped status message for a queue item (nested `statusMessages[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackedDownloadStatusMessage {
    /// Message title/subject.
    pub title: Option<String>,
    /// Detail message lines.
    #[serde(default)]
    pub messages: Vec<String>,
}

/// Paged wrapper for queue records (`GET /api/v3/queue`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueResourcePagingResource {
    /// 1-based page index.
    pub page: Option<i32>,
    /// Records per page.
    pub page_size: Option<i32>,
    /// Active sort field.
    pub sort_key: Option<String>,
    /// Sort direction enum (default/ascending/descending) — NOT a string.
    pub sort_direction: Option<SortDirection>,
    /// Total matching records across pages.
    pub total_records: Option<i32>,
    /// The page of queue items.
    #[serde(default)]
    pub records: Vec<QueueResource>,
}

// ───────────────────────────── History ─────────────────────────────

/// A history event entry (`GET /api/v3/history` records).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResource {
    /// History entry id.
    pub id: Option<i32>,
    /// Episode id.
    pub episode_id: Option<i32>,
    /// Series id.
    pub series_id: Option<i32>,
    /// Original release/source title.
    pub source_title: Option<String>,
    /// Languages.
    #[serde(default)]
    pub languages: Vec<Language>,
    /// Quality model.
    pub quality: Option<QualityModel>,
    /// Matched custom formats.
    #[serde(default)]
    pub custom_formats: Vec<CustomFormatResource>,
    /// Custom format score.
    pub custom_format_score: Option<i32>,
    /// Whether below cutoff.
    pub quality_cutoff_not_met: Option<bool>,
    /// Event date-time (ISO-8601).
    pub date: Option<String>,
    /// Download client id/hash.
    pub download_id: Option<String>,
    /// Event type enum (grabbed/downloadFolderImported/downloadFailed/…).
    pub event_type: Option<EpisodeHistoryEventType>,
    /// Free-form metadata map; values are ALWAYS strings even for numeric data.
    /// Keys vary by `eventType` (e.g. `indexer`, `releaseGroup`, `size`, `age`,
    /// `downloadClient`, `tvdbId`, `publishedDate`, `droppedPath`, `importedPath`,
    /// `reason`).
    pub data: Option<std::collections::HashMap<String, Option<String>>>,
    /// Embedded episode (when `includeEpisode=true`).
    pub episode: Option<EpisodeResource>,
    /// Embedded series (when `includeSeries=true`).
    pub series: Option<SeriesResource>,
}

/// Paged wrapper for history records (`GET /api/v3/history`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResourcePagingResource {
    /// Page index.
    pub page: Option<i32>,
    /// Records per page.
    pub page_size: Option<i32>,
    /// Sort field.
    pub sort_key: Option<String>,
    /// Sort direction enum.
    pub sort_direction: Option<SortDirection>,
    /// Total records.
    pub total_records: Option<i32>,
    /// Page of history entries.
    #[serde(default)]
    pub records: Vec<HistoryResource>,
}

// ───────────────────────────── Root folders ─────────────────────────────

/// A configured root folder (`GET /api/v3/rootfolder`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RootFolderResource {
    /// Root folder id.
    pub id: Option<i32>,
    /// Filesystem path.
    pub path: Option<String>,
    /// Whether the path is accessible.
    pub accessible: Option<bool>,
    /// Free bytes available (int64, nullable).
    pub free_space: Option<i64>,
    /// Folders present but not mapped to a series.
    #[serde(default)]
    pub unmapped_folders: Vec<UnmappedFolder>,
}

/// A folder under a root not mapped to a series (nested in rootfolder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnmappedFolder {
    /// Folder name.
    pub name: Option<String>,
    /// Absolute path.
    pub path: Option<String>,
    /// Path relative to the root folder.
    pub relative_path: Option<String>,
}

// ───────────────────────────── Commands ─────────────────────────────

/// A queued/running/completed command task (`GET`/`POST /api/v3/command`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandResource {
    /// Command id.
    pub id: Option<i32>,
    /// Command display name.
    pub name: Option<String>,
    /// Command type name (e.g. RefreshSeries).
    pub command_name: Option<String>,
    /// Current progress message.
    pub message: Option<String>,
    /// Command body/payload object.
    pub body: Option<Command>,
    /// Priority enum (normal/high/low).
    pub priority: Option<CommandPriority>,
    /// Status enum (queued/started/completed/failed/aborted/cancelled/orphaned).
    pub status: Option<CommandStatus>,
    /// Result enum (unknown/successful/unsuccessful).
    pub result: Option<CommandResult>,
    /// Queued date-time (ISO-8601).
    pub queued: Option<String>,
    /// Started date-time (ISO-8601).
    pub started: Option<String>,
    /// Ended date-time (ISO-8601).
    pub ended: Option<String>,
    /// Duration as a .NET `TimeSpan` string (date-span).
    pub duration: Option<String>,
    /// Exception text if failed.
    pub exception: Option<String>,
    /// What triggered the command (enum).
    pub trigger: Option<CommandTrigger>,
    /// User agent of the requesting client.
    pub client_user_agent: Option<String>,
    /// Last state change date-time (ISO-8601).
    pub state_change_time: Option<String>,
    /// Push progress to clients.
    pub send_updates_to_client: Option<bool>,
    /// Update scheduled task timing.
    pub update_scheduled_task: Option<bool>,
    /// Last execution date-time (ISO-8601).
    pub last_execution_time: Option<String>,
}

/// Command payload/base (nested in [`CommandResource::body`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    /// Push progress to clients.
    pub send_updates_to_client: Option<bool>,
    /// readOnly. Update scheduled task.
    pub update_scheduled_task: Option<bool>,
    /// readOnly. Message on completion.
    pub completion_message: Option<String>,
    /// readOnly. Needs disk access.
    pub requires_disk_access: Option<bool>,
    /// readOnly. Runs exclusively.
    pub is_exclusive: Option<bool>,
    /// readOnly. Long-running task.
    pub is_long_running: Option<bool>,
    /// readOnly. Command name.
    pub name: Option<String>,
    /// Last execution date-time (ISO-8601).
    pub last_execution_time: Option<String>,
    /// Last start date-time (ISO-8601).
    pub last_start_time: Option<String>,
    /// Trigger enum.
    pub trigger: Option<CommandTrigger>,
    /// Suppress progress messages.
    pub suppress_messages: Option<bool>,
    /// Requesting client user agent.
    pub client_user_agent: Option<String>,
}

// ───────────────────────────── Health & system ─────────────────────────────

/// A health check result (`GET /api/v3/health`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResource {
    /// Health entry id.
    pub id: Option<i32>,
    /// Originating health-check name.
    pub source: Option<String>,
    /// RESERVED KEY `type` → renamed to `kind`. Severity enum
    /// (ok/notice/warning/error).
    #[serde(rename = "type")]
    pub kind: Option<HealthCheckResult>,
    /// Human-readable message.
    pub message: Option<String>,
    /// Wiki link (HttpUri serialized as string).
    pub wiki_url: Option<String>,
}

/// System/instance status (`GET /api/v3/system/status`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SystemResource {
    /// Application name (Sonarr).
    pub app_name: Option<String>,
    /// Configured instance name.
    pub instance_name: Option<String>,
    /// App version string.
    pub version: Option<String>,
    /// Build date-time (ISO-8601).
    pub build_time: Option<String>,
    /// Debug build.
    pub is_debug: Option<bool>,
    /// Production build.
    pub is_production: Option<bool>,
    /// Running as admin.
    pub is_admin: Option<bool>,
    /// Interactive session.
    pub is_user_interactive: Option<bool>,
    /// Binary startup path.
    pub startup_path: Option<String>,
    /// AppData/config path.
    pub app_data: Option<String>,
    /// OS name.
    pub os_name: Option<String>,
    /// OS version.
    pub os_version: Option<String>,
    /// Running on .NET Core.
    pub is_net_core: Option<bool>,
    /// Linux host.
    pub is_linux: Option<bool>,
    /// macOS host.
    pub is_osx: Option<bool>,
    /// Windows host.
    pub is_windows: Option<bool>,
    /// Running in Docker.
    pub is_docker: Option<bool>,
    /// Runtime mode enum (console/service/tray).
    pub mode: Option<RuntimeMode>,
    /// Update branch.
    pub branch: Option<String>,
    /// Authentication type enum.
    pub authentication: Option<AuthenticationType>,
    /// SQLite version.
    pub sqlite_version: Option<String>,
    /// DB migration version.
    pub migration_version: Option<i32>,
    /// Configured URL base path.
    pub url_base: Option<String>,
    /// .NET runtime version.
    pub runtime_version: Option<String>,
    /// Runtime name.
    pub runtime_name: Option<String>,
    /// Process start date-time (ISO-8601).
    pub start_time: Option<String>,
    /// Package version.
    pub package_version: Option<String>,
    /// Package author.
    pub package_author: Option<String>,
    /// Update mechanism enum.
    pub package_update_mechanism: Option<UpdateMechanism>,
    /// Update mechanism note.
    pub package_update_mechanism_message: Option<String>,
    /// Database version string.
    pub database_version: Option<String>,
    /// Database type enum (sqLite/postgreSQL).
    pub database_type: Option<DatabaseType>,
}

// ───────────────────────────── Releases ─────────────────────────────

/// A search/lookup release result (`GET`/`POST /api/v3/release`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseResource {
    /// Release id (0 for lookup results).
    pub id: Option<i32>,
    /// Release GUID.
    pub guid: Option<String>,
    /// Parsed quality model.
    pub quality: Option<QualityModel>,
    /// Quality weight for ranking.
    pub quality_weight: Option<i32>,
    /// Age in days.
    pub age: Option<i32>,
    /// Age in hours (double).
    pub age_hours: Option<f64>,
    /// Age in minutes (double).
    pub age_minutes: Option<f64>,
    /// Size in bytes (int64).
    pub size: Option<i64>,
    /// Indexer id.
    pub indexer_id: Option<i32>,
    /// Indexer name.
    pub indexer: Option<String>,
    /// Release group.
    pub release_group: Option<String>,
    /// Anime sub group.
    pub sub_group: Option<String>,
    /// Release hash.
    pub release_hash: Option<String>,
    /// Release title.
    pub title: Option<String>,
    /// Is a full-season pack.
    pub full_season: Option<bool>,
    /// From a scene source.
    pub scene_source: Option<bool>,
    /// Season number.
    pub season_number: Option<i32>,
    /// Languages.
    #[serde(default)]
    pub languages: Vec<Language>,
    /// Language weight for ranking.
    pub language_weight: Option<i32>,
    /// Air date (daily series).
    pub air_date: Option<String>,
    /// Parsed series title.
    pub series_title: Option<String>,
    /// Parsed episode numbers.
    #[serde(default)]
    pub episode_numbers: Vec<i32>,
    /// Parsed absolute episode numbers.
    #[serde(default)]
    pub absolute_episode_numbers: Vec<i32>,
    /// Mapped season number.
    pub mapped_season_number: Option<i32>,
    /// Mapped episode numbers.
    #[serde(default)]
    pub mapped_episode_numbers: Vec<i32>,
    /// Mapped absolute episode numbers.
    #[serde(default)]
    pub mapped_absolute_episode_numbers: Vec<i32>,
    /// Resolved series id.
    pub mapped_series_id: Option<i32>,
    /// Resolved episode info list.
    #[serde(default)]
    pub mapped_episode_info: Vec<ReleaseEpisodeResource>,
    /// Passed all decision rules.
    pub approved: Option<bool>,
    /// Temporarily rejected (retry later).
    pub temporarily_rejected: Option<bool>,
    /// Permanently rejected.
    pub rejected: Option<bool>,
    /// TheTVDB id.
    pub tvdb_id: Option<i32>,
    /// TVRage id.
    pub tv_rage_id: Option<i32>,
    /// IMDb id.
    pub imdb_id: Option<String>,
    /// Rejection reason strings.
    #[serde(default)]
    pub rejections: Vec<String>,
    /// Release publish date-time (ISO-8601).
    pub publish_date: Option<String>,
    /// Comments URL.
    pub comment_url: Option<String>,
    /// Download URL.
    pub download_url: Option<String>,
    /// Info/details URL.
    pub info_url: Option<String>,
    /// Whether the episode is wanted.
    pub episode_requested: Option<bool>,
    /// Whether grab is allowed.
    pub download_allowed: Option<bool>,
    /// Overall ranking weight.
    pub release_weight: Option<i32>,
    /// Matched custom formats.
    #[serde(default)]
    pub custom_formats: Vec<CustomFormatResource>,
    /// Custom format score.
    pub custom_format_score: Option<i32>,
    /// Scene mapping (AlternateTitleResource).
    pub scene_mapping: Option<AlternateTitleResource>,
    /// Torrent magnet URL.
    pub magnet_url: Option<String>,
    /// Torrent info hash.
    pub info_hash: Option<String>,
    /// Torrent seeders.
    pub seeders: Option<i32>,
    /// Torrent leechers.
    pub leechers: Option<i32>,
    /// Protocol enum (usenet/torrent).
    pub protocol: Option<DownloadProtocol>,
    /// Indexer flag bitmask.
    pub indexer_flags: Option<i32>,
    /// Daily-series release.
    pub is_daily: Option<bool>,
    /// Uses absolute numbering.
    pub is_absolute_numbering: Option<bool>,
    /// Might be a special.
    pub is_possible_special_episode: Option<bool>,
    /// Is a special episode.
    pub special: Option<bool>,
    /// Series id (manual grab override).
    pub series_id: Option<i32>,
    /// Episode id (manual grab override).
    pub episode_id: Option<i32>,
    /// Episode ids (manual grab override).
    #[serde(default)]
    pub episode_ids: Vec<i32>,
    /// Download client id (manual grab override).
    pub download_client_id: Option<i32>,
    /// Download client name.
    pub download_client: Option<String>,
    /// Force manual override on grab.
    pub should_override: Option<bool>,
}

/// Episode info resolved for a release (nested in
/// [`ReleaseResource::mapped_episode_info`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseEpisodeResource {
    /// Episode id.
    pub id: Option<i32>,
    /// Season number.
    pub season_number: Option<i32>,
    /// Episode number.
    pub episode_number: Option<i32>,
    /// Absolute episode number.
    pub absolute_episode_number: Option<i32>,
    /// Episode title.
    pub title: Option<String>,
}

// ───────────────────────────── Custom formats & languages ─────────────────────────────

/// A custom format definition (nested + `GET /api/v3/customformat`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomFormatResource {
    /// Custom format id.
    pub id: Option<i32>,
    /// Custom format name.
    pub name: Option<String>,
    /// Include token in renames.
    pub include_custom_format_when_renaming: Option<bool>,
    /// Matching specifications. Untyped — free-form objects (Field.value untyped).
    #[serde(default)]
    pub specifications: Vec<serde_json::Value>,
}

/// A language reference (`{id,name}`). Inferred shape — used across many
/// resources (nested).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    /// Language id.
    pub id: Option<i32>,
    /// Language name.
    pub name: Option<String>,
}

// ───────────────────────────── Enums ─────────────────────────────

/// Series status. Serialized lowercase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SeriesStatusType {
    Continuing,
    Ended,
    Upcoming,
    Deleted,
}

/// Series type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SeriesTypes {
    Standard,
    Daily,
    Anime,
}

/// Image cover type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Quality source. NOTE mixed-case values (televisionRaw, webRip, blurayRaw) —
/// serde camelCase matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QualitySource {
    Unknown,
    Television,
    TelevisionRaw,
    Web,
    WebRip,
    Dvd,
    Bluray,
    BlurayRaw,
}

/// Queue item status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Tracked download status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrackedDownloadStatus {
    Ok,
    Warning,
    Error,
}

/// Tracked download state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// History event type (Sonarr-specific). Drives which keys appear in
/// [`HistoryResource::data`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EpisodeHistoryEventType {
    Unknown,
    Grabbed,
    SeriesFolderImported,
    DownloadFolderImported,
    DownloadFailed,
    EpisodeFileDeleted,
    EpisodeFileRenamed,
    DownloadIgnored,
}

/// Paging sort direction. Used as `sortDirection` in all `*PagingResource`
/// wrappers — it's an ENUM not a string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    Default,
    Ascending,
    Descending,
}

/// Command priority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CommandPriority {
    Normal,
    High,
    Low,
}

/// Command status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Command result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CommandResult {
    Unknown,
    Successful,
    Unsuccessful,
}

/// Health severity. Used as [`HealthResource::kind`] (the `type` key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HealthCheckResult {
    Ok,
    Notice,
    Warning,
    Error,
}

/// Process runtime mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeMode {
    Console,
    Service,
    Tray,
}

/// Release type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseType {
    Unknown,
    SingleEpisode,
    MultiEpisode,
    SeasonPack,
}

/// [`AddSeriesOptions::monitor`] policy (representative values; verify against
/// your Sonarr version).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MonitorTypes {
    Unknown,
    All,
    Future,
    Missing,
    Existing,
    FirstSeason,
    LastSeason,
    LatestSeason,
    Pilot,
    Recent,
    MonitorSpecials,
    UnmonitorSpecials,
    None,
    Skip,
}

/// [`SeriesResource::monitor_new_items`] policy (representative — confirm enum
/// members in your version).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NewItemMonitorTypes {
    All,
    None,
}

/// Download protocol. Referenced by [`QueueResource`] and [`ReleaseResource`];
/// the OpenAPI doc declares it `{unknown,usenet,torrent}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DownloadProtocol {
    Unknown,
    Usenet,
    Torrent,
}

/// What triggered a command. Referenced by [`CommandResource`] and [`Command`];
/// the OpenAPI doc declares it `{unspecified,manual,scheduled}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CommandTrigger {
    Unspecified,
    Manual,
    Scheduled,
}

/// Authentication type ([`SystemResource::authentication`]); the OpenAPI doc
/// declares it `{none,basic,forms,external}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AuthenticationType {
    None,
    Basic,
    Forms,
    External,
}

/// Update mechanism ([`SystemResource::package_update_mechanism`]); the OpenAPI
/// doc declares it `{builtIn,script,external,apt,docker}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UpdateMechanism {
    BuiltIn,
    Script,
    External,
    Apt,
    Docker,
}

/// Database type ([`SystemResource::database_type`]); the OpenAPI doc declares it
/// `{sqLite,postgreSQL}` (note the mixed casing serde camelCase preserves).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseType {
    #[serde(rename = "sqLite")]
    SqLite,
    #[serde(rename = "postgreSQL")]
    PostgreSql,
}

#[cfg(test)]
#[path = "sonarr_tests.rs"]
mod tests;
