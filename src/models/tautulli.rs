//! Tautulli models — the Plex stats API (`GET /api/v2?cmd=…`).
//!
//! Tautulli wraps **every** command in a `{ response: { result, message, data } }`
//! envelope ([`TautulliEnvelope`] / [`TautulliResponse`]). The envelope is generic
//! over the command's `data` payload `T`, so a caller can decode any `cmd` into a
//! typed body. `result` is the literal `"success"` / `"error"`; `message` is `null`
//! on success and a string on error; `data` is the command-specific payload.
//!
//! Casing is **mixed**. Most data rows are snake_case (`rename_all = "snake_case"`),
//! but the `get_history` DataTables envelope mixes camelCase (`recordsTotal`,
//! `recordsFiltered`, `draw`) with snake_case (`total_duration`, `filter_duration`),
//! so [`GetHistoryData`] is modelled field-by-field with explicit `#[serde(rename)]`.
//!
//! Tautulli quirks worth flagging at the type level:
//! - **String-encoded numerics in `get_activity`**: top-level `stream_count`
//!   (`"1"`) and per-session `progress_percent` / `view_offset` / `duration` /
//!   `bitrate` / `bandwidth` are all serialised as *strings* — modelled `String`.
//! - **String-encoded counts in `get_libraries`**: `section_id`, `count`,
//!   `parent_count`, `child_count` are strings (`"62"`, `"2"`); but
//!   `get_library_names` reports `section_id` as a real `int`.
//! - **Epoch ints**: `get_history` `date` / `started` / `stopped` and `get_users`
//!   `last_seen` are Unix-timestamp integers.
//! - **`watched_status`** is `0` / `1` in the docs but Tautulli internals can emit
//!   fractional `0.5` for partial watches, so it is typed `f64`.
//! - The reserved word `type` on a session is renamed to `kind`.
//!
//! Unknown upstream fields deserialize-and-ignore via serde defaults; every
//! non-guaranteed field is `Option<T>` and every list field carries
//! `#[serde(default)]`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The outer Tautulli envelope present on every API response: `{ response: { … } }`.
/// Generic over the command's `data` payload `T` so any `cmd` decodes into a typed
/// body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TautulliEnvelope<T> {
    /// The single top-level key present in every Tautulli API response.
    pub response: Option<TautulliResponse<T>>,
}

/// The inner response object carrying result status, optional message, and the
/// command's `data` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TautulliResponse<T> {
    /// Literal status string: `"success"` or `"error"`.
    pub result: Option<String>,
    /// `null` on success; an error description string on failure.
    pub message: Option<String>,
    /// Command-specific payload. May be absent / null on error responses.
    pub data: Option<T>,
}

/// `cmd=get_activity` data: aggregate stream counts plus the active session list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetActivityData {
    /// Total active streams, STRING-encoded (e.g. `"1"`).
    pub stream_count: Option<String>,
    /// Number of direct-play streams (int).
    pub stream_count_direct_play: Option<i64>,
    /// Number of direct-stream streams (int).
    pub stream_count_direct_stream: Option<i64>,
    /// Number of transcoding streams (int).
    pub stream_count_transcode: Option<i64>,
    /// Total bandwidth in use, kbps (int).
    pub total_bandwidth: Option<i64>,
    /// LAN bandwidth usage (int).
    pub lan_bandwidth: Option<i64>,
    /// WAN bandwidth usage (int).
    pub wan_bandwidth: Option<i64>,
    /// Active streaming sessions.
    #[serde(default)]
    pub sessions: Vec<StreamSession>,
}

