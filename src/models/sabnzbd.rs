//! SABnzbd models — the Usenet download client (`GET /api?mode=…&output=json`).
//!
//! Every SABnzbd endpoint wraps its payload in a single-key envelope —
//! `{ "queue": { … } }`, `{ "history": { … } }`, `{ "version": "…" }` — so each
//! gets a thin wrapper struct ([`QueueResponse`], [`HistoryResponse`],
//! [`VersionResponse`]). The fields here are the full documented superset, not a
//! slim subset.
//!
//! SABnzbd serialisation quirk (verified against the SABnzbd Tavern test
//! fixtures, which are ground truth): the JSON output encodes most numeric
//! size/speed/disk/progress values as JSON *strings*, not numbers — e.g. queue
//! `speed`/`kbpersec`/`size`/`mb`/`diskspace1`, and slot `percentage`/`mb`/
//! `priority`/`unpackopts`. Those are all modelled as `Option<String>` and the
//! field docs call it out. The genuinely integral fields (queue `noofslots`,
//! slot `time_added`, all of the history `bytes`/`downloaded`/`completed`
//! timestamps) stay `i64`/`Option<i64>`. Booleans (`paused`, `paused_all`,
//! `have_quota`, history `loaded`/`archive`/`retry`) are real JSON booleans.
//!
//! All keys are already snake_case on the wire, so `rename_all = "snake_case"`
//! is a no-op set defensively. No upstream key collides with a Rust reserved
//! word, so no `#[serde(rename = …)]` exceptions are needed.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Envelope for `GET /api?mode=version&output=json`: `{ "version": "4.3.3" }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VersionResponse {
    /// SABnzbd version string, e.g. `"4.3.3"`.
    pub version: Option<String>,
}

/// Envelope for `GET /api?mode=queue&output=json`, wrapping the payload under
/// the single key `"queue"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueueResponse {
    pub queue: Queue,
}

/// Top-level download queue state. Numeric size/speed/disk fields are JSON
/// *strings*, not numbers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Queue {
    /// SABnzbd version string.
    pub version: Option<String>,
    /// Queue state, e.g. `"Downloading"`, `"Paused"`, `"Idle"`.
    pub status: Option<String>,
    /// Whether the queue is paused.
    pub paused: Option<bool>,
    /// Global/all-activity pause state (pauses post-processing too).
    pub paused_all: Option<bool>,
    /// Remaining pause interval as a string (e.g. `"0"` or `"5:00"`). STRING.
    pub pause_int: Option<String>,
    /// Current download speed, human-formatted string (e.g. `"1.3 M"`). STRING.
    pub speed: Option<String>,
    /// Current speed in KB/s as a numeric STRING (e.g. `"1234.5"`).
    pub kbpersec: Option<String>,
    /// Speed limit as percentage of max, numeric STRING.
    pub speedlimit: Option<String>,
    /// Absolute speed limit in bytes/s, numeric STRING (may be empty when unset).
    pub speedlimit_abs: Option<String>,
    /// Total queue size, human-formatted STRING (e.g. `"1.2 GB"`).
    pub size: Option<String>,
    /// Remaining queue size, human-formatted STRING.
    pub sizeleft: Option<String>,
    /// Total queue size in MB as a numeric STRING (e.g. `"1234.56"`).
    pub mb: Option<String>,
    /// Remaining size in MB as a numeric STRING.
    pub mbleft: Option<String>,
    /// ETA for the whole queue in `"H:MM:SS"` format.
    pub timeleft: Option<String>,
    /// Estimated completion clock time string (present in some versions; not in
    /// the core fixture). Modelled as optional.
    pub eta: Option<String>,
    /// Number of slots returned/matched (after start/limit/search filtering).
    pub noofslots: Option<i64>,
    /// Total number of jobs in the queue (unfiltered).
    pub noofslots_total: Option<i64>,
    /// Start index echoed from the request paging params.
    pub start: Option<i64>,
    /// Limit echoed from the request paging params.
    pub limit: Option<i64>,
    /// Count of jobs in finishing/post-processing state.
    pub finish: Option<i64>,
    /// Queued on-finish action (shutdown/hibernate/etc.); null when none set.
    pub finishaction: Option<String>,
    /// Free space on the primary (incomplete) download path, in GB as a numeric STRING.
    pub diskspace1: Option<String>,
    /// Free space on the secondary (complete) download path, in GB as a numeric STRING.
    pub diskspace2: Option<String>,
    /// Total space on the primary path, in GB as a numeric STRING.
    pub diskspacetotal1: Option<String>,
    /// Total space on the secondary path, in GB as a numeric STRING.
    pub diskspacetotal2: Option<String>,
    /// Free primary space, human-normalized STRING (e.g. `"161.2 G"`).
    pub diskspace1_norm: Option<String>,
    /// Free secondary space, human-normalized STRING.
    pub diskspace2_norm: Option<String>,
    /// Number of pending warnings as a numeric STRING.
    pub have_warnings: Option<String>,
    /// Configured quota, human-formatted STRING.
    pub quota: Option<String>,
    /// Whether a download quota is configured.
    pub have_quota: Option<bool>,
    /// Remaining quota, human-formatted STRING.
    pub left_quota: Option<String>,
    /// Number of cached articles as a numeric STRING.
    pub cache_art: Option<String>,
    /// Article cache size, human-formatted STRING.
    pub cache_size: Option<String>,
    /// Per-job queue entries.
    #[serde(default)]
    pub slots: Vec<QueueSlot>,
}

