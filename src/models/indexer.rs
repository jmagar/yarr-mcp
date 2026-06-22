//! Indexer models — Prowlarr (`/api/v1`).
//!
//! Fields mirror the slim keep-lists in `crate::app::indexer`: the indexer
//! list, Newznab search releases, and the per-indexer stats counters.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `GET /api/v1/indexer` row, slimmed to routing-relevant identity fields (the
/// live row also carries the full indexer definition/capabilities payload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Indexer {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub enable: Option<bool>,
    /// `usenet` or `torrent`.
    pub protocol: Option<String>,
    pub priority: Option<i64>,
}

/// A `GET /api/v1/search` release hit, slimmed to enough to pick a result and
/// identify its source indexer (the live row carries large magnet/download URLs
/// and poster metadata that deserialize-and-ignore).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchRelease {
    pub title: Option<String>,
    pub indexer: Option<String>,
    pub indexer_id: Option<i64>,
    pub protocol: Option<String>,
    pub seeders: Option<i64>,
    pub leechers: Option<i64>,
    pub size: Option<u64>,
    pub publish_date: Option<String>,
    pub info_hash: Option<String>,
}

/// `GET /api/v1/indexerstats` — the stats object wraps per-indexer counters under
/// `indexers` (and user-agent breakdowns under `userAgents`, omitted here).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct IndexerStatsResponse {
    #[serde(default)]
    pub indexers: Vec<IndexerStat>,
}

/// A per-indexer counter row inside [`IndexerStatsResponse`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IndexerStat {
    pub indexer_id: Option<i64>,
    pub indexer_name: Option<String>,
    pub number_of_queries: Option<i64>,
    pub number_of_grabs: Option<i64>,
    pub number_of_rss_queries: Option<i64>,
    pub number_of_auth_queries: Option<i64>,
    pub number_of_failed_queries: Option<i64>,
    pub number_of_failed_grabs: Option<i64>,
    pub average_response_time: Option<f64>,
}

#[cfg(test)]
#[path = "indexer_tests.rs"]
mod tests;