/// One active stream from `get_activity.sessions[]`. The full Tautulli object
/// documents ~200+ fields; this models the load-bearing identity / media subset.
/// STRING-encoded numerics are noted per field. Everything is `Option` because
/// presence varies widely by `media_type` and transcode state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StreamSession {
    /// Plex session key for this stream.
    pub session_key: Option<String>,
    /// Tautulli internal session id.
    pub session_id: Option<String>,
    /// Plex username, e.g. `"LordCommanderSnow"`.
    pub user: Option<String>,
    /// Plex username (duplicate of `user`).
    pub username: Option<String>,
    /// Plex user id (int).
    pub user_id: Option<i64>,
    /// User's friendly / display name.
    pub friendly_name: Option<String>,
    /// Combined title, e.g. `"Game of Thrones - The Red Woman"`.
    pub full_title: Option<String>,
    /// Item title, e.g. `"The Red Woman"`.
    pub title: Option<String>,
    /// Media type: `movie` | `episode` | `track` | `clip` etc.
    pub media_type: Option<String>,
    /// Playback state: `playing` | `paused` | `buffering`.
    pub state: Option<String>,
    /// Playback progress percent, STRING-encoded (e.g. `"0"`).
    pub progress_percent: Option<String>,
    /// Current playback offset in ms, STRING-encoded (e.g. `"1000"`).
    pub view_offset: Option<String>,
    /// Total item duration in ms, STRING-encoded (e.g. `"2998272"`).
    pub duration: Option<String>,
    /// Source bitrate, STRING-encoded (e.g. `"10617"`).
    pub bitrate: Option<String>,
    /// Player / device name, e.g. `"Castle-PC"`.
    pub player: Option<String>,
    /// Player platform, e.g. `"Plex Media Player"`.
    pub platform: Option<String>,
    /// Stream quality profile, e.g. `"Original"`.
    pub quality_profile: Option<String>,
    /// Overall stream decision: `direct play` | `copy` | `transcode`.
    pub transcode_decision: Option<String>,
    /// Video stream decision: `direct play` | `copy` | `transcode`.
    pub video_decision: Option<String>,
    /// Audio stream decision: `direct play` | `copy` | `transcode`.
    pub audio_decision: Option<String>,
    /// Container decision.
    pub container_decision: Option<String>,
    /// Plex rating key for the item.
    pub rating_key: Option<String>,
    /// Rating key of the parent (season / album).
    pub parent_rating_key: Option<String>,
    /// Rating key of the grandparent (show / artist).
    pub grandparent_rating_key: Option<String>,
    /// Grandparent title (show / artist name).
    pub grandparent_title: Option<String>,
    /// Library / section name the item belongs to.
    pub library_name: Option<String>,
    /// Library section id (string in the session payload).
    pub section_id: Option<String>,
    /// Client IP address.
    pub ip_address: Option<String>,
    /// Estimated required bandwidth, STRING-encoded.
    pub bandwidth: Option<String>,
    /// Plex client machine identifier.
    pub machine_id: Option<String>,
    /// Plex client product name.
    pub product: Option<String>,
    /// Release year, STRING-encoded.
    pub year: Option<String>,
    /// Item type. The reserved word `type` is renamed to `kind`.
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

/// `cmd=get_history` data: a DataTables-style payload. Mixed casing —
/// `recordsTotal` / `recordsFiltered` / `draw` are camelCase ints,
/// `total_duration` / `filter_duration` are snake_case STRINGS — so each key is
/// renamed explicitly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GetHistoryData {
    /// DataTables draw counter (int).
    #[serde(rename = "draw")]
    pub draw: Option<i64>,
    /// Total records available (int, camelCase key on the wire).
    #[serde(rename = "recordsTotal")]
    pub records_total: Option<i64>,
    /// Records matching the active filter (int, camelCase key on the wire).
    #[serde(rename = "recordsFiltered")]
    pub records_filtered: Option<i64>,
    /// Aggregate watch time, STRING (human / seconds formatted).
    #[serde(rename = "total_duration")]
    pub total_duration: Option<String>,
    /// Filtered aggregate watch time, STRING.
    #[serde(rename = "filter_duration")]
    pub filter_duration: Option<String>,
    /// History entry rows.
    #[serde(rename = "data", default)]
    pub data: Vec<HistoryRow>,
}

