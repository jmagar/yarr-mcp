//! Indexer capability (Prowlarr) curated commands.
//!
//! Prowlarr is the sole [`Capability::Indexer`](crate::capability::Capability)
//! kind. Each method resolves the api prefix (`/api/v1`) from the service's
//! [`KindDescriptor`](crate::capability::KindDescriptor) — descriptor-driven, no
//! hard-coded version — then issues the request through the shared transport and
//! slims read payloads to the fields agents actually need (AN-6 context budget).
//!
//! Scope split (locked in the bead): `indexer_list`, `indexer_search`, and
//! `indexer_stats` are READ; `indexer_test` TRIGGERS an indexer health-check
//! command, so it is WRITE + confirm-gated. Resource-noun/path resolution and
//! field-selection are *business* decisions and live here, never in a shim.

use anyhow::Result;
use serde_json::Value;

use crate::app::RustarrService;
use crate::capability::Capability;
use crate::config::ServiceConfig;
use crate::rustarr::{query_get, slim};

/// Default cap on `indexer_search` results. Prowlarr's Newznab search endpoint
/// honours `limit`, so we page at the API instead of fetching every hit and then
/// byte-truncating the payload downstream (P2-7). Large libraries can return
/// thousands of releases; 100 is plenty for an agent to pick a grab from.
const SEARCH_RESULT_LIMIT: i64 = 100;

/// Fields kept for a slimmed `indexers` row: enough to identify an indexer and
/// reason about routing without dragging the full (large) definition payload.
const INDEXER_FIELDS: &[&str] = &["id", "name", "enable", "protocol", "priority"];

/// Fields kept for a slimmed `stats` indexer entry. Prowlarr's `indexerstats`
/// returns `{ indexers: [...], userAgents: [...], ... }`; we slim the indexer
/// rows to the counters an agent reasons about (success/failure/grabs/queries).
const STATS_FIELDS: &[&str] = &[
    "indexerId",
    "indexerName",
    "numberOfQueries",
    "numberOfGrabs",
    "numberOfRssQueries",
    "numberOfAuthQueries",
    "numberOfFailedQueries",
    "numberOfFailedGrabs",
    "averageResponseTime",
];

impl RustarrService {
    /// Resolve an Indexer service and verify its capability. Central helper so
    /// every indexer method shares one capability-checked resolution path; a
    /// non-prowlarr kind is rejected here before any request is built.
    fn indexer_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::Indexer)
    }

    /// Build `{api_prefix}/{suffix}` for the resolved Indexer service.
    fn indexer_path(config: &ServiceConfig, suffix: &str) -> String {
        format!("{}/{}", config.kind.descriptor().api_prefix, suffix)
    }

    /// GET `{prefix}/indexer` — the configured indexers, slimmed to
    /// [`INDEXER_FIELDS`].
    pub async fn indexer_list(&self, service: &str) -> Result<Value> {
        let config = self.indexer_context(service)?;
        let path = Self::indexer_path(config, "indexer");
        let raw = self.client_ref().get_json(config, &path).await?;
        Ok(slim(raw, INDEXER_FIELDS))
    }

    /// GET `{prefix}/search?query=&type=search&limit=N[&indexerIds[]=…]` — a
    /// Newznab-style manual search across indexers. `query` is required;
    /// `indexer_ids` restricts the search to specific indexers (empty searches
    /// all). Results are capped at [`SEARCH_RESULT_LIMIT`] (paged at the Prowlarr
    /// API, P2-7) so a broad query can't fetch thousands of releases only to be
    /// byte-truncated; the caller chooses what to grab from the capped set.
    pub async fn indexer_search(
        &self,
        service: &str,
        query: &str,
        indexer_ids: &[i64],
    ) -> Result<Value> {
        let config = self.indexer_context(service)?;
        let base_path = Self::indexer_path(config, "search");
        // S6: every value (including the non-injectable i64 ids) flows through the
        // percent-encoding `query_get` builder — never `format!`'d into the query.
        let limit = SEARCH_RESULT_LIMIT.to_string();
        let mut params: Vec<(&str, &str)> = vec![
            ("query", query),
            ("type", "search"),
            ("limit", limit.as_str()),
        ];
        // Owned strings for the i64 ids so we can pass `&str` pairs; Prowlarr
        // accepts the repeated `indexerIds` key (url-encoded `[]` suffix).
        let id_strings: Vec<String> = indexer_ids.iter().map(|id| id.to_string()).collect();
        for id in &id_strings {
            params.push(("indexerIds[]", id.as_str()));
        }
        let url = query_get(config, &base_path, &params)?;
        self.client_ref().send_get(config, url, None).await
    }

    /// GET `{prefix}/indexerstats` — per-indexer query/grab/failure counters,
    /// slimmed to [`STATS_FIELDS`] (slims nested `indexers` rows in place).
    pub async fn indexer_stats(&self, service: &str) -> Result<Value> {
        let config = self.indexer_context(service)?;
        let path = Self::indexer_path(config, "indexerstats");
        let raw = self.client_ref().get_json(config, &path).await?;
        // `indexerstats` is an object; slim its `indexers` array if present,
        // otherwise slim whatever array shape was returned.
        let slimmed = match raw {
            Value::Object(mut map) => {
                if let Some(indexers) = map.remove("indexers") {
                    map.insert("indexers".into(), slim(indexers, STATS_FIELDS));
                }
                Value::Object(map)
            }
            other => slim(other, STATS_FIELDS),
        };
        Ok(slimmed)
    }

    /// POST a health-check trigger for indexers.
    ///
    /// Servarr/Prowlarr has NO `{prefix}/indexer/{id}/test` route. The correct
    /// shapes are:
    ///   * test ALL -> `POST {prefix}/indexer/testall`.
    ///   * test ONE -> `GET {prefix}/indexer/{id}` to fetch the indexer definition,
    ///     then `POST {prefix}/indexer/test` with that body (the test endpoint
    ///     validates a full indexer payload, not an id in the path).
    ///
    /// This TRIGGERS a command, so it mutates and is confirm-gated by the
    /// descriptor; the confirm check runs here too so the CLI and MCP paths share
    /// one guard.
    pub async fn indexer_test(
        &self,
        service: &str,
        id: Option<i64>,
        confirm: bool,
    ) -> Result<Value> {
        if !confirm {
            anyhow::bail!(
                "indexer test triggers an indexer health check (write); pass confirm=true (CLI --confirm) to run it"
            );
        }
        let config = self.indexer_context(service)?;
        match id {
            None => {
                let path = Self::indexer_path(config, "indexer/testall");
                self.client_ref()
                    .post_json(config, &path, Value::Null)
                    .await
            }
            Some(id) => {
                // Fetch the indexer definition, then POST it to the test endpoint.
                let get_path = Self::indexer_path(config, &format!("indexer/{id}"));
                let definition = self.client_ref().get_json(config, &get_path).await?;
                let test_path = Self::indexer_path(config, "indexer/test");
                self.client_ref()
                    .post_json(config, &test_path, definition)
                    .await
            }
        }
    }
}

#[cfg(test)]
#[path = "indexer_tests.rs"]
mod tests;
