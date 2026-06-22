//! MediaServer models — Jellyfin (`/Items`, `/Users`, `/Sessions`, `/System`).
//!
//! Jellyfin serialises every property as PascalCase via System.Text.Json, so
//! each struct carries `#[serde(rename_all = "PascalCase")]`. Field selection
//! follows the C# DTOs (`BaseItemDto`, `UserDto`, `SessionInfoDto`, …) read from
//! the upstream OpenAPI document; the app layer reads a subset, but the models
//! capture the documented shape so a JSON Schema can be emitted per response.
//!
//! Jellyfin quirks worth knowing:
//!   * **`Guid` → JSON string.** Every `*Id` is a UUID *string* on the wire, not
//!     a number, so all id fields are `String` / `Option<String>`.
//!   * **`*Ticks` are .NET ticks, not epoch.** `RunTimeTicks`,
//!     `PositionTicks`, `PlaybackPositionTicks`, etc. are `long` integers of
//!     100-nanosecond ticks (10,000,000 per second) — kept as `i64`, never
//!     treated as Unix time.
//!   * **`Type` is reserved.** `BaseItemDto.Type` (wire key `"Type"`) is renamed
//!     to `kind` and is always present.
//!   * **`#nullable disable` reference types** (strings, arrays, nested objects)
//!     are emitted as `"nullable": true` even when unmarked, so `Option` is the
//!     safe default; only non-nullable C# value types are bare.
//!   * Heavily-nested maps the app layer never destructures (`ImageBlurHashes`,
//!     `Trickplay`, `LibraryOptions`, `Capabilities`, `TranscodingInfo`, …) are
//!     kept as `serde_json::Value` rather than modelled in full.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The kind of a [`BaseItemDto`] (`BaseItemKind` enum, e.g. `Movie`, `Series`,
/// `Episode`, `Season`, `MusicAlbum`). Modelled as an open string enum with a
/// catch-all so unknown future kinds deserialize cleanly rather than failing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BaseItemKind {
    /// Any documented or future `BaseItemKind` value, carried verbatim.
    Known(String),
}

impl Default for BaseItemKind {
    fn default() -> Self {
        BaseItemKind::Known(String::new())
    }
}

/// The media type of a [`BaseItemDto`] (`MediaType` enum: `Unknown`, `Video`,
/// `Audio`, `Photo`, `Book`). Non-nullable on the wire (defaults to `Unknown`);
/// modelled as an open string enum so unknown values still decode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MediaType {
    /// Any documented or future `MediaType` value, carried verbatim.
    Known(String),
}

impl Default for MediaType {
    fn default() -> Self {
        MediaType::Known("Unknown".to_string())
    }
}

/// An external provider link on an item (`BaseItemDto.ExternalUrls[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ExternalUrl {
    /// Provider name (e.g. `"IMDb"`).
    pub name: Option<String>,
    /// Link URL.
    pub url: Option<String>,
}

/// A remote trailer URL (`BaseItemDto.RemoteTrailers[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct MediaUrl {
    /// Trailer URL.
    pub url: Option<String>,
    /// Display name.
    pub name: Option<String>,
}

/// A name + Guid pair used for studios, genres, and artists
/// (`BaseItemDto.Studios[]`, `GenreItems[]`, `ArtistItems[]`, `AlbumArtists[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct NameGuidPair {
    /// Display name.
    pub name: Option<String>,
    /// Item Guid serialised as a UUID string.
    pub id: Option<String>,
}

/// A cast/crew entry (`BaseItemDto.People[]`), slimmed to the identity fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct BaseItemPerson {
    /// Person display name.
    pub name: Option<String>,
    /// Person Guid serialised as a UUID string.
    pub id: Option<String>,
    /// Role text (e.g. character name).
    pub role: Option<String>,
    /// Person type enum (Actor, Director, Writer, …).
    #[serde(rename = "Type")]
    pub kind: Option<String>,
    /// Primary image tag for the person.
    pub primary_image_tag: Option<String>,
}

/// A chapter marker (`BaseItemDto.Chapters[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ChapterInfo {
    /// Chapter start position in .NET ticks (100ns units).
    pub start_position_ticks: Option<i64>,
    /// Chapter name.
    pub name: Option<String>,
    /// Primary image tag for the chapter.
    pub image_tag: Option<String>,
}

