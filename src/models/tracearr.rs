//! Tracearr models — public API (`/api/v1/public`).
//!
//! Source of truth is the zod-to-openapi schema (OpenAPI 3.0.0, "Tracearr
//! Public API") that drives both validation and docs. List endpoints
//! (`/users`, `/violations`, `/history`) wrap rows in a `{ data, meta }`
//! envelope; `/streams` wraps in `{ data, summary }`. `/health`, `/stats`,
//! `/stats/today`, and `/activity` are bare objects.
//!
//! Tracearr quirks captured here:
//! - The reserved word `type` on [`ServerStatus`] and [`ViolationRule`] is
//!   renamed to `kind` via `#[serde(rename = "type")]`.
//! - zod `.nullable()` keys are always present but may be `null` → `Option<T>`;
//!   zod `.optional()` keys (only the [`StreamDetails`] sub-objects) may be
//!   absent → `Option<T>` with `#[serde(default)]`.
//! - Timestamps are ISO-8601 datetime *strings* (`2024-01-15T12:00:00.000Z`),
//!   not epoch ints. Activity bucket dates are non-ISO `YYYY-MM-DD HH:MM:SS`
//!   strings. All modelled as `String`.
//! - `StreamsSummary.totalBitrate` is a pre-formatted string (`"22.5 Mbps"`),
//!   not numeric.
//! - The shared `ServerInfo` / `MediaInfo` / `DeviceInfo` / `StreamDetails` /
//!   `DisplayValues` shapes are spread inline (top-level JSON keys) in the
//!   source, so they are `#[serde(flatten)]` mixins here.
//! - `Violation.data` is an arbitrary JSON object (`z.record(string, unknown)`)
//!   → `serde_json::Value`.
//! - Float fields (`watchTimeHours`, `aspectRatio`, transcode `speed`, …) mean
//!   `Eq` is *not* derived anywhere in this module.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Media server kind (`ServerStatus.type`). Lowercase string values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ServerType {
    Plex,
    Jellyfin,
    Emby,
}

/// Media item type (`MediaInfo.mediaType`). Lowercase string values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Movie,
    Episode,
    Track,
    Live,
    Photo,
    Unknown,
}

/// Playback state (`Stream.state` / `SessionHistory.state`). Lowercase values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

/// Violation severity (`Violation.severity`). Lowercase values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Warning,
    High,
}

/// User role (`User.role`). Lowercase values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Owner,
    Admin,
    Viewer,
    Member,
    Disabled,
    Pending,
}

/// Per-track transcode decision. Note `directplay` is one word (no underscore),
/// so lowercase serialisation matches the wire form exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TranscodeDecision {
    Directplay,
    Copy,
    Transcode,
}

/// Activity query time range (`ActivityResponse.period`). Lowercase values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ActivityPeriod {
    Week,
    Month,
    Year,
}

/// Violation rule kind (`ViolationRule.type`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViolationRuleKind {
    ConcurrentStreams,
    Transcode,
    Bitrate,
    Resolution,
    Location,
    Device,
    Platform,
    #[serde(other)]
    Unknown,
}

// ---------------------------------------------------------------------------
// Shared mixins (spread inline via `#[serde(flatten)]`)
// ---------------------------------------------------------------------------

/// Shared server-identity mixin spread into [`Stream`], [`SessionHistory`],
/// [`User`], [`Violation`], and [`ServerStreamSummary`] (fields land at the top
/// level of each containing object).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub server_id: Option<String>,
    pub server_name: Option<String>,
}

/// Shared media-metadata mixin spread into [`Stream`] and [`SessionHistory`].
/// Every field is nullable on the wire (key present, value may be `null`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub media_title: Option<String>,
    pub media_type: Option<MediaType>,
    /// Show name (episodes only); nullable.
    pub show_title: Option<String>,
    pub season_number: Option<i64>,
    pub episode_number: Option<i64>,
    pub year: Option<i64>,
    /// Artist (music tracks only); nullable.
    pub artist_name: Option<String>,
    /// Album (music tracks only); nullable.
    pub album_name: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    /// Poster path; nullable.
    pub thumb_path: Option<String>,
    /// Proxied poster URL; nullable.
    pub poster_url: Option<String>,
}

/// Shared device mixin spread into [`Stream`] and [`SessionHistory`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// Device name, e.g. `"Apple TV"`; nullable.
    pub device: Option<String>,
    /// Player app name; nullable.
    pub player: Option<String>,
    /// Product name; nullable.
    pub product: Option<String>,
    /// Platform, e.g. `"tvOS"`; nullable.
    pub platform: Option<String>,
}