/// One watch-history entry from `get_history.data[]`. `date` / `started` /
/// `stopped` are EPOCH ints; `duration` / `play_duration` are ints (ms / sec).
/// `watched_status` is `0` / `1` in the docs but typed `f64` to allow Tautulli's
/// fractional `0.5` partial-watch values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct HistoryRow {
    /// Tautulli internal history row id.
    pub row_id: Option<i64>,
    /// Grouping reference id for continued plays.
    pub reference_id: Option<i64>,
    /// Watch date as a Unix EPOCH timestamp (int, e.g. `1462687607`).
    pub date: Option<i64>,
    /// Stream start Unix EPOCH timestamp (int).
    pub started: Option<i64>,
    /// Stream stop Unix EPOCH timestamp (int).
    pub stopped: Option<i64>,
    /// Plex username, e.g. `"DanyKhaleesi69"`.
    pub user: Option<String>,
    /// Plex user id (int).
    pub user_id: Option<i64>,
    /// User friendly / display name.
    pub friendly_name: Option<String>,
    /// Combined item title.
    pub full_title: Option<String>,
    /// Item title.
    pub title: Option<String>,
    /// Media type: `movie` | `episode` | `track` etc.
    pub media_type: Option<String>,
    /// Watched flag: `0` or `1` in the docs; Tautulli may emit fractional `0.5`
    /// for partial watches, hence `f64`.
    pub watched_status: Option<f64>,
    /// Completion percent 0-100 (int, e.g. `84`).
    pub percent_complete: Option<i64>,
    /// Player platform, e.g. `"Windows"`.
    pub platform: Option<String>,
    /// Player / device name, e.g. `"Castle-PC"`.
    pub player: Option<String>,
    /// Plex client product name.
    pub product: Option<String>,
    /// Total item duration (int ms / sec, e.g. `2998290`).
    pub duration: Option<i64>,
    /// Actual time played in seconds (int, e.g. `263`).
    pub play_duration: Option<i64>,
    /// Seconds spent paused (int).
    pub paused_counter: Option<i64>,
    /// Stream decision: `direct play` | `copy` | `transcode` (e.g. `"transcode"`).
    pub transcode_decision: Option<String>,
    /// Number of grouped history entries.
    pub group_count: Option<i64>,
    /// Comma-joined row ids in the group.
    pub group_ids: Option<String>,
    /// Plex rating key for the item.
    pub rating_key: Option<String>,
    /// Parent rating key (season / album).
    pub parent_rating_key: Option<String>,
    /// Grandparent rating key (show / artist).
    pub grandparent_rating_key: Option<String>,
    /// Grandparent title (show / artist name).
    pub grandparent_title: Option<String>,
    /// Parent title (season / album).
    pub parent_title: Option<String>,
    /// Track original / artist title.
    pub original_title: Option<String>,
    /// Episode / track index.
    pub media_index: Option<i64>,
    /// Season / disc index.
    pub parent_media_index: Option<i64>,
    /// Release year (int).
    pub year: Option<i64>,
    /// Client IP address.
    pub ip_address: Option<String>,
    /// Plex client machine id.
    pub machine_id: Option<String>,
    /// Plex session key (null for completed history).
    pub session_key: Option<String>,
    /// State if still active, else null.
    pub state: Option<String>,
    /// Thumbnail resource path.
    pub thumb: Option<String>,
    /// Plex item GUID.
    pub guid: Option<String>,
    /// Live-TV flag (0/1 int).
    pub live: Option<i64>,
    /// Plex relay flag (0/1 int).
    pub relayed: Option<i64>,
    /// Secure-connection flag (0/1 int).
    pub secure: Option<i64>,
    /// Connection location: `lan` | `wan` | `cellular`.
    pub location: Option<String>,
    /// Original air / release date string (YYYY-MM-DD).
    pub originally_available_at: Option<String>,
}

/// A user row. Models the `get_users_table` (DataTables) shape, which includes the
/// `plays` / `email` / `user_thumb` / `last_seen` fields; the plain `cmd=get_users`
/// variant omits `plays` / `duration` / `last_seen` and uses `thumb` instead of
/// `user_thumb`. `last_seen` is an EPOCH int; the `is_*` / `do_notify` /
/// `keep_history` / `allow_guest` flags are `0` / `1` ints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TautulliUserRow {
    /// Tautulli internal user row id.
    pub row_id: Option<i64>,
    /// Plex user id (int).
    pub user_id: Option<i64>,
    /// Plex username.
    pub username: Option<String>,
    /// User friendly / display name (falls back to username).
    pub friendly_name: Option<String>,
    /// Title of the user's last-played item (table variant).
    pub title: Option<String>,
    /// User email address.
    pub email: Option<String>,
    /// Avatar URL (table variant key; plain `get_users` uses `thumb`).
    pub user_thumb: Option<String>,
    /// Total play count for the user (int).
    pub plays: Option<i64>,
    /// Total watch duration in seconds (int).
    pub duration: Option<i64>,
    /// Unix EPOCH timestamp the user was last seen (int).
    pub last_seen: Option<i64>,
    /// Title of the last played item.
    pub last_played: Option<String>,
    /// Last seen client IP address.
    pub ip_address: Option<String>,
    /// Last seen player platform.
    pub platform: Option<String>,
    /// Last seen player / device name.
    pub player: Option<String>,
    /// Media type of the last played item.
    pub media_type: Option<String>,
    /// Transcode decision of the last play.
    pub transcode_decision: Option<String>,
    /// Notifications enabled flag (0/1 int).
    pub do_notify: Option<i64>,
    /// History tracking enabled flag (0/1 int).
    pub keep_history: Option<i64>,
    /// Guest-access allowed flag (0/1 int).
    pub allow_guest: Option<i64>,
    /// User active flag (0/1 int).
    pub is_active: Option<i64>,
    /// Admin flag (0/1 int) — present in plain `get_users`, absent in the table.
    pub is_admin: Option<i64>,
    /// Plex Home user flag (0/1 int).
    pub is_home_user: Option<i64>,
    /// Sync / download allowed flag (0/1 int).
    pub is_allow_sync: Option<i64>,
    /// Restricted (managed) user flag (0/1 int).
    pub is_restricted: Option<i64>,
    /// List of shared library section ids (split from a semicolon-delimited string).
    #[serde(default)]
    pub shared_libraries: Vec<String>,
}