/// An audio/video/subtitle stream descriptor (`BaseItemDto.MediaStreams[]`),
/// slimmed to the broadly-useful identity fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct MediaStream {
    /// Codec name (e.g. `h264`, `aac`).
    pub codec: Option<String>,
    /// Stream language.
    pub language: Option<String>,
    /// Stream type enum (Video, Audio, Subtitle, …).
    #[serde(rename = "Type")]
    pub kind: Option<String>,
    /// Human display title.
    pub display_title: Option<String>,
    /// Zero-based stream index.
    pub index: Option<i32>,
    /// Whether this is the default stream.
    pub is_default: Option<bool>,
    /// Whether the stream is external (e.g. sidecar subtitle).
    pub is_external: Option<bool>,
    /// Pixel width (video).
    pub width: Option<i32>,
    /// Pixel height (video).
    pub height: Option<i32>,
}

/// An available media source / version (`BaseItemDto.MediaSources[]`), slimmed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct MediaSourceInfo {
    /// Media source id (string, not a Guid).
    pub id: Option<String>,
    /// Filesystem path on the server.
    pub path: Option<String>,
    /// Container format.
    pub container: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Total size in bytes.
    pub size: Option<i64>,
    /// Runtime in .NET ticks (100ns units).
    pub run_time_ticks: Option<i64>,
    /// Stream descriptors for this source.
    #[serde(default)]
    pub media_streams: Vec<MediaStream>,
}

/// Per-user playback/favorite state for an item (`BaseItemDto.UserData`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct UserItemDataDto {
    /// User rating (double).
    pub rating: Option<f64>,
    /// Played percentage (0-100).
    pub played_percentage: Option<f64>,
    /// Count of unplayed child items.
    pub unplayed_item_count: Option<i32>,
    /// Resume position in .NET ticks (100ns units); non-nullable.
    pub playback_position_ticks: i64,
    /// Total play count; non-nullable.
    pub play_count: i32,
    /// Whether marked favorite; non-nullable.
    pub is_favorite: bool,
    /// Thumbs up/down (`null` = unset).
    pub likes: Option<bool>,
    /// Last played date (ISO-8601).
    pub last_played_date: Option<String>,
    /// Whether item is marked played; non-nullable.
    pub played: bool,
    /// User-data key; C# `required`, always present.
    pub key: String,
    /// Item Guid (UUID string); non-nullable value type.
    pub item_id: String,
}

