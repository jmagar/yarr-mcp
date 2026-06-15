//! Requests capability (Overseerr) curated commands (C7).
//!
//! Overseerr is the sole [`Capability::Requests`](crate::capability::Capability)
//! kind. Each method resolves the api prefix (`/api/v1`) from the service's
//! [`KindDescriptor`](crate::capability::KindDescriptor) — descriptor-driven, no
//! hard-coded version — then issues the request through the shared transport and
//! slims read payloads to the fields agents actually need (AN-6 context budget).
//!
//! Scope split (locked in the bead): `req_list` and `req_search` are READ;
//! `req_create`, `req_approve`, and `req_decline` MUTATE, so they are WRITE +
//! confirm-gated. Resource-noun/path resolution, request-body shape, and
//! field-selection are *business* decisions and live here, never in a shim.
//!
//! Permissions note: `req_approve` / `req_decline` hit Overseerr's
//! `MANAGE_REQUESTS`-gated endpoints — they succeed only with an admin API key;
//! a user-scoped key returns 403. This is documented in the command help text.

use anyhow::Result;
use serde_json::{Value, json};

use crate::app::RustarrService;
use crate::app::util::urlencode;
use crate::capability::Capability;
use crate::config::ServiceConfig;
use crate::rustarr::slim;

/// Fields kept for a slimmed `request` row: enough to identify a request and
/// reason about its state without the full (large) media/user payloads.
const REQUEST_FIELDS: &[&str] = &["id", "type", "status", "media", "requestedBy"];

/// Fields kept for a slimmed `search` result: enough to pick a title to request
/// (the `id` here is the TMDB id passed back into `req_create`).
const SEARCH_FIELDS: &[&str] = &[
    "id",
    "mediaType",
    "title",
    "name",
    "releaseDate",
    "firstAirDate",
    "overview",
];

impl RustarrService {
    /// Resolve a Requests service and verify its capability. Central helper so
    /// every request method shares one capability-checked resolution path; a
    /// non-overseerr kind is rejected here before any request is built.
    fn requests_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::Requests)
    }

    /// Build `{api_prefix}/{suffix}` for the resolved Requests service.
    fn requests_path(config: &ServiceConfig, suffix: &str) -> String {
        format!("{}/{}", config.kind.descriptor().api_prefix, suffix)
    }

    /// GET `{prefix}/request?filter=&take=&skip=` — the request list, slimmed to
    /// [`REQUEST_FIELDS`]. `filter` (e.g. `pending`/`approved`/`available`),
    /// `take`, and `skip` are optional pagination/selection knobs.
    pub async fn req_list(
        &self,
        service: &str,
        filter: Option<&str>,
        take: Option<i64>,
        skip: Option<i64>,
    ) -> Result<Value> {
        let config = self.requests_context(service)?;
        let mut path = Self::requests_path(config, "request");
        let mut params: Vec<String> = Vec::new();
        if let Some(filter) = filter {
            params.push(format!("filter={}", urlencode(filter)));
        }
        if let Some(take) = take {
            params.push(format!("take={take}"));
        }
        if let Some(skip) = skip {
            params.push(format!("skip={skip}"));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }
        let raw = self.client_ref().get_json(config, &path).await?;
        // Overseerr returns `{ pageInfo, results: [...] }`; slim the `results`
        // array in place and keep the rest of the envelope.
        let slimmed = match raw {
            Value::Object(mut map) => {
                if let Some(results) = map.remove("results") {
                    map.insert("results".into(), slim(results, REQUEST_FIELDS));
                }
                Value::Object(map)
            }
            other => slim(other, REQUEST_FIELDS),
        };
        Ok(slimmed)
    }

    /// POST `{prefix}/request` — create a request. Body is
    /// `{mediaType, mediaId, seasons?}` where `mediaId` is the TMDB id and
    /// `seasons` (TV only) is an optional list of season numbers. MUTATES +
    /// confirm-gated; the confirm check runs here so the CLI and MCP paths share
    /// one guard.
    pub async fn req_create(
        &self,
        service: &str,
        media_type: &str,
        media_id: i64,
        seasons: &[i64],
        confirm: bool,
    ) -> Result<Value> {
        if !confirm {
            anyhow::bail!(
                "request create submits a new media request (write); pass confirm=true (CLI --confirm) to run it"
            );
        }
        let config = self.requests_context(service)?;
        let path = Self::requests_path(config, "request");
        let mut body = json!({ "mediaType": media_type, "mediaId": media_id });
        if !seasons.is_empty() {
            body["seasons"] = json!(seasons);
        }
        self.client_ref().post_json(config, &path, body).await
    }

    /// POST `{prefix}/request/{id}/approve` — approve a pending request.
    /// MUTATES + confirm-gated. Requires the `MANAGE_REQUESTS` permission on the
    /// Overseerr API key (admin key); a user-scoped key returns 403.
    pub async fn req_approve(&self, service: &str, id: i64, confirm: bool) -> Result<Value> {
        if !confirm {
            anyhow::bail!(
                "request approve mutates a request (write, needs MANAGE_REQUESTS / admin key); pass confirm=true (CLI --confirm) to run it"
            );
        }
        let config = self.requests_context(service)?;
        let path = Self::requests_path(config, &format!("request/{id}/approve"));
        self.client_ref()
            .post_json(config, &path, Value::Null)
            .await
    }

    /// POST `{prefix}/request/{id}/decline` — decline a pending request.
    /// MUTATES + confirm-gated. Requires the `MANAGE_REQUESTS` permission on the
    /// Overseerr API key (admin key); a user-scoped key returns 403.
    pub async fn req_decline(&self, service: &str, id: i64, confirm: bool) -> Result<Value> {
        if !confirm {
            anyhow::bail!(
                "request decline mutates a request (write, needs MANAGE_REQUESTS / admin key); pass confirm=true (CLI --confirm) to run it"
            );
        }
        let config = self.requests_context(service)?;
        let path = Self::requests_path(config, &format!("request/{id}/decline"));
        self.client_ref()
            .post_json(config, &path, Value::Null)
            .await
    }

    /// GET `{prefix}/search?query=` — multi-search for titles to request, slimmed
    /// to [`SEARCH_FIELDS`]. The `id` in each result is the TMDB id to pass into
    /// [`req_create`](Self::req_create). READ.
    pub async fn req_search(&self, service: &str, query: &str) -> Result<Value> {
        let config = self.requests_context(service)?;
        let path = format!(
            "{}/search?query={}",
            config.kind.descriptor().api_prefix,
            urlencode(query)
        );
        let raw = self.client_ref().get_json(config, &path).await?;
        let slimmed = match raw {
            Value::Object(mut map) => {
                if let Some(results) = map.remove("results") {
                    map.insert("results".into(), slim(results, SEARCH_FIELDS));
                }
                Value::Object(map)
            }
            other => slim(other, SEARCH_FIELDS),
        };
        Ok(slimmed)
    }
}

#[cfg(test)]
#[path = "requests_tests.rs"]
mod tests;
