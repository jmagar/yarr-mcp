//! ArrManager (Sonarr/Radarr/Lidarr/Readarr) curated READ commands.
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
use crate::rustarr::slim;

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

/// Build `{api_prefix}/{suffix}` for an ArrManager kind. Pure (no `self`) so the
/// per-kind path mapping (sonarr/radarr `/api/v3`, lidarr/readarr `/api/v1`) is
/// unit-testable without a live service.
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
    fn arr_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::ArrManager)
    }

    /// GET `{prefix}/qualityprofile` — the configured quality profiles. Used to
    /// map a profile name<->id. Not slimmed (profiles are small and a caller needs
    /// the id/name pairing).
    pub async fn arr_quality_profiles(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, "qualityprofile");
        self.client_ref().get_json(config, &path).await
    }

    /// GET the primary resource collection (`series` for sonarr, `movie` for
    /// radarr, …), slimmed to [`LIST_FIELDS`].
    pub async fn arr_list(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, arr_resource_noun(config.kind));
        let raw = self.client_ref().get_json(config, &path).await?;
        Ok(slim(raw, LIST_FIELDS))
    }

    /// GET `{prefix}/wanted/missing` — items the manager is monitoring but has
    /// not yet acquired. Returned as-is (the upstream paginates).
    pub async fn arr_wanted(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, "wanted/missing");
        self.client_ref().get_json(config, &path).await
    }

    /// GET `{prefix}/queue` — the current download/import queue.
    pub async fn arr_queue(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, "queue");
        self.client_ref().get_json(config, &path).await
    }

    /// GET `{prefix}/history` — recent grab/import/delete events (upstream
    /// paginates; the generic passthrough remains available for filters).
    pub async fn arr_history(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, "history");
        self.client_ref().get_json(config, &path).await
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