/// Shared codec/quality mixin spread into [`Stream`] and [`SessionHistory`].
/// The six nested detail objects are themselves nullable *and* carry
/// all-optional inner fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreamDetails {
    pub is_transcode: Option<bool>,
    pub video_decision: Option<TranscodeDecision>,
    pub audio_decision: Option<TranscodeDecision>,
    /// Bitrate in kbps; nullable.
    pub bitrate: Option<i64>,
    pub source_video_codec: Option<String>,
    pub source_audio_codec: Option<String>,
    pub source_audio_channels: Option<i64>,
    pub source_video_width: Option<i64>,
    pub source_video_height: Option<i64>,
    pub source_video_details: Option<SourceVideoDetails>,
    pub source_audio_details: Option<SourceAudioDetails>,
    pub stream_video_codec: Option<String>,
    pub stream_audio_codec: Option<String>,
    pub stream_video_details: Option<StreamVideoDetails>,
    pub stream_audio_details: Option<StreamAudioDetails>,
    pub transcode_info: Option<TranscodeInfo>,
    pub subtitle_info: Option<SubtitleInfo>,
}

/// Shared human-readable display mixin spread into [`Stream`] and
/// [`SessionHistory`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DisplayValues {
    /// `4K` / `1080p` / `720p`; nullable.
    pub resolution: Option<String>,
    pub source_video_codec_display: Option<String>,
    pub source_audio_codec_display: Option<String>,
    /// e.g. `"7.1"`; nullable.
    pub audio_channels_display: Option<String>,
    pub stream_video_codec_display: Option<String>,
    pub stream_audio_codec_display: Option<String>,
}

// ---------------------------------------------------------------------------
// StreamDetails sub-objects (whole object nullable; every field truly optional)
// ---------------------------------------------------------------------------

/// Detailed source video metadata. The whole object is nullable; every field is
/// `.optional()` on the wire (may be absent), hence `#[serde(default)]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceVideoDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<f64>,
    /// Frame rate as a string, e.g. `"23.976"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framerate: Option<String>,
    /// e.g. `"HDR10"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_range: Option<String>,
    /// e.g. `1.78`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<f64>,
    /// e.g. `"main 10"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// e.g. `"5.1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// e.g. `"bt2020nc"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_space: Option<String>,
    /// Bit depth, e.g. `10`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_depth: Option<f64>,
}

/// Detailed source audio metadata. Whole object nullable; all fields optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SourceAudioDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<f64>,
    /// e.g. `"7.1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_layout: Option<String>,
    /// e.g. `"eng"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// e.g. `48000`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<f64>,
}

/// Detailed output (transcoded) video metadata. Whole object nullable; all
/// fields optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreamVideoDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<f64>,
    /// Output width px, e.g. `1920`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// Output height px, e.g. `1080`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// e.g. `"23.976"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framerate: Option<String>,
    /// e.g. `"SDR"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_range: Option<String>,
}

/// Detailed output audio metadata. Whole object nullable; all fields optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreamAudioDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<f64>,
    /// Output channel count, e.g. `2`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<f64>,
    /// e.g. `"eng"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// Transcoding session details. Whole object nullable; all fields optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeInfo {
    pub container_decision: Option<TranscodeDecision>,
    /// e.g. `"mkv"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_container: Option<String>,
    /// e.g. `"mpegts"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_container: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hw_requested: Option<bool>,
    /// e.g. `"videotoolbox"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hw_decoding: Option<String>,
    /// e.g. `"videotoolbox"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hw_encoding: Option<String>,
    /// Transcode speed multiplier, e.g. `2.5`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub throttled: Option<bool>,
    /// Reasons transcoding was triggered.
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// Subtitle handling details. Whole object nullable; all fields optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleInfo {
    /// e.g. `"burn"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    /// e.g. `"srt"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    /// e.g. `"eng"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forced: Option<bool>,
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

/// `GET /api/v1/public/health` — connection status for all configured media
/// servers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    /// Literal `"ok"` (`z.literal('ok')`).
    pub status: Option<String>,
    /// Tracearr server version, e.g. `"1.4.22"`.
    pub version: Option<String>,
    /// ISO-8601 datetime, e.g. `"2024-01-15T12:00:00.000Z"`.
    pub timestamp: Option<String>,
    #[serde(default)]
    pub servers: Vec<ServerStatus>,
}