/// A media item (movie, series, episode, season, album, …) in the
/// client-friendly shape Jellyfin returns from `GET /Items`,
/// `GET /Users/{userId}/Items/{itemId}`, and `GET /Items/{itemId}`.
///
/// `Type` (reserved word) is renamed to `kind`; `CurrentProgram` is `Box`ed
/// because it is itself a `BaseItemDto`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct BaseItemDto {
    /// Display name of the item.
    pub name: Option<String>,
    /// Original (untranslated) title.
    pub original_title: Option<String>,
    /// Identifier of the server that owns this item.
    pub server_id: Option<String>,
    /// Item Guid serialised as a UUID string.
    pub id: Option<String>,
    /// ETag for caching.
    pub etag: Option<String>,
    /// Source type of the item.
    pub source_type: Option<String>,
    /// Playlist item identifier.
    pub playlist_item_id: Option<String>,
    /// Creation date (ISO-8601).
    pub date_created: Option<String>,
    /// Date media was last added (ISO-8601).
    pub date_last_media_added: Option<String>,
    /// Extra type enum (e.g. Trailer, BehindTheScenes).
    pub extra_type: Option<String>,
    /// Episode airs-before season number.
    pub airs_before_season_number: Option<i32>,
    /// Episode airs-after season number.
    pub airs_after_season_number: Option<i32>,
    /// Episode airs-before episode number.
    pub airs_before_episode_number: Option<i32>,
    /// Whether the requesting user may delete this item.
    pub can_delete: Option<bool>,
    /// Whether the item can be downloaded.
    pub can_download: Option<bool>,
    /// Whether lyrics are available.
    pub has_lyrics: Option<bool>,
    /// Whether subtitles are available.
    pub has_subtitles: Option<bool>,
    /// Preferred metadata language.
    pub preferred_metadata_language: Option<String>,
    /// Preferred metadata country code.
    pub preferred_metadata_country_code: Option<String>,
    /// Media container format.
    pub container: Option<String>,
    /// Computed sort name.
    pub sort_name: Option<String>,
    /// Manually forced sort name.
    pub forced_sort_name: Option<String>,
    /// 3D video format enum.
    #[serde(rename = "Video3DFormat")]
    pub video3_d_format: Option<String>,
    /// Premiere/release date (ISO-8601).
    pub premiere_date: Option<String>,
    /// External provider links.
    #[serde(default)]
    pub external_urls: Vec<ExternalUrl>,
    /// Available media source/version descriptors.
    #[serde(default)]
    pub media_sources: Vec<MediaSourceInfo>,
    /// Critic rating (float).
    pub critic_rating: Option<f32>,
    /// Production location names.
    #[serde(default)]
    pub production_locations: Vec<String>,
    /// Filesystem path on the server.
    pub path: Option<String>,
    /// Whether to display media sources.
    pub enable_media_source_display: Option<bool>,
    /// Official content rating (e.g. PG-13).
    pub official_rating: Option<String>,
    /// Custom content rating.
    pub custom_rating: Option<String>,
    /// Live TV channel Guid (UUID string).
    pub channel_id: Option<String>,
    /// Live TV channel name.
    pub channel_name: Option<String>,
    /// Plot/overview text.
    pub overview: Option<String>,
    /// Taglines.
    #[serde(default)]
    pub taglines: Vec<String>,
    /// Genre names.
    #[serde(default)]
    pub genres: Vec<String>,
    /// Community rating (float).
    pub community_rating: Option<f32>,
    /// Cumulative runtime in .NET ticks (100ns units).
    pub cumulative_run_time_ticks: Option<i64>,
    /// Runtime in .NET ticks (100ns units); divide by 10,000,000 for seconds.
    pub run_time_ticks: Option<i64>,
    /// Play access enum (Full/None).
    pub play_access: Option<String>,
    /// Aspect ratio string.
    pub aspect_ratio: Option<String>,
    /// Production year.
    pub production_year: Option<i32>,
    /// Whether this is a placeholder item.
    pub is_place_holder: Option<bool>,
    /// Number string (e.g. channel number).
    pub number: Option<String>,
    /// Live TV channel number.
    pub channel_number: Option<String>,
    /// Index within parent (e.g. episode number).
    pub index_number: Option<i32>,
    /// End index for multi-episode items.
    pub index_number_end: Option<i32>,
    /// Parent index (e.g. season number).
    pub parent_index_number: Option<i32>,
    /// Remote trailer URLs.
    #[serde(default)]
    pub remote_trailers: Vec<MediaUrl>,
    /// External provider id map (e.g. Imdb, Tmdb -> id).
    pub provider_ids: Option<HashMap<String, String>>,
    /// Whether the item is HD.
    #[serde(rename = "IsHD")]
    pub is_hd: Option<bool>,
    /// Whether this item is a folder/container.
    pub is_folder: Option<bool>,
    /// Parent item Guid (UUID string).
    pub parent_id: Option<String>,
    /// Item kind enum (BaseItemKind). Reserved word `Type` renamed to `kind`;
    /// always present.
    #[serde(rename = "Type", default)]
    pub kind: BaseItemKind,
    /// Cast and crew.
    #[serde(default)]
    pub people: Vec<BaseItemPerson>,
    /// Studios (name + Guid).
    #[serde(default)]
    pub studios: Vec<NameGuidPair>,
    /// Genres as name+Guid pairs.
    #[serde(default)]
    pub genre_items: Vec<NameGuidPair>,
    /// Guid of ancestor holding the logo image.
    pub parent_logo_item_id: Option<String>,
    /// Guid of ancestor holding backdrops.
    pub parent_backdrop_item_id: Option<String>,
    /// Backdrop image tags from the parent.
    #[serde(default)]
    pub parent_backdrop_image_tags: Vec<String>,
    /// Count of local trailers.
    pub local_trailer_count: Option<i32>,
    /// Per-user playback/favorite state for this item.
    pub user_data: Option<UserItemDataDto>,
    /// Recursive count of descendant items.
    pub recursive_item_count: Option<i32>,
    /// Direct child count.
    pub child_count: Option<i32>,
    /// Name of the parent series.
    pub series_name: Option<String>,
    /// Parent series Guid (UUID string).
    pub series_id: Option<String>,
    /// Parent season Guid (UUID string).
    pub season_id: Option<String>,
    /// Special feature count.
    pub special_feature_count: Option<i32>,
    /// Display preferences identifier.
    pub display_preferences_id: Option<String>,
    /// Series status (e.g. Continuing/Ended).
    pub status: Option<String>,
    /// Broadcast air time.
    pub air_time: Option<String>,
    /// Broadcast air days (DayOfWeek enum values).
    #[serde(default)]
    pub air_days: Vec<String>,
    /// Free-form tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Primary image aspect ratio (double).
    pub primary_image_aspect_ratio: Option<f64>,
    /// Artist names (music).
    #[serde(default)]
    pub artists: Vec<String>,
    /// Artists as name+Guid pairs.
    #[serde(default)]
    pub artist_items: Vec<NameGuidPair>,
    /// Album name.
    pub album: Option<String>,
    /// Collection type enum (movies, tvshows, music, …).
    pub collection_type: Option<String>,
    /// Display order setting.
    pub display_order: Option<String>,
    /// Album Guid (UUID string).
    pub album_id: Option<String>,
    /// Album primary image tag.
    pub album_primary_image_tag: Option<String>,
    /// Series primary image tag.
    pub series_primary_image_tag: Option<String>,
    /// Album artist name.
    pub album_artist: Option<String>,
    /// Album artists as name+Guid pairs.
    #[serde(default)]
    pub album_artists: Vec<NameGuidPair>,
    /// Parent season name.
    pub season_name: Option<String>,
    /// Audio/video/subtitle stream descriptors.
    #[serde(default)]
    pub media_streams: Vec<MediaStream>,
    /// Video type enum (VideoFile, Dvd, BluRay, …).
    pub video_type: Option<String>,
    /// Number of parts.
    pub part_count: Option<i32>,
    /// Number of media sources.
    pub media_source_count: Option<i32>,
    /// Map of ImageType -> image tag string.
    pub image_tags: Option<HashMap<String, String>>,
    /// Backdrop image tags.
    #[serde(default)]
    pub backdrop_image_tags: Vec<String>,
    /// Screenshot image tags.
    #[serde(default)]
    pub screenshot_image_tags: Vec<String>,
    /// Parent logo image tag.
    pub parent_logo_image_tag: Option<String>,
    /// Guid of ancestor holding art image.
    pub parent_art_item_id: Option<String>,
    /// Parent art image tag.
    pub parent_art_image_tag: Option<String>,
    /// Series thumbnail image tag.
    pub series_thumb_image_tag: Option<String>,
    /// Nested map ImageType -> (image tag -> blurhash string).
    pub image_blur_hashes: Option<serde_json::Value>,
    /// Series studio name.
    pub series_studio: Option<String>,
    /// Guid of ancestor holding thumb image.
    pub parent_thumb_item_id: Option<String>,
    /// Parent thumb image tag.
    pub parent_thumb_image_tag: Option<String>,
    /// Guid of ancestor holding primary image.
    pub parent_primary_image_item_id: Option<String>,
    /// Parent primary image tag.
    pub parent_primary_image_tag: Option<String>,
    /// Chapter markers.
    #[serde(default)]
    pub chapters: Vec<ChapterInfo>,
    /// Trickplay manifest: nested map of mediaSourceId -> (width -> TrickplayInfo).
    pub trickplay: Option<serde_json::Value>,
    /// Location type enum (FileSystem, Remote, Virtual, …).
    pub location_type: Option<String>,
    /// ISO type enum (Dvd, BluRay).
    pub iso_type: Option<String>,
    /// Media type enum (Unknown, Video, Audio, Photo, Book); non-nullable,
    /// defaults to Unknown.
    #[serde(default)]
    pub media_type: MediaType,
    /// End date (ISO-8601).
    pub end_date: Option<String>,
    /// Metadata fields locked from auto-update (MetadataField enum).
    #[serde(default)]
    pub locked_fields: Vec<String>,
    /// Trailer count.
    pub trailer_count: Option<i32>,
    /// Movie count (folders).
    pub movie_count: Option<i32>,
    /// Series count (folders).
    pub series_count: Option<i32>,
    /// Program count.
    pub program_count: Option<i32>,
    /// Episode count.
    pub episode_count: Option<i32>,
    /// Song count.
    pub song_count: Option<i32>,
    /// Album count.
    pub album_count: Option<i32>,
    /// Artist count.
    pub artist_count: Option<i32>,
    /// Music video count.
    pub music_video_count: Option<i32>,
    /// Whether metadata is locked from internet providers.
    pub lock_data: Option<bool>,
    /// Pixel width.
    pub width: Option<i32>,
    /// Pixel height.
    pub height: Option<i32>,
    /// Photo camera make (EXIF).
    pub camera_make: Option<String>,
    /// Photo camera model (EXIF).
    pub camera_model: Option<String>,
    /// Photo software (EXIF).
    pub software: Option<String>,
    /// Photo exposure time (EXIF).
    pub exposure_time: Option<f64>,
    /// Photo focal length (EXIF).
    pub focal_length: Option<f64>,
    /// Image orientation enum (EXIF).
    pub image_orientation: Option<String>,
    /// Photo aperture (EXIF).
    pub aperture: Option<f64>,
    /// Photo shutter speed (EXIF).
    pub shutter_speed: Option<f64>,
    /// Photo GPS latitude (EXIF).
    pub latitude: Option<f64>,
    /// Photo GPS longitude (EXIF).
    pub longitude: Option<f64>,
    /// Photo GPS altitude (EXIF).
    pub altitude: Option<f64>,
    /// Photo ISO speed rating (EXIF).
    pub iso_speed_rating: Option<i32>,
    /// Live TV series timer id.
    pub series_timer_id: Option<String>,
    /// Live TV program id.
    pub program_id: Option<String>,
    /// Channel primary image tag.
    pub channel_primary_image_tag: Option<String>,
    /// Recording start date in UTC (ISO-8601).
    pub start_date: Option<String>,
    /// Completion percentage.
    pub completion_percentage: Option<f64>,
    /// Whether the broadcast is a repeat.
    pub is_repeat: Option<bool>,
    /// Episode title.
    pub episode_title: Option<String>,
    /// Channel type enum (TV, Radio).
    pub channel_type: Option<String>,
    /// Program audio enum (Mono, Stereo, …).
    pub audio: Option<String>,
    /// Whether program is a movie.
    pub is_movie: Option<bool>,
    /// Whether program is sports.
    pub is_sports: Option<bool>,
    /// Whether program is a series.
    pub is_series: Option<bool>,
    /// Whether program is live.
    pub is_live: Option<bool>,
    /// Whether program is news.
    pub is_news: Option<bool>,
    /// Whether program is kids content.
    pub is_kids: Option<bool>,
    /// Whether program is a premiere.
    pub is_premiere: Option<bool>,
    /// Live TV timer id.
    pub timer_id: Option<String>,
    /// Audio normalization gain.
    pub normalization_gain: Option<f32>,
    /// Album-inherited audio normalization gain.
    pub album_normalization_gain: Option<f32>,
    /// Current live TV program (recursive `BaseItemDto`; `Box` for recursion).
    pub current_program: Option<Box<BaseItemDto>>,
    /// Original language.
    pub original_language: Option<String>,
}

