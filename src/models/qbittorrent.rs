//! qBittorrent models — WebUI API v5.0 (`/api/v2`).
//!
//! Source: the official wiki (no OpenAPI spec). Several quirks shape these
//! types:
//!
//! * **Mixed casing.** Most fields are flat snake_case, but a handful are
//!   camelCase exceptions *inside* otherwise-snake_case structs ([`TorrentInfo`]
//!   `isPrivate`; [`TorrentProperties`] `isPrivate`; [`Category`] `savePath`).
//!   serde has no "leave names alone" mode (`rename_all` only renames), so these
//!   structs carry **no** `rename_all` — the Rust field names already match the
//!   snake_case keys and the camelCase keys get an explicit
//!   `#[serde(rename = "…")]`.
//! * **`GET /app/version` is plain text**, not JSON (e.g. `v4.1.3`) — there is no
//!   struct for it; deserialize the raw body to `String`.
//! * **Native numerics.** Counts/speeds are real JSON integers/floats, *not*
//!   string-encoded (unlike SABnzbd / Tautulli).
//! * **Sentinels.** `dl_limit`/`up_limit` `-1` means unlimited; `priority` `-1`
//!   means queuing disabled or seed mode; any unknown integer property on
//!   [`TorrentProperties`] is reported as `-1`.
//! * **Epoch ints.** `added_on`, `completion_on`, `last_activity`,
//!   `seen_complete`, `creation_date`, `addition_date`, `completion_date`,
//!   `last_seen` are Unix epoch seconds (`i64`).
//! * **`progress`** is a fraction `0..=1`; **`ratio`** is capped at `9999`.
//! * **`tags`** is a single comma-concatenated `String`, *not* an array.
//! * **`GET /torrents/categories`** returns a JSON *object* keyed by category
//!   name (decode as `HashMap<String, Category>`), not an array.
//!
//! Per house style every field is `Option<T>` so partial / sync-maindata
//! variants and unknown upstream fields decode without error.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A torrent entry from `GET /api/v2/torrents/info` (returned as a JSON array).
///
/// Dominant casing is snake_case; `isPrivate` is the lone camelCase exception
/// (added in 5.0.0, absent on older clients) and carries an explicit rename.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TorrentInfo {
    /// Unix epoch seconds when the torrent was added.
    pub added_on: Option<i64>,
    /// Amount of data left to download (bytes).
    pub amount_left: Option<i64>,
    /// Whether the torrent is managed by Automatic Torrent Management.
    pub auto_tmm: Option<bool>,
    /// Percentage of file pieces currently available.
    pub availability: Option<f64>,
    /// Category of the torrent (may be an empty string).
    pub category: Option<String>,
    /// Amount of transfer data completed (bytes).
    pub completed: Option<i64>,
    /// Unix epoch seconds when the torrent completed.
    pub completion_on: Option<i64>,
    /// Absolute path of torrent content (root for multifile, file for single).
    pub content_path: Option<String>,
    /// Download speed limit (bytes/s); `-1` if unlimited.
    pub dl_limit: Option<i64>,
    /// Download speed (bytes/s).
    pub dlspeed: Option<i64>,
    /// Amount of data downloaded (bytes).
    pub downloaded: Option<i64>,
    /// Amount of data downloaded this session (bytes).
    pub downloaded_session: Option<i64>,
    /// ETA (seconds).
    pub eta: Option<i64>,
    /// True if first/last piece are prioritized.
    pub f_l_piece_prio: Option<bool>,
    /// True if force start is enabled.
    pub force_start: Option<bool>,
    /// Torrent hash (info hash).
    pub hash: Option<String>,
    /// True if from a private tracker (added in 5.0.0). camelCase key on the
    /// wire (`isPrivate`); absent on pre-5.0 clients.
    #[serde(rename = "isPrivate")]
    pub is_private: Option<bool>,
    /// Unix epoch seconds of the last downloaded/uploaded chunk.
    pub last_activity: Option<i64>,
    /// Magnet URI corresponding to this torrent.
    pub magnet_uri: Option<String>,
    /// Maximum share ratio until seeding stops.
    pub max_ratio: Option<f64>,
    /// Maximum seeding time (seconds) until seeding stops.
    pub max_seeding_time: Option<i64>,
    /// Torrent name.
    pub name: Option<String>,
    /// Number of seeds in the swarm.
    pub num_complete: Option<i64>,
    /// Number of leechers in the swarm.
    pub num_incomplete: Option<i64>,
    /// Number of leechers connected to.
    pub num_leechs: Option<i64>,
    /// Number of seeds connected to.
    pub num_seeds: Option<i64>,
    /// Priority; `-1` if queuing is disabled or the torrent is in seed mode.
    pub priority: Option<i64>,
    /// Progress as a fraction `0..=1` (percentage / 100).
    pub progress: Option<f64>,
    /// Share ratio; capped at `9999`.
    pub ratio: Option<f64>,
    /// Per-torrent share ratio limit setting.
    pub ratio_limit: Option<f64>,
    /// Seconds until the next tracker reannounce.
    pub reannounce: Option<i64>,
    /// Path where this torrent's data is stored.
    pub save_path: Option<String>,
    /// Elapsed time while complete (seconds).
    pub seeding_time: Option<i64>,
    /// Per-torrent seeding time limit (seconds).
    pub seeding_time_limit: Option<i64>,
    /// Unix epoch seconds this torrent was last seen complete.
    pub seen_complete: Option<i64>,
    /// True if sequential download is enabled.
    pub seq_dl: Option<bool>,
    /// Total size (bytes) of files selected for download.
    pub size: Option<i64>,
    /// Torrent state.
    pub state: Option<TorrentState>,
    /// True if super seeding is enabled.
    pub super_seeding: Option<bool>,
    /// Comma-concatenated tag list (a single `String`, *not* an array).
    pub tags: Option<String>,
    /// Total active time (seconds).
    pub time_active: Option<i64>,
    /// Total size (bytes) of all files (including unselected ones).
    pub total_size: Option<i64>,
    /// First tracker with working status; empty string if none is working.
    pub tracker: Option<String>,
    /// Upload speed limit (bytes/s); `-1` if unlimited.
    pub up_limit: Option<i64>,
    /// Amount of data uploaded (bytes).
    pub uploaded: Option<i64>,
    /// Amount of data uploaded this session (bytes).
    pub uploaded_session: Option<i64>,
    /// Upload speed (bytes/s).
    pub upspeed: Option<i64>,
}