/// One job in the download queue. `percentage`/`mb`/`size` and friends are JSON
/// *strings*.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueueSlot {
    /// Unique job identifier (used to address the job in other API calls).
    pub nzo_id: Option<String>,
    /// Job display name / final name.
    pub filename: Option<String>,
    /// Job state: `"Downloading"`, `"Paused"`, `"Queued"`, `"Fetching"`, etc.
    pub status: Option<String>,
    /// Zero-based position in the queue. Serialised as a numeric STRING in JSON output.
    pub index: Option<String>,
    /// Download progress percentage as a numeric STRING (e.g. `"42"`).
    pub percentage: Option<String>,
    /// Total job size in MB as a numeric STRING.
    pub mb: Option<String>,
    /// Remaining size in MB as a numeric STRING.
    pub mbleft: Option<String>,
    /// Missing (un-fetchable) MB as a numeric STRING.
    pub mbmissing: Option<String>,
    /// Total job size, human-formatted STRING (e.g. `"1.2 GB"`).
    pub size: Option<String>,
    /// Remaining size, human-formatted STRING.
    pub sizeleft: Option<String>,
    /// Per-job ETA in `"H:MM:SS"` format.
    pub timeleft: Option<String>,
    /// Assigned category name.
    pub cat: Option<String>,
    /// Priority label STRING (e.g. `"Normal"`, `"High"`, `"Force"`).
    pub priority: Option<String>,
    /// Post-processing option as a numeric STRING: `"0"`=None, `"1"`=Repair,
    /// `"2"`=Repair/Unpack, `"3"`=Repair/Unpack/Delete.
    pub unpackopts: Option<String>,
    /// Post-processing script assigned to the job.
    pub script: Option<String>,
    /// Archive password set on the job, if any.
    pub password: Option<String>,
    /// Average age of the job's articles, human STRING (e.g. `"2895d"`).
    pub avg_age: Option<String>,
    /// Direct-unpack progress STRING (e.g. `"10/30"`); null when not direct-unpacking.
    pub direct_unpack: Option<String>,
    /// Job labels, e.g. `"DUPLICATE"`, `"ENCRYPTED"`, `"TOO LARGE"`.
    #[serde(default)]
    pub labels: Vec<String>,
    /// Unix epoch (seconds) when the job was added. EPOCH INT.
    pub time_added: Option<i64>,
}

/// Envelope for `GET /api?mode=history&output=json`, wrapping the payload under
/// the single key `"history"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct HistoryResponse {
    pub history: History,
}

/// Completed/failed job history plus rolling download totals. The `*_size`
/// totals are human-formatted strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct History {
    /// Completed/failed/post-processing job entries.
    #[serde(default)]
    pub slots: Vec<HistorySlot>,
    /// Total number of history entries (unfiltered count).
    pub noofslots: Option<i64>,
    /// Number of jobs currently in post-processing.
    pub ppslots: Option<i64>,
    /// All-time downloaded total, human-formatted STRING (e.g. `"3.4 T"`).
    pub total_size: Option<String>,
    /// Downloaded in the past month, human-formatted STRING.
    pub month_size: Option<String>,
    /// Downloaded in the past week, human-formatted STRING.
    pub week_size: Option<String>,
    /// Downloaded in the past 24h, human-formatted STRING.
    pub day_size: Option<String>,
    /// Unix epoch (seconds) of the last history update. EPOCH INT.
    pub last_history_update: Option<i64>,
    /// SABnzbd version string (also echoed inside the history payload).
    pub version: Option<String>,
}