/// Connectivity status of one media server (`HealthResponse.servers[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub id: Option<String>,
    /// Server display name, e.g. `"Main Plex Server"`.
    pub name: Option<String>,
    /// Reserved word `type` on the wire → `kind`. `plex` / `jellyfin` / `emby`.
    #[serde(rename = "type")]
    pub kind: Option<ServerType>,
    pub online: Option<bool>,
    pub active_streams: Option<i64>,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// `GET /api/v1/public/stats` — aggregate dashboard counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub active_streams: Option<i64>,
    pub total_users: Option<i64>,
    /// Sessions in the last 30 days.
    pub total_sessions: Option<i64>,
    /// Violations in the last 7 days.
    pub recent_violations: Option<i64>,
    /// ISO-8601 datetime.
    pub timestamp: Option<String>,
}

/// `GET /api/v1/public/stats/today` — today's metrics in a given IANA timezone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatsTodayResponse {
    pub active_streams: Option<i64>,
    /// Validated plays today (>= 2 min).
    pub today_plays: Option<i64>,
    /// Hours watched today (float, e.g. `12.5`).
    pub watch_time_hours: Option<f64>,
    pub alerts_last24h: Option<i64>,
    pub active_users_today: Option<i64>,
    /// ISO-8601 datetime.
    pub timestamp: Option<String>,
}

// ---------------------------------------------------------------------------
// Activity
// ---------------------------------------------------------------------------

/// `GET /api/v1/public/activity` — consolidated playback activity across six
/// dimensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActivityResponse {
    /// `week` / `month` / `year`.
    pub period: Option<ActivityPeriod>,
    /// Inline `{ start, end }` ISO datetime window.
    pub range: Option<ActivityRange>,
    /// Play counts bucketed over time (engagement >= 2 min).
    #[serde(default)]
    pub plays: Vec<PlayDataPoint>,
    /// Peak concurrent streams per bucket by playback type.
    #[serde(default)]
    pub concurrent: Vec<ConcurrentDataPoint>,
    /// Play distribution by weekday; always 7 entries.
    #[serde(default)]
    pub by_day_of_week: Vec<DayOfWeekDataPoint>,
    /// Play distribution by hour; always 24 entries.
    #[serde(default)]
    pub by_hour_of_day: Vec<HourOfDayDataPoint>,
    /// Session counts by client platform, desc.
    #[serde(default)]
    pub platforms: Vec<PlatformDataPoint>,
    /// Direct/transcode breakdown with percentages.
    pub quality: Option<QualityBreakdown>,
}

/// Inline `{ start, end }` object on [`ActivityResponse::range`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActivityRange {
    /// ISO-8601 datetime window start.
    pub start: Option<String>,
    /// ISO-8601 datetime window end.
    pub end: Option<String>,
}

/// `ActivityResponse.plays[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlayDataPoint {
    /// Bucket start time, non-ISO `YYYY-MM-DD HH:MM:SS` (space separator, no TZ).
    pub date: Option<String>,
    pub count: Option<i64>,
}

/// `ActivityResponse.concurrent[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConcurrentDataPoint {
    /// Bucket start time, non-ISO `YYYY-MM-DD HH:MM:SS`.
    pub date: Option<String>,
    /// Peak concurrent streams.
    pub total: Option<i64>,
    /// Direct play streams.
    pub direct: Option<i64>,
    /// Direct stream (remux).
    pub direct_stream: Option<i64>,
    /// Transcoding streams.
    pub transcode: Option<i64>,
}

/// `ActivityResponse.byDayOfWeek[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DayOfWeekDataPoint {
    /// `0`=Sunday .. `6`=Saturday.
    pub day: Option<i64>,
    /// Short weekday name, e.g. `"Fri"`.
    pub name: Option<String>,
    pub count: Option<i64>,
}

/// `ActivityResponse.byHourOfDay[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HourOfDayDataPoint {
    /// `0`-`23`.
    pub hour: Option<i64>,
    pub count: Option<i64>,
}

/// `ActivityResponse.platforms[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDataPoint {
    /// Client platform name; nullable (key always present, value may be `null`).
    pub platform: Option<String>,
    pub count: Option<i64>,
}

/// Playback-quality breakdown with rounded percentages
/// (`ActivityResponse.quality`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityBreakdown {
    pub direct_play: Option<i64>,
    pub direct_stream: Option<i64>,
    pub transcode: Option<i64>,
    pub total: Option<i64>,
    pub direct_play_percent: Option<i64>,
    pub direct_stream_percent: Option<i64>,
    pub transcode_percent: Option<i64>,
}

// ---------------------------------------------------------------------------
// Streams
// ---------------------------------------------------------------------------