/// A Jellyfin user account, from `GET /Users`, `GET /Users/{userId}`, and
/// `GET /Users/Me`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct UserDto {
    /// User display name.
    pub name: Option<String>,
    /// Owning server identifier.
    pub server_id: Option<String>,
    /// Server name (client-side only, not set by server).
    pub server_name: Option<String>,
    /// User Guid (UUID string).
    pub id: Option<String>,
    /// Primary (avatar) image tag.
    pub primary_image_tag: Option<String>,
    /// Deprecated/obsolete: always true; no longer provided meaningfully.
    pub has_password: Option<bool>,
    /// Deprecated/obsolete: always true.
    pub has_configured_password: Option<bool>,
    /// Deprecated: easy password replaced by Quick Connect.
    pub has_configured_easy_password: Option<bool>,
    /// Whether auto-login is enabled.
    pub enable_auto_login: Option<bool>,
    /// Last login date (ISO-8601).
    pub last_login_date: Option<String>,
    /// Last activity date (ISO-8601).
    pub last_activity_date: Option<String>,
    /// User configuration object (UserConfiguration). Always constructed
    /// server-side; schema nullable.
    pub configuration: Option<serde_json::Value>,
    /// User policy / permissions object. Always constructed server-side; schema
    /// nullable.
    pub policy: Option<UserPolicy>,
    /// Primary image aspect ratio.
    pub primary_image_aspect_ratio: Option<f64>,
}