/// Documented values for [`TorrentInfo`] `state`.
///
/// The wire values are mixed-case literals, so each variant carries an explicit
/// `#[serde(rename)]`. A catch-all [`TorrentState::Unknown`] (`#[serde(other)]`)
/// keeps forward compatibility: 5.x clients may emit `stopped*` variants in
/// place of the older `paused*` ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TorrentState {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "missingFiles")]
    MissingFiles,
    #[serde(rename = "uploading")]
    Uploading,
    #[serde(rename = "pausedUP")]
    PausedUP,
    #[serde(rename = "queuedUP")]
    QueuedUP,
    #[serde(rename = "stalledUP")]
    StalledUP,
    #[serde(rename = "checkingUP")]
    CheckingUP,
    #[serde(rename = "forcedUP")]
    ForcedUP,
    #[serde(rename = "allocating")]
    Allocating,
    #[serde(rename = "downloading")]
    Downloading,
    #[serde(rename = "metaDL")]
    MetaDL,
    #[serde(rename = "pausedDL")]
    PausedDL,
    #[serde(rename = "queuedDL")]
    QueuedDL,
    #[serde(rename = "stalledDL")]
    StalledDL,
    #[serde(rename = "checkingDL")]
    CheckingDL,
    #[serde(rename = "forcedDL")]
    ForcedDL,
    #[serde(rename = "checkingResumeData")]
    CheckingResumeData,
    #[serde(rename = "moving")]
    Moving,
    /// Any unrecognised state (forward-compat fallback, e.g. 5.x `stopped*`).
    #[serde(other)]
    Unknown,
}