/// `GET /api/v1/public/streams` — active sessions.
///
/// UNION quirk: when the query is `summary=true` the `data` array is omitted
/// (the `StreamsSummaryOnlyResponse` arm); both arms collapse into this one
/// shape with `data: Option<Vec<Stream>>` and a `summary` that is always
/// present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreamsResponse {
    /// Active streams; ABSENT when `summary=true` (union quirk).
    #[serde(default)]
    pub data: Option<Vec<Stream>>,
    /// Aggregate summary; always present.
    pub summary: Option<StreamsSummary>,
}

/// Active playback session.
///
/// `ServerInfo` / `MediaInfo` / `StreamDetails` / `DisplayValues` / `DeviceInfo`
/// are spread inline in the source — their keys appear at the top level of this
/// JSON object, so they are `#[serde(flatten)]` mixins here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    pub id: Option<String>,
    #[serde(flatten)]
    pub server_info: ServerInfo,
    /// User display name, e.g. `"John Doe"`.
    pub username: Option<String>,
    /// Avatar path; nullable.
    pub user_thumb: Option<String>,
    /// Proxied avatar URL; nullable.
    pub user_avatar_url: Option<String>,
    #[serde(flatten)]
    pub media_info: MediaInfo,
    /// Total media length in ms; nullable.
    pub duration_ms: Option<i64>,
    /// `playing` / `paused` / `stopped`.
    pub state: Option<PlaybackState>,
    /// Current playback position in ms.
    pub progress_ms: Option<i64>,
    /// ISO-8601 datetime.
    pub started_at: Option<String>,
    #[serde(flatten)]
    pub stream_details: StreamDetails,
    #[serde(flatten)]
    pub display_values: DisplayValues,
    #[serde(flatten)]
    pub device_info: DeviceInfo,
}

/// Aggregate summary of all active streams (`StreamsResponse.summary`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreamsSummary {
    pub total: Option<i64>,
    pub transcodes: Option<i64>,
    pub direct_streams: Option<i64>,
    pub direct_plays: Option<i64>,
    /// Pre-formatted string, e.g. `"45.2 Mbps"` (NOT numeric).
    pub total_bitrate: Option<String>,
    #[serde(default)]
    pub by_server: Vec<ServerStreamSummary>,
}

/// Per-server stream summary (`StreamsSummary.byServer[]`). Spreads
/// [`ServerInfo`] inline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerStreamSummary {
    #[serde(flatten)]
    pub server_info: ServerInfo,
    pub total: Option<i64>,
    pub transcodes: Option<i64>,
    pub direct_streams: Option<i64>,
    pub direct_plays: Option<i64>,
    /// Pre-formatted string, e.g. `"22.5 Mbps"`.
    pub total_bitrate: Option<String>,
}

/// `POST /api/v1/public/streams/{id}/terminate` request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminateStreamBody {
    /// Message shown to the user before termination; optional (may be absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// `POST .../terminate` response body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub enum TerminateStreamOutcome {
    Success(TerminateStreamResponse),
    Error(TerminateStreamErrorResponse),
}

impl<'de> Deserialize<'de> for TerminateStreamOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value.get("success").and_then(serde_json::Value::as_bool) {
            Some(true) => serde_json::from_value(value)
                .map(Self::Success)
                .map_err(serde::de::Error::custom),
            Some(false) => serde_json::from_value(value)
                .map(Self::Error)
                .map_err(serde::de::Error::custom),
            None => Err(serde::de::Error::custom(
                "terminate stream response missing boolean `success`",
            )),
        }
    }
}

/// `POST .../terminate` 200 success body. `success` is the literal `true`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminateStreamResponse {
    /// Always `true` (`z.literal(true)`).
    pub success: Option<bool>,
    /// UUID of the termination log entry.
    pub termination_log_id: Option<String>,
    /// e.g. `"Stream termination command sent successfully"`.
    pub message: Option<String>,
}

/// `POST .../terminate` 500 failure body. `success` is the literal `false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminateStreamErrorResponse {
    /// Always `false` (`z.literal(false)`).
    pub success: Option<bool>,
    pub error: Option<String>,
    /// UUID of the termination log entry.
    pub termination_log_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

/// `GET /api/v1/public/users` — paginated users (`{ data, meta }` envelope).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UsersResponse {
    #[serde(default)]
    pub data: Vec<User>,
    pub meta: Option<PaginationMeta>,
}