/// User permission/policy settings (`UserDto.Policy`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct UserPolicy {
    /// Whether the user is an administrator; non-nullable.
    pub is_administrator: bool,
    /// Whether the user is hidden from login screens.
    pub is_hidden: bool,
    /// Whether the user can manage collections.
    pub enable_collection_management: bool,
    /// Whether the user can manage subtitles.
    pub enable_subtitle_management: bool,
    /// Whether the user can manage lyrics.
    pub enable_lyric_management: bool,
    /// Whether the account is disabled.
    pub is_disabled: bool,
    /// Maximum allowed parental rating.
    pub max_parental_rating: Option<i32>,
    /// Maximum allowed parental sub-rating.
    pub max_parental_sub_rating: Option<i32>,
    /// Tags blocked for this user.
    #[serde(default)]
    pub blocked_tags: Vec<String>,
    /// Tags explicitly allowed for this user.
    #[serde(default)]
    pub allowed_tags: Vec<String>,
    /// Whether the user can change their preferences.
    pub enable_user_preference_access: bool,
    /// Time-of-day access schedules (AccessSchedule).
    #[serde(default)]
    pub access_schedules: Vec<serde_json::Value>,
    /// Unrated item categories to block (UnratedItem enum).
    #[serde(default)]
    pub block_unrated_items: Vec<String>,
    /// Whether the user can remote-control other users' sessions.
    pub enable_remote_control_of_other_users: bool,
    /// Whether the user can control shared devices.
    pub enable_shared_device_control: bool,
    /// Whether the user can connect from outside the LAN.
    pub enable_remote_access: bool,
    /// Whether the user can manage Live TV.
    pub enable_live_tv_management: bool,
    /// Whether the user can access Live TV.
    pub enable_live_tv_access: bool,
    /// Whether media playback is allowed.
    pub enable_media_playback: bool,
    /// Whether audio transcoding is allowed.
    pub enable_audio_playback_transcoding: bool,
    /// Whether video transcoding is allowed.
    pub enable_video_playback_transcoding: bool,
    /// Whether remuxing is allowed.
    pub enable_playback_remuxing: bool,
    /// Whether remote sources are forced to transcode.
    pub force_remote_source_transcoding: bool,
    /// Whether the user can delete content.
    pub enable_content_deletion: bool,
    /// Folders the user may delete from.
    #[serde(default)]
    pub enable_content_deletion_from_folders: Vec<String>,
    /// Whether the user can download content.
    pub enable_content_downloading: bool,
    /// Whether sync transcoding is allowed.
    pub enable_sync_transcoding: bool,
    /// Whether media conversion is allowed.
    pub enable_media_conversion: bool,
    /// Explicitly enabled device ids.
    #[serde(default)]
    pub enabled_devices: Vec<String>,
    /// Whether all devices are enabled.
    pub enable_all_devices: bool,
    /// Enabled channel Guids (UUID strings).
    #[serde(default)]
    pub enabled_channels: Vec<String>,
    /// Whether all channels are enabled.
    pub enable_all_channels: bool,
    /// Enabled folder Guids (UUID strings).
    #[serde(default)]
    pub enabled_folders: Vec<String>,
    /// Whether all folders are enabled.
    pub enable_all_folders: bool,
    /// Current count of invalid login attempts.
    pub invalid_login_attempt_count: i32,
    /// Allowed invalid attempts before lockout (-1 = unlimited).
    pub login_attempts_before_lockout: i32,
    /// Max concurrent sessions (0 = unlimited).
    pub max_active_sessions: i32,
    /// Whether public sharing is allowed.
    pub enable_public_sharing: bool,
    /// Blocked media folder Guids (UUID strings).
    #[serde(default)]
    pub blocked_media_folders: Vec<String>,
    /// Blocked channel Guids (UUID strings).
    #[serde(default)]
    pub blocked_channels: Vec<String>,
    /// Remote client bitrate limit (bps).
    pub remote_client_bitrate_limit: i32,
    /// Authentication provider id (required, non-empty in C#, reference type).
    pub authentication_provider_id: Option<String>,
    /// Password reset provider id.
    pub password_reset_provider_id: Option<String>,
    /// SyncPlay access level enum (CreateAndJoinGroups, JoinGroups, None);
    /// non-nullable.
    pub sync_play_access: String,
}

