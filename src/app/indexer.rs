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
use crate::app::util::urlencode;
use crate::capability::Capability;
use crate::config::ServiceConfig;
use crate::rustarr::slim;

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
    /// `INDEXER_FIELDS`.
    pub async fn indexer_list(&self, service: &str) -> Result<Value> {
        let config = self.indexer_context(service)?;
        let path = Self::indexer_path(config, "indexer");
        let raw = self.client_ref().get_json(config, &path).await?;
        Ok(slim(raw, INDEXER_FIELDS))
    }

    /// GET `{prefix}/search?query=&type=search[&indexerIds[]=…]` — a Newznab-style
    /// manual search across indexers. `query` is required; `indexer_ids` restricts
    /// the search to specific indexers (empty searches all). Results are returned
    /// as-is (the caller chooses what to grab).
    pub async fn indexer_search(
        &self,
        service: &str,
        query: &str,
        indexer_ids: &[i64],
    ) -> Result<Value> {
        let config = self.indexer_context(service)?;
        let mut path = format!(
            "{}/search?query={}&type=search",
            config.kind.descriptor().api_prefix,
            urlencode(query)
        );
        for id in indexer_ids {
            path.push_str(&format!("&indexerIds[]={id}"));
        }
        self.client_ref().get_json(config, &path).await
    }

    /// GET `{prefix}/indexerstats` — per-indexer query/grab/failure counters,
    /// slimmed to `STATS_FIELDS` (slims nested `indexers` rows in place).
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
