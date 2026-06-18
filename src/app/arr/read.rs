//! ArrManager (Sonarr/Radarr) curated READ commands.
//!
//! Each method resolves the api prefix and primary resource noun from the
//! service's [`KindDescriptor`](crate::capability::KindDescriptor) — so the same
//! code serves Sonarr (`/api/v3`, `series`) and Radarr (`/api/v3`, `movie`)
//! without per-kind branching — then issues a GET via the shared transport and
//! slims the payload to the fields agents actually need (AN-6 context budget).
//!
//! All commands are READ scope, non-mutating. Resource-noun resolution and
//! field-selection are *business* decisions and live here, never in a shim.

use anyhow::Result;
use serde_json::Value;

use crate::app::RustarrService;
use crate::capability::Capability;
use crate::config::ServiceConfig;
use crate::rustarr::{query_get, slim};

/// Fields kept for a slimmed `list` row across sonarr/radarr. `qualityProfileId`
/// is retained so a caller can pair `list` with `quality_profiles` to choose a
/// profile by id; `status`/`monitored`/`sizeOnDisk`/`added` give a compact
/// library overview.
const LIST_FIELDS: &[&str] = &[
    "id",
    "title",
    "qualityProfileId",
    "monitored",
    "sizeOnDisk",
    "status",
    "added",
];

/// Fields kept for a slimmed quality profile row. Sonarr/Radarr quality profiles
/// include a full nested quality tree; callers usually need the stable id/name
/// pairing plus cutoff/upgrade settings.
const QUALITY_PROFILE_FIELDS: &[&str] = &[
    "id",
    "name",
    "cutoff",
    "cutoffFormatScore",
    "minFormatScore",
    "minUpgradeFormatScore",
    "upgradeAllowed",
];

/// Default `pageSize` pushed down to the *arr paged endpoints (`wanted/missing`,
/// `queue`, `history`) so a huge library is not fully materialised upstream and
/// then byte-truncated by the token limiter (P2-7). The *arr v1/v3 paging APIs
/// support `?page=`/`?pageSize=`; callers that need more can page explicitly via
/// the generic `api_get` passthrough.
const DEFAULT_PAGE_SIZE: usize = 50;

/// Build `{api_prefix}/{suffix}` for an ArrManager kind. Pure (no `self`) so the
/// path mapping is unit-testable without a live service.
pub(crate) fn arr_path(kind: crate::config::ServiceKind, suffix: &str) -> String {
    format!("{}/{}", kind.descriptor().api_prefix, suffix)
}

/// The primary resource noun (`series`/`movie`/…) for an ArrManager kind,
/// defaulting to `series` for any kind without one. Pure for testability.
pub(crate) fn arr_resource_noun(kind: crate::config::ServiceKind) -> &'static str {
    kind.descriptor().resource_noun.unwrap_or("series")
}

impl RustarrService {
    /// Resolve an ArrManager service and verify its capability. Central helper so
    /// every read method shares one capability-checked resolution path; an
    /// incompatible kind is rejected here before any request is built.
    pub(super) fn arr_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::ArrManager)
    }

    /// GET `{prefix}/qualityprofile` — the configured quality profiles, slimmed
    /// to stable identifiers and cutoff/upgrade settings.
    pub async fn arr_quality_profiles(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, "qualityprofile");
        let raw = self.client_ref().get_json(config, &path).await?;
        Ok(slim(raw, QUALITY_PROFILE_FIELDS))
    }

    /// GET the primary resource collection (`series` for sonarr, `movie` for
    /// radarr, …), slimmed to `LIST_FIELDS`.
    pub async fn arr_list(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, arr_resource_noun(config.kind));
        let raw = self.client_ref().get_json(config, &path).await?;
        Ok(slim(raw, LIST_FIELDS))
    }

    /// GET `{prefix}/wanted/missing` — items the manager is monitoring but has
    /// not yet acquired. Capped to [`DEFAULT_PAGE_SIZE`] rows via `?pageSize=` so a
    /// large library is paged upstream rather than fully fetched then truncated
    /// (P2-7). For more rows, page explicitly through the generic `api_get`.
    pub async fn arr_wanted(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        self.arr_paged_get(config, "wanted/missing").await
    }

    /// GET `{prefix}/queue` — the current download/import queue. Capped to
    /// [`DEFAULT_PAGE_SIZE`] rows via `?pageSize=` (P2-7).
    pub async fn arr_queue(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        self.arr_paged_get(config, "queue").await
    }

    /// GET `{prefix}/history` — recent grab/import/delete events. Capped to
    /// [`DEFAULT_PAGE_SIZE`] rows via `?pageSize=` (P2-7); the generic passthrough
    /// remains available for explicit paging/filters.
    pub async fn arr_history(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        self.arr_paged_get(config, "history").await
    }

    /// Issue a GET against an *arr paged endpoint with a default `?pageSize=`
    /// ([`DEFAULT_PAGE_SIZE`]), routing the (fixed, numeric) param through the
    /// percent-encoding `query_get` helper for consistency with the S6 contract.
    async fn arr_paged_get(&self, config: &ServiceConfig, suffix: &str) -> Result<Value> {
        let base = arr_path(config.kind, suffix);
        let page_size = DEFAULT_PAGE_SIZE.to_string();
        let url = query_get(config, &base, &[("pageSize", page_size.as_str())])?;
        self.client_ref().send_get(config, url, None).await
    }

    /// GET `{prefix}/rootfolder` — configured root folders with free/total space.
    pub async fn arr_rootfolders(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, "rootfolder");
        self.client_ref().get_json(config, &path).await
    }

    /// GET `{prefix}/health` — health-check messages. An empty array means
    /// healthy.
    pub async fn arr_health(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, "health");
        self.client_ref().get_json(config, &path).await
    }
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