/// Playback state of a session (`SessionInfoDto.PlayState`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerStateInfo {
    /// Current playback position in .NET ticks (100ns units).
    pub position_ticks: Option<i64>,
    /// Whether the player can seek; non-nullable.
    pub can_seek: bool,
    /// Whether playback is paused; non-nullable.
    pub is_paused: bool,
    /// Whether audio is muted; non-nullable.
    pub is_muted: bool,
    /// Volume level (0-100).
    pub volume_level: Option<i32>,
    /// Index of the active audio stream.
    pub audio_stream_index: Option<i32>,
    /// Index of the active subtitle stream.
    pub subtitle_stream_index: Option<i32>,
    /// Currently playing media source id.
    pub media_source_id: Option<String>,
    /// Play method enum (DirectPlay, DirectStream, Transcode).
    pub play_method: Option<String>,
    /// Repeat mode enum (RepeatNone, RepeatAll, RepeatOne); non-nullable.
    pub repeat_mode: String,
    /// Playback order enum (Default, Shuffle); non-nullable.
    pub playback_order: String,
    /// Live stream id for the now-playing item.
    pub live_stream_id: Option<String>,
}

/// An active client session (`GET /Sessions`): playback device, now-playing
/// item, and play state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct SessionInfoDto {
    /// Current playback state.
    pub play_state: Option<PlayerStateInfo>,
    /// Additional users attached to the session (SessionUserInfo).
    #[serde(default)]
    pub additional_users: Vec<serde_json::Value>,
    /// Client capabilities (ClientCapabilitiesDto).
    pub capabilities: Option<serde_json::Value>,
    /// Remote endpoint / client IP.
    pub remote_end_point: Option<String>,
    /// Media types the client can play (MediaType enum); defaults to empty array,
    /// always present.
    #[serde(default)]
    pub playable_media_types: Vec<String>,
    /// Session identifier (string, not a Guid).
    pub id: Option<String>,
    /// User Guid (UUID string); non-nullable value type, always present.
    pub user_id: String,
    /// Username of the session owner.
    pub user_name: Option<String>,
    /// Client application name.
    pub client: Option<String>,
    /// Last activity date (ISO-8601); non-nullable DateTime.
    pub last_activity_date: String,
    /// Last playback check-in date (ISO-8601); non-nullable.
    pub last_playback_check_in: String,
    /// Last paused date (ISO-8601).
    pub last_paused_date: Option<String>,
    /// Friendly device name.
    pub device_name: Option<String>,
    /// Device type.
    pub device_type: Option<String>,
    /// Item currently playing in the session.
    pub now_playing_item: Option<BaseItemDto>,
    /// Item currently being viewed (browsing) in the session.
    pub now_viewing_item: Option<BaseItemDto>,
    /// Device identifier.
    pub device_id: Option<String>,
    /// Client application version.
    pub application_version: Option<String>,
    /// Active transcoding info (TranscodingInfo).
    pub transcoding_info: Option<serde_json::Value>,
    /// Whether the session is currently active; non-nullable.
    pub is_active: bool,
    /// Whether the session supports media control; non-nullable.
    pub supports_media_control: bool,
    /// Whether the session supports remote control; non-nullable.
    pub supports_remote_control: bool,
    /// Now-playing queue (QueueItem).
    #[serde(default)]
    pub now_playing_queue: Vec<serde_json::Value>,
    /// Whether the device name is user-customized; non-nullable.
    pub has_custom_device_name: bool,
    /// Current playlist item id.
    pub playlist_item_id: Option<String>,
    /// Server id.
    pub server_id: Option<String>,
    /// User avatar image tag.
    pub user_primary_image_tag: Option<String>,
    /// Supported general commands (GeneralCommandType enum); defaults to empty
    /// array, always present.
    #[serde(default)]
    pub supported_commands: Vec<String>,
}

