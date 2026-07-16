use super::*;
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
