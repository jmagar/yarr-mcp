//! DownloadClient models — SABnzbd (`?mode=` query API) and qBittorrent
//! (`/api/v2` WebUI REST).
//!
//! The two clients share no schema. SABnzbd wraps its queue under
//! `{ queue: { slots: [...] } }` and reports numeric fields as *strings*
//! (`"96.5"`, `"1024.00"`); qBittorrent's `/torrents/info` is a flat array of
//! torrents with native numeric fields. Field selection mirrors the slim
//! keep-lists in `crate::app::download`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── SABnzbd ──────────────────────────────────────────────────────────────────

/// `GET /api?mode=queue&output=json` → `{ queue: { slots: [...] } }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SabQueueResponse {
    pub queue: Option<SabQueue>,
}

/// The `queue` object holding the active download slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SabQueue {
    #[serde(default)]
    pub slots: Vec<SabSlot>,
}

/// A single SABnzbd queue slot, slimmed. SABnzbd serialises numeric progress/size
/// fields as strings, so `percentage` / `mb` / `mbleft` are `String`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SabSlot {
    pub nzo_id: Option<String>,
    pub filename: Option<String>,
    pub status: Option<String>,
    /// Percent complete as a string, e.g. `"96"`.
    pub percentage: Option<String>,
    /// Total size in MB as a string, e.g. `"1024.00"`.
    pub mb: Option<String>,
    /// Remaining size in MB as a string.
    pub mbleft: Option<String>,
    pub timeleft: Option<String>,
    pub cat: Option<String>,
    pub priority: Option<String>,
}

// ── qBittorrent ──────────────────────────────────────────────────────────────

/// A `GET /api/v2/torrents/info` row, slimmed to identity + progress/throughput.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TorrentInfo {
    pub hash: Option<String>,
    pub name: Option<String>,
    /// e.g. `downloading`, `stalledUP`, `pausedDL`.
    pub state: Option<String>,
    /// Fraction complete in `0.0..=1.0`.
    pub progress: Option<f64>,
    /// Download speed in bytes/sec.
    pub dlspeed: Option<i64>,
    /// Total size in bytes.
    pub size: Option<i64>,
    pub category: Option<String>,
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod tests;