/// A configured library (virtual folder) with its physical locations
/// (`GET /Library/VirtualFolders`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct VirtualFolderInfo {
    /// Library display name.
    pub name: Option<String>,
    /// Physical filesystem paths backing the library; defaults to empty array,
    /// always present.
    #[serde(default)]
    pub locations: Vec<String>,
    /// Collection type enum (movies, tvshows, music, …) — CollectionTypeOptions.
    pub collection_type: Option<String>,
    /// Library scanning/metadata options (LibraryOptions).
    pub library_options: Option<serde_json::Value>,
    /// Library folder item id (string form of the Guid).
    pub item_id: Option<String>,
    /// Item id providing the primary image.
    pub primary_image_item_id: Option<String>,
    /// Library scan progress (0-100).
    pub refresh_progress: Option<f64>,
    /// Library scan status text.
    pub refresh_status: Option<String>,
}

/// Unauthenticated server info (`GET /System/Info/Public`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct PublicSystemInfo {
    /// Server's local network address.
    pub local_address: Option<String>,
    /// Configured server name.
    pub server_name: Option<String>,
    /// Server version string (e.g. `"10.9.11"`).
    pub version: Option<String>,
    /// Product name (AssemblyProduct, e.g. `"Jellyfin Server"`).
    pub product_name: Option<String>,
    /// Obsolete: no longer set (empty string). Kept for backward compatibility.
    pub operating_system: Option<String>,
    /// Server unique id (string).
    pub id: Option<String>,
    /// Whether the startup wizard has completed (nullable for API client
    /// backward compat).
    pub startup_wizard_completed: Option<bool>,
}