/// Detailed per-torrent properties from `GET /api/v2/torrents/properties`.
///
/// Dominant casing is snake_case with one camelCase exception (`isPrivate`).
/// `-1` is returned for any integer property whose value is unknown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TorrentProperties {
    /// Save path.
    pub save_path: Option<String>,
    /// Creation date (Unix epoch seconds).
    pub creation_date: Option<i64>,
    /// Piece size (bytes).
    pub piece_size: Option<i64>,
    /// Comment.
    pub comment: Option<String>,
    /// Total data wasted (bytes).
    pub total_wasted: Option<i64>,
    /// Total data uploaded (bytes).
    pub total_uploaded: Option<i64>,
    /// Total data uploaded this session (bytes).
    pub total_uploaded_session: Option<i64>,
    /// Total data downloaded (bytes).
    pub total_downloaded: Option<i64>,
    /// Total data downloaded this session (bytes).
    pub total_downloaded_session: Option<i64>,
    /// Upload limit (bytes/s); `-1` if unknown.
    pub up_limit: Option<i64>,
    /// Download limit (bytes/s); `-1` if unknown.
    pub dl_limit: Option<i64>,
    /// Elapsed time (seconds).
    pub time_elapsed: Option<i64>,
    /// Elapsed time while complete (seconds).
    pub seeding_time: Option<i64>,
    /// Connection count.
    pub nb_connections: Option<i64>,
    /// Connection count limit.
    pub nb_connections_limit: Option<i64>,
    /// Share ratio.
    pub share_ratio: Option<f64>,
    /// When this torrent was added (Unix epoch seconds).
    pub addition_date: Option<i64>,
    /// Completion date (Unix epoch seconds); `-1` if not completed.
    pub completion_date: Option<i64>,
    /// Torrent creator.
    pub created_by: Option<String>,
    /// Average download speed (bytes/s).
    pub dl_speed_avg: Option<i64>,
    /// Download speed (bytes/s).
    pub dl_speed: Option<i64>,
    /// ETA (seconds).
    pub eta: Option<i64>,
    /// Last seen complete date (Unix epoch seconds); `-1` if unknown.
    pub last_seen: Option<i64>,
    /// Number of peers connected to.
    pub peers: Option<i64>,
    /// Number of peers in the swarm.
    pub peers_total: Option<i64>,
    /// Number of pieces owned.
    pub pieces_have: Option<i64>,
    /// Number of pieces of the torrent.
    pub pieces_num: Option<i64>,
    /// Seconds until the next announce.
    pub reannounce: Option<i64>,
    /// Number of seeds connected to.
    pub seeds: Option<i64>,
    /// Number of seeds in the swarm.
    pub seeds_total: Option<i64>,
    /// Total size (bytes).
    pub total_size: Option<i64>,
    /// Average upload speed (bytes/s).
    pub up_speed_avg: Option<i64>,
    /// Upload speed (bytes/s).
    pub up_speed: Option<i64>,
    /// True if from a private tracker. camelCase key on the wire (`isPrivate`);
    /// may be absent on older clients.
    #[serde(rename = "isPrivate")]
    pub is_private: Option<bool>,
}

/// Global transfer/connection statistics from `GET /api/v2/transfer/info`.
///
/// All keys are snake_case. The last three fields (`queueing`,
/// `use_alt_speed_limits`, `refresh_interval`) only appear in the sync-maindata
/// "partial data" variant, so they are `Option` regardless.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TransferInfo {
    /// Global download rate (bytes/s).
    pub dl_info_speed: Option<i64>,
    /// Data downloaded this session (bytes).
    pub dl_info_data: Option<i64>,
    /// Global upload rate (bytes/s).
    pub up_info_speed: Option<i64>,
    /// Data uploaded this session (bytes).
    pub up_info_data: Option<i64>,
    /// Download rate limit (bytes/s).
    pub dl_rate_limit: Option<i64>,
    /// Upload rate limit (bytes/s).
    pub up_rate_limit: Option<i64>,
    /// Number of DHT nodes connected to.
    pub dht_nodes: Option<i64>,
    /// Connection status.
    pub connection_status: Option<ConnectionStatus>,
    /// True if torrent queueing is enabled. Sync-maindata partial data only.
    pub queueing: Option<bool>,
    /// True if alternative speed limits are enabled. Sync-maindata partial only.
    pub use_alt_speed_limits: Option<bool>,
    /// Transfer list refresh interval (ms). Sync-maindata partial data only.
    pub refresh_interval: Option<i64>,
}

/// Documented values for [`TransferInfo`] `connection_status`. The three wire
/// values are lowercase, so `rename_all = "lowercase"` covers them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connected,
    Firewalled,
    Disconnected,
}

/// A single category — a value in the `GET /api/v2/torrents/categories` map
/// (decode the endpoint as `HashMap<String, Category>`, *not* an array).
///
/// Casing is non-uniform: `name` is lowercase, `savePath` is camelCase — hence
/// no blanket `rename_all` and an explicit rename on `save_path`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Category {
    /// Category name.
    pub name: Option<String>,
    /// Path where torrents in this category are saved. camelCase key on the
    /// wire (`savePath`).
    #[serde(rename = "savePath")]
    pub save_path: Option<String>,
}

/// Application build/dependency versions from `GET /api/v2/app/buildInfo`.
///
/// Uniform lowercase keys; `bitness` is an integer (e.g. `64`), not a string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildInfo {
    /// Qt version.
    pub qt: Option<String>,
    /// libtorrent version.
    pub libtorrent: Option<String>,
    /// Boost version.
    pub boost: Option<String>,
    /// OpenSSL version.
    pub openssl: Option<String>,
    /// Application bitness, e.g. `64` (integer, not a string).
    pub bitness: Option<i64>,
}

#[cfg(test)]
#[path = "qbittorrent_tests.rs"]
mod tests;