/// A lightweight library identity row from `cmd=get_library_names`. `section_id`
/// is a real `int` here (contrast [`LibraryRow`] where it is a STRING).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LibraryNameRow {
    /// Plex library section id (int in this command).
    pub section_id: Option<i64>,
    /// Library display name.
    pub section_name: Option<String>,
    /// Library type: `movie` | `show` | `artist` | `photo`.
    pub section_type: Option<String>,
}

/// A library / section row from `cmd=get_libraries`. NOTE: `section_id`, `count`,
/// `parent_count`, `child_count` are STRING-encoded numbers; `is_active` is an int.
/// `agent` is not in the `get_libraries` example (it appears on `get_library`) and
/// is modelled Optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LibraryRow {
    /// Library section id, STRING-encoded (e.g. `"2"`).
    pub section_id: Option<String>,
    /// Library display name, e.g. `"TV Shows"`.
    pub section_name: Option<String>,
    /// Library type: `movie` | `show` | `artist` | `photo` (e.g. `"show"`).
    pub section_type: Option<String>,
    /// Metadata agent (e.g. `com.plexapp.agents.thetvdb`). Not in the
    /// `get_libraries` example; present on `get_library` — Optional.
    pub agent: Option<String>,
    /// Top-level item count, STRING-encoded (e.g. `"62"`).
    pub count: Option<String>,
    /// Parent item count (e.g. seasons), STRING-encoded (e.g. `"240"`).
    pub parent_count: Option<String>,
    /// Child item count (e.g. episodes), STRING-encoded (e.g. `"3745"`).
    pub child_count: Option<String>,
    /// Library active flag (0/1 int).
    pub is_active: Option<i64>,
    /// Fanart / art resource path.
    pub art: Option<String>,
    /// Thumbnail resource path.
    pub thumb: Option<String>,
}

/// Plex Media Server info from `cmd=get_server_info`. The `pms_*` int fields are
/// real ints; `pms_is_remote` / `pms_plexpass` / `pms_ssl` / `pms_url_manual` are
/// `0` / `1` flags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetServerInfoData {
    /// Friendly server name, e.g. `"Winterfell-Server"`.
    pub pms_name: Option<String>,
    /// PMS version string, e.g. `"1.20.0.3133-fede5bdc7"`.
    pub pms_version: Option<String>,
    /// Server OS / platform, e.g. `"Windows"`.
    pub pms_platform: Option<String>,
    /// Server machine identifier.
    pub pms_identifier: Option<String>,
    /// Server IP address, e.g. `"10.10.10.1"`.
    pub pms_ip: Option<String>,
    /// Server port (int, e.g. `32400`).
    pub pms_port: Option<i64>,
    /// Base server URL, e.g. `"http://10.10.10.1:32400"`.
    pub pms_url: Option<String>,
    /// Manual URL config flag (0/1 int).
    pub pms_url_manual: Option<i64>,
    /// Remote server flag (0/1 int).
    pub pms_is_remote: Option<i64>,
    /// SSL enabled flag (0/1 int).
    pub pms_ssl: Option<i64>,
    /// PlexPass status flag (0/1 int).
    pub pms_plexpass: Option<i64>,
}

#[cfg(test)]
#[path = "tautulli_tests.rs"]
mod tests;