/// Authenticated server info (`GET /System/Info`); extends [`PublicSystemInfo`].
/// Many fields are obsolete but still serialised.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct SystemInfo {
    /// Inherited: local network address.
    pub local_address: Option<String>,
    /// Inherited: configured server name.
    pub server_name: Option<String>,
    /// Inherited: server version string.
    pub version: Option<String>,
    /// Inherited: product name.
    pub product_name: Option<String>,
    /// Inherited, obsolete: empty string.
    pub operating_system: Option<String>,
    /// Inherited: server unique id.
    pub id: Option<String>,
    /// Inherited: whether the startup wizard completed.
    pub startup_wizard_completed: Option<bool>,
    /// Obsolete: no longer set (empty string).
    pub operating_system_display_name: Option<String>,
    /// Build package name (from -package CLI arg).
    pub package_name: Option<String>,
    /// Whether a restart is pending; non-nullable.
    pub has_pending_restart: bool,
    /// Whether the server is shutting down; non-nullable.
    pub is_shutting_down: bool,
    /// Whether realtime library monitoring is supported; non-nullable.
    pub supports_library_monitor: bool,
    /// WebSocket port number; non-nullable.
    pub web_socket_port_number: i32,
    /// Completed plugin installations (InstallationInfo); defaults to empty array.
    #[serde(default)]
    pub completed_installations: Vec<serde_json::Value>,
    /// Obsolete: always true; non-nullable.
    pub can_self_restart: bool,
    /// Obsolete: always false; non-nullable.
    pub can_launch_web_browser: bool,
    /// Obsolete (use SystemStorageDto): program data path.
    pub program_data_path: Option<String>,
    /// Obsolete: web UI resources path.
    pub web_path: Option<String>,
    /// Obsolete: items-by-name path.
    pub items_by_name_path: Option<String>,
    /// Obsolete: cache path.
    pub cache_path: Option<String>,
    /// Obsolete: log path.
    pub log_path: Option<String>,
    /// Obsolete: internal metadata path.
    pub internal_metadata_path: Option<String>,
    /// Obsolete: transcoding temp path.
    pub transcoding_temp_path: Option<String>,
    /// Configured Cast receiver applications (CastReceiverApplication).
    #[serde(default)]
    pub cast_receiver_applications: Vec<serde_json::Value>,
    /// Obsolete: always false; non-nullable.
    pub has_update_available: bool,
    /// Obsolete: not set correctly anymore (defaults `"System"`).
    pub encoder_location: Option<String>,
    /// Obsolete: no longer set (defaults `"X64"`).
    pub system_architecture: Option<String>,
}

/// The paginated list envelope of [`BaseItemDto`] — the concrete instantiation
/// of Jellyfin's generic `QueryResult<T>`. Returned by `GET /Items`,
/// `GET /Users/{userId}/Items`, and most item-list endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct BaseItemDtoQueryResult {
    /// Page of items; defaults to empty array, always present.
    #[serde(default)]
    pub items: Vec<BaseItemDto>,
    /// Total number of records matching the query (across all pages);
    /// non-nullable.
    pub total_record_count: i32,
    /// Index of the first item in `Items` within the full result set;
    /// non-nullable.
    pub start_index: i32,
}

#[cfg(test)]
#[path = "jellyfin_tests.rs"]
mod tests;
