use super::*;
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