/// A user with activity metrics. Spreads [`ServerInfo`] inline; appears once per
/// server for multi-server users.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Option<String>,
    /// e.g. `"john_doe"`.
    pub username: Option<String>,
    /// e.g. `"John Doe"`.
    pub display_name: Option<String>,
    /// Avatar path; nullable.
    pub thumb_url: Option<String>,
    /// Proxied avatar URL; nullable.
    pub avatar_url: Option<String>,
    /// `owner` / `admin` / `viewer` / `member` / `disabled` / `pending`.
    pub role: Option<UserRole>,
    /// Trust score, `0`-`100`.
    pub trust_score: Option<i64>,
    pub total_violations: Option<i64>,
    #[serde(flatten)]
    pub server_info: ServerInfo,
    /// ISO-8601 datetime; nullable.
    pub last_activity_at: Option<String>,
    pub session_count: Option<i64>,
    /// ISO-8601 datetime.
    pub created_at: Option<String>,
}

/// Pagination metadata wrapper used by the list endpoints (`{ data, meta }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    /// Total matching records.
    pub total: Option<i64>,
    /// Current page (1-indexed).
    pub page: Option<i64>,
    /// Page size (max 100, default 25).
    pub page_size: Option<i64>,
}

// ---------------------------------------------------------------------------
// Violations
// ---------------------------------------------------------------------------

/// `GET /api/v1/public/violations` — paginated violations (`{ data, meta }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViolationsResponse {
    /// Violations, descending order.
    #[serde(default)]
    pub data: Vec<Violation>,
    pub meta: Option<PaginationMeta>,
}

/// A rule violation. Spreads [`ServerInfo`] inline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    pub id: Option<String>,
    #[serde(flatten)]
    pub server_info: ServerInfo,
    /// `low` / `warning` / `high`.
    pub severity: Option<Severity>,
    pub acknowledged: Option<bool>,
    /// Rule-specific violation data; an arbitrary JSON object
    /// (`z.record(string, unknown)`).
    pub data: Option<serde_json::Value>,
    /// ISO-8601 datetime.
    pub created_at: Option<String>,
    /// Triggering rule (inline object).
    pub rule: Option<ViolationRule>,
    /// Associated user (compact [`UserInfo`]).
    pub user: Option<UserInfo>,
}

/// Inline rule object on [`Violation::rule`]. Note the reserved word `type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ViolationRule {
    pub id: Option<String>,
    /// Reserved word `type` on the wire → `kind`.
    #[serde(rename = "type")]
    pub kind: Option<ViolationRuleKind>,
    /// Rule display name, e.g. `"Max 2 concurrent streams"`.
    pub name: Option<String>,
}

/// Compact user reference embedded in [`Violation::user`] and
/// [`SessionHistory::user`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: Option<String>,
    /// e.g. `"john_doe"`.
    pub username: Option<String>,
    /// Avatar path; nullable.
    pub thumb_url: Option<String>,
    /// Proxied avatar URL; nullable.
    pub avatar_url: Option<String>,
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

/// `GET /api/v1/public/history` — paginated session history (`{ data, meta }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResponse {
    #[serde(default)]
    pub data: Vec<SessionHistory>,
    pub meta: Option<PaginationMeta>,
}

/// Historical playback session (pause/resume cycles aggregated). Spreads
/// [`ServerInfo`] / [`MediaInfo`] / [`DeviceInfo`] / [`StreamDetails`] /
/// [`DisplayValues`] inline (top-level fields).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistory {
    pub id: Option<String>,
    #[serde(flatten)]
    pub server_info: ServerInfo,
    /// `playing` / `paused` / `stopped`.
    pub state: Option<PlaybackState>,
    #[serde(flatten)]
    pub media_info: MediaInfo,
    /// Total watch time across segments in ms; nullable.
    pub duration_ms: Option<i64>,
    /// Last position in ms; nullable.
    pub progress_ms: Option<i64>,
    /// Media length in ms; nullable.
    pub total_duration_ms: Option<i64>,
    /// ISO-8601 datetime.
    pub started_at: Option<String>,
    /// ISO-8601 datetime; nullable.
    pub stopped_at: Option<String>,
    /// True if watched 85%+.
    pub watched: Option<bool>,
    /// Pause/resume segment count.
    pub segment_count: Option<i64>,
    #[serde(flatten)]
    pub device_info: DeviceInfo,
    #[serde(flatten)]
    pub stream_details: StreamDetails,
    #[serde(flatten)]
    pub display_values: DisplayValues,
    /// Associated user (compact [`UserInfo`]).
    pub user: Option<UserInfo>,
}

#[cfg(test)]
#[path = "tracearr_tests.rs"]
mod tests;
