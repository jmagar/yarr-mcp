//! ArrManager (Sonarr/Radarr/Lidarr/Readarr) async `/command`-intent methods (C2).
//!
//! Split out of `write.rs` (P2-2) so the editor-based mutations and these
//! fire-and-forget `/command` intents each stay well under the LOC cap. These are
//! the search/refresh verbs: they `POST /<prefix>/command` and return the started
//! job(s) without polling (the *arr `/command` API is async fire-and-forget).
//!
//! Capability-wide safety contract is identical to the editor writes (see
//! [`super::write`]): dry-run by default, count-capped, confirm-gated.
//!
//! `*arr` API facts (best-practices FACT): `/command` names are CASE-SENSITIVE and
//! only Radarr accepts a PLURAL `{noun}Ids` batch in one POST. Sonarr/Lidarr/Readarr
//! have NO plural form, so multi-id search/refresh fans out to one POST per id — run
//! with bounded concurrency (P2-6) since these are single-instance home services.

use anyhow::Result;
use serde_json::{Value, json};
use tokio::task::JoinSet;

use crate::app::RustarrService;
use crate::app::arr::editor::{
    command_body_plural, command_body_single, editor_id_key_singular,
    kind_command_supports_plural_ids, refresh_command_name, search_command_name,
};
use crate::app::arr::read::arr_path;
use crate::config::ServiceConfig;

/// Max number of in-flight `/command` POSTs for the singular per-id fan-out
/// (P2-6). Kept modest — these wrap single-instance home services (Sonarr et al.),
/// so wall-time becomes ~⌈N/[`FANOUT_CONCURRENCY`]⌉×RTT without hammering the box.
const FANOUT_CONCURRENCY: usize = 8;

impl RustarrService {
    /// Start an async search job via `POST /command`. With no selector it searches
    /// the whole monitored library; with `ids` it searches those items. Returns
    /// the started job; does NOT poll (the `/command` API is fire-and-forget).
    pub async fn arr_search(
        &self,
        service: &str,
        ids: &[i64],
        confirm: bool,
        bulk: bool,
    ) -> Result<Value> {
        let config = self.arr_context(service)?;
        let name = search_command_name(config.kind);
        // Explicit ids are capped up-front (cheap, no network). The dry-run preview
        // is network-free, so the whole-library size is NOT fetched here.
        if !ids.is_empty() {
            super::editor::guard_count(ids.len(), bulk)?;
        }
        if !confirm {
            return Ok(command_preview(
                "search",
                service,
                name,
                ids,
                "all-monitored",
            ));
        }
        // Apply path: a whole-library (empty ids) search must still respect the
        // cap, so count the real library size before mutating. L3-perf: this GET
        // pulls the full resource collection only to count rows for the cap — if a
        // future *arr exposes a cheap `count`/`total` endpoint, prefer it here to
        // avoid materialising the whole library just to enforce MAX_BULK.
        if ids.is_empty() {
            super::editor::guard_count(self.arr_resource_row_count(config).await?, bulk)?;
        }
        self.run_arr_command("search", config, name, ids).await
    }

    /// Start an async refresh/rescan job via `POST /command`. Same async contract
    /// as [`arr_search`](Self::arr_search).
    pub async fn arr_refresh(
        &self,
        service: &str,
        ids: &[i64],
        confirm: bool,
        bulk: bool,
    ) -> Result<Value> {
        let config = self.arr_context(service)?;
        let name = refresh_command_name(config.kind);
        // Explicit ids are capped up-front (cheap, no network). The dry-run preview
        // is network-free, so the whole-library size is NOT fetched here.
        if !ids.is_empty() {
            super::editor::guard_count(ids.len(), bulk)?;
        }
        if !confirm {
            return Ok(command_preview("refresh", service, name, ids, "all"));
        }
        // Apply path: a whole-library (empty ids) refresh must still respect the
        // cap, so count the real library size before mutating. L3-perf: cap-only
        // fetch — see the note in `arr_search`; a cheap count endpoint would avoid
        // pulling the whole library just to enforce MAX_BULK.
        if ids.is_empty() {
            super::editor::guard_count(self.arr_resource_row_count(config).await?, bulk)?;
        }
        self.run_arr_command("refresh", config, name, ids).await
    }

