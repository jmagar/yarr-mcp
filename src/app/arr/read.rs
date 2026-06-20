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
use serde_json::{Map, Value, json};

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

/// Fields kept for paged ArrManager records. Queue/history/wanted records can
/// carry large nested release/custom-format/download metadata; agents usually
/// need the status, title, quality, monitored state, and import messages.
pub(crate) const QUEUE_FIELDS: &[&str] = &[
    "page",
    "pageSize",
    "totalRecords",
    "records",
    "id",
    "title",
    "status",
    "monitored",
    "movie",
    "series",
    "episode",
    "quality",
    "languages",
    "size",
    "sizeleft",
    "timeleft",
    "trackedDownloadStatus",
    "trackedDownloadState",
    "statusMessages",
    "errorMessage",
    "downloadClient",
    "indexer",
    "eventType",
    "date",
];

/// Default `pageSize` pushed down to the *arr paged endpoints (`wanted/missing`,
/// `queue`, `history`) so a huge library is not fully materialised upstream and
/// then byte-truncated by the token limiter (P2-7). The *arr v1/v3 paging APIs
/// support `?page=`/`?pageSize=`; callers that need more can page explicitly via
/// the generic `api_get` passthrough.
const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_LIST_LIMIT: usize = 500;

/// Response-shaping options for the ArrManager `list` command.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArrListOptions {
    /// Maximum number of slimmed rows to return. `None` uses
    /// [`DEFAULT_PAGE_SIZE`]; `Some(0)` returns a summary-only response.
    pub limit: Option<usize>,
    /// Number of rows to skip before taking returned `items`.
    pub offset: usize,
    /// Optional item fields to return. The summary is always computed from the
    /// full upstream rows before item field selection is applied.
    pub fields: Vec<String>,
}

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
    /// radarr, …), returning a bounded summary envelope instead of an unbounded
    /// array that would trip the MCP token cap for real libraries.
    pub async fn arr_list(&self, service: &str, options: ArrListOptions) -> Result<Value> {
        let config = self.arr_context(service)?;
        let path = arr_path(config.kind, arr_resource_noun(config.kind));
        let raw = self.client_ref().get_json(config, &path).await?;
        let profiles_path = arr_path(config.kind, "qualityprofile");
        let profiles = self.client_ref().get_json(config, &profiles_path).await?;
        Ok(shape_arr_list(config.kind, raw, profiles, options))
    }

    /// GET `{prefix}/wanted/missing` — items the manager is monitoring but has
    /// not yet acquired. Capped to `DEFAULT_PAGE_SIZE` rows via `?pageSize=` so a
    /// large library is paged upstream rather than fully fetched then truncated
    /// (P2-7). For more rows, page explicitly through the generic `api_get`.
    pub async fn arr_wanted(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let raw = self.arr_paged_get(config, "wanted/missing").await?;
        Ok(slim_paged_records(raw, QUEUE_FIELDS))
    }

    /// GET `{prefix}/queue` — the current download/import queue. Capped to
    /// `DEFAULT_PAGE_SIZE` rows via `?pageSize=` (P2-7).
    pub async fn arr_queue(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let raw = self.arr_paged_get(config, "queue").await?;
        Ok(slim_paged_records(raw, QUEUE_FIELDS))
    }

    /// GET `{prefix}/history` — recent grab/import/delete events. Capped to
    /// `DEFAULT_PAGE_SIZE` rows via `?pageSize=` (P2-7); the generic passthrough
    /// remains available for explicit paging/filters.
    pub async fn arr_history(&self, service: &str) -> Result<Value> {
        let config = self.arr_context(service)?;
        let raw = self.arr_paged_get(config, "history").await?;
        Ok(slim_paged_records(raw, QUEUE_FIELDS))
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

/// Shape a raw Sonarr/Radarr library array into a bounded, agent-friendly
/// response with exact summary counts plus a paged item slice.
pub(crate) fn shape_arr_list(
    kind: crate::config::ServiceKind,
    raw: Value,
    profiles: Value,
    options: ArrListOptions,
) -> Value {
    let items = raw.as_array().cloned().unwrap_or_default();
    let total = items.len();
    let limit = options
        .limit
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .min(MAX_LIST_LIMIT);
    let offset = options.offset.min(total);
    let returned_items: Vec<Value> = items
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .map(|item| slim_list_item(item, &options.fields))
        .collect();
    let returned = returned_items.len();

    json!({
        "total": total,
        "offset": offset,
        "limit": limit,
        "returned": returned,
        "has_more": offset + returned < total,
        "summary": arr_list_summary(kind, &items, &profiles),
        "items": returned_items,
    })
}

fn slim_list_item(item: Value, requested_fields: &[String]) -> Value {
    if requested_fields.is_empty() {
        return slim(item, LIST_FIELDS);
    }

    match item {
        Value::Object(mut map) => {
            let mut kept = Map::new();
            for field in requested_fields {
                if let Some(value) = map.remove(field) {
                    kept.insert(field.clone(), value);
                }
            }
            Value::Object(kept)
        }
        other => other,
    }
}

pub(crate) fn slim_paged_records(value: Value, keep_fields: &[&str]) -> Value {
    match value {
        Value::Object(mut map) => {
            if let Some(records) = map.remove("records") {
                map.insert("records".into(), slim(records, keep_fields));
            }
            slim(Value::Object(map), keep_fields)
        }
        other => slim(other, keep_fields),
    }
}

fn arr_list_summary(kind: crate::config::ServiceKind, items: &[Value], profiles: &Value) -> Value {
    let profile_names = profile_names_by_id(profiles);
    let mut profile_counts: std::collections::BTreeMap<i64, usize> =
        std::collections::BTreeMap::new();
    let mut monitored = 0usize;
    let mut unmonitored = 0usize;
    let mut missing_items = 0usize;
    let mut size_on_disk = 0u64;
    let mut episode_count = 0u64;
    let mut episode_file_count = 0u64;

    for item in items {
        if let Some(id) = item.get("qualityProfileId").and_then(Value::as_i64) {
            *profile_counts.entry(id).or_default() += 1;
        }
        match item.get("monitored").and_then(Value::as_bool) {
            Some(true) => monitored += 1,
            Some(false) => unmonitored += 1,
            None => {}
        }
        if let Some(size) = item.get("sizeOnDisk").and_then(Value::as_u64) {
            size_on_disk += size;
            if kind == crate::config::ServiceKind::Radarr && size == 0 {
                missing_items += 1;
            }
        }
        if let Some(stats) = item.get("statistics") {
            let episodes = stats
                .get("episodeCount")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let files = stats
                .get("episodeFileCount")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            episode_count += episodes;
            episode_file_count += files;
            if kind == crate::config::ServiceKind::Sonarr && episodes > files {
                missing_items += 1;
            }
        }
    }

    let mut by_quality_profile: Vec<Value> = profile_counts
        .into_iter()
        .map(|(id, count)| {
            json!({
                "id": id,
                "name": profile_names
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| format!("id:{id}")),
                "count": count,
            })
        })
        .collect();
    by_quality_profile.sort_by(|a, b| {
        b.get("count")
            .and_then(Value::as_u64)
            .cmp(&a.get("count").and_then(Value::as_u64))
            .then_with(|| {
                a.get("name")
                    .and_then(Value::as_str)
                    .cmp(&b.get("name").and_then(Value::as_str))
            })
    });

    let missing_episodes = episode_count.saturating_sub(episode_file_count);
    json!({
        "monitored": monitored,
        "unmonitored": unmonitored,
        "missing_items": missing_items,
        "missing_episodes": missing_episodes,
        "episode_count": episode_count,
        "episode_file_count": episode_file_count,
        "size_on_disk": size_on_disk,
        "by_quality_profile": by_quality_profile,
    })
}

fn profile_names_by_id(profiles: &Value) -> std::collections::BTreeMap<i64, String> {
    profiles
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|profile| {
            let id = profile.get("id").and_then(Value::as_i64)?;
            let name = profile.get("name").and_then(Value::as_str)?;
            Some((id, name.to_owned()))
        })
        .collect()
}