/// One completed/failed/queued history entry. `bytes`/`downloaded`/`times`/
/// `completed` are real ints; `size` is a formatted string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct HistorySlot {
    /// Unique job identifier.
    pub nzo_id: Option<String>,
    /// Final job name.
    pub name: Option<String>,
    /// Original NZB filename.
    pub nzb_name: Option<String>,
    /// Completion status: `"Completed"`, `"Failed"`, `"Queued"`, `"Verifying"`,
    /// `"Repairing"`, `"Extracting"`, etc.
    pub status: Option<String>,
    /// Assigned category.
    pub category: Option<String>,
    /// Total size, human-formatted STRING (e.g. `"1.2 G"`).
    pub size: Option<String>,
    /// Total job size in bytes. REAL INT.
    pub bytes: Option<i64>,
    /// Bytes actually downloaded. REAL INT.
    pub downloaded: Option<i64>,
    /// Failure/error description; empty string when the job succeeded.
    pub fail_message: Option<String>,
    /// Final destination path of the completed job.
    pub storage: Option<String>,
    /// Temporary/working directory path used during processing.
    pub path: Option<String>,
    /// Download duration in seconds. REAL INT.
    pub download_time: Option<i64>,
    /// Post-processing duration in seconds. REAL INT.
    pub postproc_time: Option<i64>,
    /// Unix epoch (seconds) when the job completed. EPOCH INT.
    pub completed: Option<i64>,
    /// Unix epoch (seconds) when the job was added. EPOCH INT.
    pub time_added: Option<i64>,
    /// Post-processing applied, single-letter STRING: `""`/`"R"`=Repair,
    /// `"U"`=Repair+Unpack, `"D"`=Repair+Unpack+Delete.
    pub pp: Option<String>,
    /// Post-processing script used.
    pub script: Option<String>,
    /// Script result/output summary line.
    pub script_line: Option<String>,
    /// Current/last action detail line.
    pub action_line: Option<String>,
    /// Processing report; may be null.
    pub report: Option<String>,
    /// Source URL or originating filename.
    pub url: Option<String>,
    /// Additional URL/source info.
    pub url_info: Option<String>,
    /// Archive password recorded for the job.
    pub password: Option<String>,
    /// MD5 hash of the job (hex string).
    pub md5sum: Option<String>,
    /// Deduplication key (e.g. show/episode identifier); may be null.
    pub duplicate_key: Option<String>,
    /// Metadata field; typically null.
    pub meta: Option<String>,
    /// Series identifier; may be null/absent.
    pub series: Option<String>,
    /// Completeness indicator; documented as null in the JSON fixture.
    pub completeness: Option<String>,
    /// Whether the item is currently loaded/post-processing.
    #[serde(default)]
    pub loaded: Option<bool>,
    /// Whether the item has been moved to the history archive.
    #[serde(default)]
    pub archive: Option<bool>,
    /// Whether the job can be retried (the JSON fixture types this as a bool,
    /// not a count).
    #[serde(default)]
    pub retry: Option<bool>,
    /// Whether a user rating exists (Newzbin-style rating; absent on many installs).
    pub has_rating: Option<bool>,
    /// Per-stage processing log entries (Source/Download/Servers/Repair/Unpack/etc.).
    #[serde(default)]
    pub stage_log: Vec<HistoryStageLog>,
}

/// One stage entry within a history slot's processing log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct HistoryStageLog {
    /// Stage name: `"Source"`, `"Download"`, `"Servers"`, `"Repair"`,
    /// `"Unpack"`, `"Script"`, etc.
    pub name: Option<String>,
    /// Action/detail lines for the stage (may contain HTML).
    #[serde(default)]
    pub actions: Vec<String>,
}

#[cfg(test)]
#[path = "sabnzbd_tests.rs"]
mod tests;