    /// Issue an async `/command` job for the search/refresh intents, honouring the
    /// per-kind plural-support rule (Fix 6 / *arr FACT):
    ///
    ///   * No ids -> ONE whole-library POST (`{name}`).
    ///   * Radarr (plural-capable) -> ONE POST with `{name, movieIds:[...]}`.
    ///   * Sonarr/Lidarr/Readarr -> NO plural form, so ONE POST per id with the
    ///     singular `{name, <noun>Id}` body; the started job ids are aggregated.
    ///     These POSTs run with bounded concurrency ([`FANOUT_CONCURRENCY`]) so a
    ///     large multi-id selection finishes in ~⌈N/8⌉ round-trips instead of N,
    ///     while the aggregated jobs/count response shape is preserved EXACTLY
    ///     (jobs stay in caller-id order).
    async fn run_arr_command(
        &self,
        verb: &str,
        config: &ServiceConfig,
        name: &str,
        ids: &[i64],
    ) -> Result<Value> {
        let command_path = arr_path(config.kind, "command");
        let id_key = editor_id_key_singular(config.kind);

        // Whole-library: a single name-only command.
        if ids.is_empty() {
            let body = command_body_single(name, &id_key, None);
            let started = self
                .client_ref()
                .post_json(config, &command_path, body)
                .await?;
            return Ok(started_job(verb, name, &started));
        }

        // Radarr accepts a single batched plural command.
        if kind_command_supports_plural_ids(config.kind) {
            let body = command_body_plural(name, &id_key, ids);
            let started = self
                .client_ref()
                .post_json(config, &command_path, body)
                .await?;
            return Ok(started_job(verb, name, &started));
        }

        // Sonarr/Lidarr/Readarr: no plural form — one POST per id. Fan these out
        // with bounded concurrency (P2-6) rather than awaiting serially. Results
        // are tagged with their input index and re-sorted, so the aggregated
        // `jobs` array stays in the exact caller-id order the serial loop produced.
        let jobs = self
            .run_singular_command_fanout(config, &command_path, name, &id_key, ids)
            .await?;
        Ok(json!({
            "started": verb,
            "command": name,
            "async": true,
            "count": jobs.len(),
            "jobs": jobs,
        }))
    }

    /// Fan out one `POST /command` per id with bounded concurrency, returning the
    /// started job ids in the SAME order as the input `ids`. Any failed POST
    /// aborts the whole batch (matching the serial loop's `?` short-circuit).
    ///
    /// NOTE on partial effects: these are independent fire-and-forget `*arr`
    /// commands, so POSTs that already succeeded before one fails have queued real
    /// jobs upstream — those ids are discarded and the call reports failure. A
    /// naive retry can therefore double-queue. Callers that need to know exactly
    /// which jobs started should re-query the `*arr` queue. (Tracked for a
    /// structured partial-success result; today the contract is fail-fast.)
    async fn run_singular_command_fanout(
        &self,
        config: &ServiceConfig,
        command_path: &str,
        name: &str,
        id_key: &str,
        ids: &[i64],
    ) -> Result<Vec<Value>> {
        // `JoinSet` needs `'static` tasks; clone the cheap (Arc-backed) client and
        // own the small per-task inputs so each POST runs independently.
        let mut results: Vec<Option<Value>> = vec![None; ids.len()];
        let mut next = 0_usize;
        let mut set: JoinSet<(usize, Result<Value>)> = JoinSet::new();

        let spawn_one = |set: &mut JoinSet<(usize, Result<Value>)>, idx: usize| {
            let client = self.client_ref().clone();
            let config = config.clone();
            let path = command_path.to_string();
            let body = command_body_single(name, id_key, Some(ids[idx]));
            set.spawn(async move {
                let res = client.post_json(&config, &path, body).await;
                (idx, res)
            });
        };

        // Prime the window, then refill as each task completes (bounded in-flight).
        while next < ids.len() && set.len() < FANOUT_CONCURRENCY {
            spawn_one(&mut set, next);
            next += 1;
        }
        while let Some(joined) = set.join_next().await {
            let (idx, res) = joined.map_err(|e| anyhow::anyhow!("command task panicked: {e}"))?;
            let started = res?;
            results[idx] = Some(started.get("id").cloned().unwrap_or(Value::Null));
            if next < ids.len() {
                spawn_one(&mut set, next);
                next += 1;
            }
        }

        Ok(results
            .into_iter()
            .map(|j| j.unwrap_or(Value::Null))
            .collect())
    }

    /// Count the rows in the primary resource collection (`series`/`movie`/…) for
    /// cap enforcement on whole-library commands. Thin wrapper over the shared
    /// fetch in [`super::write`]; isolated so the L3-perf cap-only fetch is one
    /// call site (a cheap upstream count endpoint could replace it here).
    async fn arr_resource_row_count(&self, config: &ServiceConfig) -> Result<usize> {
        Ok(self.arr_resource_rows(config).await?.len())
    }
}

/// Dry-run preview for an async `/command` intent (search/refresh).
fn command_preview(verb: &str, service: &str, name: &str, ids: &[i64], all_label: &str) -> Value {
    let count = if ids.is_empty() {
        Value::String(all_label.to_string())
    } else {
        json!(ids.len())
    };
    json!({
        "would_do": verb,
        "service": service,
        "command": name,
        "count": count,
        "confirm_required": true,
        "hint": "re-run with confirm=true to start the async job",
    })
}

/// Concise summary of a started async `/command` job (id only — never the blob).
fn started_job(verb: &str, name: &str, started: &Value) -> Value {
    json!({
        "started": verb,
        "command": name,
        "async": true,
        "job": started.get("id").cloned().unwrap_or(Value::Null),
    })
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
