use super::*;
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

#[cfg(test)]
#[path = "tracearr_core_tests.rs"]
mod tests;

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------
