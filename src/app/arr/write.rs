//! ArrManager (Sonarr/Radarr) curated WRITE/intent command methods (C2).
//!
//! The mutating counterparts to the C1 read commands: `set_quality` (the
//! headline name-based bulk quality-profile change), `search`, `refresh`,
//! `monitor`/`unmonitor`, `add`, and `delete`. All business logic lives here —
//! the registry descriptors (`src/actions/commands/arr.rs`) and CLI parse hook
//! (`src/cli/commands/arr.rs`) are thin shims that only marshal params.
//!
//! Pure builders + selectors (body shapes, selection, count cap, preview) live
//! in the sibling [`super::editor`] module so this file stays under the LOC cap;
//! these methods orchestrate them around the shared transport.
//!
//! Capability-wide safety contract (security S3 / agent-native AN-4):
//!   * **Dry-run by default** — when `confirm` is absent, NO upstream mutation is
//!     issued; each command returns a structured `would_do` preview.
//!   * **Count cap** — refuse > [`super::editor::MAX_BULK`] items per call unless
//!     `bulk=true` (enforced in the business layer, not the shim).
//!   * **Destructive deletes** are always confirm-gated.
//!
//! `*arr` API facts (best-practices FACT, bead rustarr-zha.7): there is NO bulk
//! `PUT /qualityprofile/editor` — resolve name→id then `PUT /<res>/editor` with
//! `{seriesIds|movieIds, qualityProfileId}`; `/command` is async fire-and-forget
//! with CASE-SENSITIVE command names; the editor id key is `{resource_noun}Ids`.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::app::arr::editor::{
    build_add_body, command_body, editor_id_key_singular, editor_monitor_body, editor_quality_body,
    guard_count, refresh_command_name, row_title, search_command_name, select_all, select_by_ids,
    select_by_profile, select_by_titles, set_quality_preview, urlencode, Selection,
};
use crate::app::arr::read::{arr_path, arr_resource_noun};
use crate::app::RustarrService;
use crate::capability::Capability;
use crate::config::ServiceConfig;

/// Parameters for [`RustarrService::arr_set_quality`], bundled so the headline
/// command keeps a single, readable signature (selectors + safety flags). Borrows
/// its string/slice inputs from the caller's marshalled params.
#[derive(Debug)]
pub struct SetQualityRequest<'a> {
    /// Optional source profile NAME: select items currently on it.
    pub from: Option<&'a str>,
    /// Target profile NAME to move selected items to (required).
    pub to: &'a str,
    /// Explicit resource ids (highest-precedence selector).
    pub ids: &'a [i64],
    /// Resource titles to select (case-insensitive).
    pub titles: &'a [String],
    /// Apply (`true`) vs dry-run preview (`false`).
    pub confirm: bool,
    /// Override the count cap for > `MAX_BULK` items.
    pub bulk: bool,
}

impl RustarrService {
    /// Resolve + capability-check an ArrManager service for a write command.
    fn arr_write_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::ArrManager)
    }

    /// Fetch the primary resource collection (`series`/`movie`) for selection.
    async fn arr_resource_rows(&self, config: &ServiceConfig) -> Result<Vec<Value>> {
        let path = arr_path(config.kind, arr_resource_noun(config.kind));
        let raw = self.client_ref().get_json(config, &path).await?;
        Ok(raw.as_array().cloned().unwrap_or_default())
    }

    /// The `/<res>/editor` path for the configured kind.
    fn editor_path(config: &ServiceConfig) -> String {
        arr_path(
            config.kind,
            &format!("{}/editor", arr_resource_noun(config.kind)),
        )
    }

    /// Select items for a `set_quality` call from the resolved selectors.
    ///
    /// Precedence: explicit `ids` win; else `titles` (case-insensitive, trimmed)
    /// pick matching rows; else if `from_profile_id` is given, every item
    /// currently on that profile; else ALL items. Title misses surface a teaching
    /// error so the caller can correct the name.
    async fn select_quality_items(
        &self,
        config: &ServiceConfig,
        ids: &[i64],
        titles: &[String],
        from_profile_id: Option<i64>,
    ) -> Result<Selection> {
        let rows = self.arr_resource_rows(config).await?;
        if !ids.is_empty() {
            return Ok(select_by_ids(&rows, ids));
        }
        if !titles.is_empty() {
            return select_by_titles(&rows, titles);
        }
        if let Some(from_id) = from_profile_id {
            return Ok(select_by_profile(&rows, from_id));
        }
        Ok(select_all(&rows))
    }

    /// Headline command: change the quality profile of selected sonarr/radarr
    /// items by PROFILE NAME. Resolves `to` (and optional `from`) names→ids via
    /// the C1 resolver, selects items, and either previews (no `confirm`) or
    /// applies via `PUT /<res>/editor` and returns a concise summary.
    pub async fn arr_set_quality(
        &self,
        service: &str,
        req: SetQualityRequest<'_>,
    ) -> Result<Value> {
        let SetQualityRequest {
            from,
            to,
            ids,
            titles,
            confirm,
            bulk,
        } = req;
        let config = self.arr_write_context(service)?;
        // Resolve target (required) and optional source profile names → ids.
        let to_id = self.arr_resolve_quality_profile_id(service, to).await?;
        let from_id = match from {
            Some(name) => Some(self.arr_resolve_quality_profile_id(service, name).await?),
            None => None,
        };
        let selection = self
            .select_quality_items(config, ids, titles, from_id)
            .await?;
        guard_count(selection.len(), bulk)?;

        if !confirm {
            return Ok(set_quality_preview(
                service, from, from_id, to, to_id, &selection,
            ));
        }
        if selection.len() == 0 {
            return Ok(json!({
                "changed": 0,
                "from": from,
                "to": to,
                "note": "no items matched the selection",
            }));
        }
        let body = editor_quality_body(config.kind, &selection.ids, to_id);
        // Apply; we discard the raw editor blobs and return a concise summary.
        self.client_ref()
            .put_json(config, &Self::editor_path(config), body)
            .await?;
        Ok(json!({ "changed": selection.len(), "from": from, "to": to }))
    }

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
        let config = self.arr_write_context(service)?;
        let name = search_command_name(config.kind);
        guard_count(ids.len(), bulk)?;
        if !confirm {
            return Ok(command_preview(
                "search",
                service,
                name,
                ids,
                "all-monitored",
            ));
        }
        let body = command_body(name, &editor_id_key_singular(config.kind), ids);
        let started = self
            .client_ref()
            .post_json(config, &arr_path(config.kind, "command"), body)
            .await?;
        Ok(started_job("search", name, &started))
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
        let config = self.arr_write_context(service)?;
        let name = refresh_command_name(config.kind);
        guard_count(ids.len(), bulk)?;
        if !confirm {
            return Ok(command_preview("refresh", service, name, ids, "all"));
        }
        let body = command_body(name, &editor_id_key_singular(config.kind), ids);
        let started = self
            .client_ref()
            .post_json(config, &arr_path(config.kind, "command"), body)
            .await?;
        Ok(started_job("refresh", name, &started))
    }

    /// Toggle the `monitored` flag on selected items via `PUT /<res>/editor`.
    /// `monitored` selects monitor vs unmonitor; selection by `ids` or `titles`,
    /// default ALL. Count-capped + confirm-gated.
    pub async fn arr_set_monitored(
        &self,
        service: &str,
        ids: &[i64],
        titles: &[String],
        monitored: bool,
        confirm: bool,
        bulk: bool,
    ) -> Result<Value> {
        let config = self.arr_write_context(service)?;
        let rows = self.arr_resource_rows(config).await?;
        let selection = if !ids.is_empty() {
            select_by_ids(&rows, ids)
        } else if !titles.is_empty() {
            select_by_titles(&rows, titles)?
        } else {
            select_all(&rows)
        };
        guard_count(selection.len(), bulk)?;
        let verb = if monitored { "monitor" } else { "unmonitor" };
        if !confirm {
            return Ok(json!({
                "would_do": verb,
                "service": service,
                "count": selection.len(),
                "sample_titles": selection.sample(10),
                "confirm_required": true,
                "hint": "re-run with confirm=true to apply",
            }));
        }
        if selection.len() == 0 {
            return Ok(json!({ "changed": 0, "monitored": monitored }));
        }
        let body = editor_monitor_body(config.kind, &selection.ids, monitored);
        self.client_ref()
            .put_json(config, &Self::editor_path(config), body)
            .await?;
        Ok(json!({ "changed": selection.len(), "monitored": monitored }))
    }

    /// Add a new item: look it up by `term`, then `POST /<res>` with the chosen
    /// quality profile (resolved name→id) and root folder. Kept minimal — the
    /// first lookup hit is used. Confirm-gated; preview shows the resolved match.
    pub async fn arr_add(
        &self,
        service: &str,
        term: &str,
        quality_profile: &str,
        root_folder: &str,
        confirm: bool,
    ) -> Result<Value> {
        let config = self.arr_write_context(service)?;
        let profile_id = self
            .arr_resolve_quality_profile_id(service, quality_profile)
            .await?;
        let lookup_path = format!(
            "{}/lookup?term={}",
            arr_path(config.kind, arr_resource_noun(config.kind)),
            urlencode(term)
        );
        let hits = self.client_ref().get_json(config, &lookup_path).await?;
        let first = hits
            .as_array()
            .and_then(|a| a.first())
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "no `{}` lookup match for term `{term}`",
                    arr_resource_noun(config.kind)
                )
            })?;
        let title = row_title(&first);
        if !confirm {
            return Ok(json!({
                "would_do": "add",
                "service": service,
                "match": { "title": title },
                "quality_profile": { "name": quality_profile, "id": profile_id },
                "root_folder": root_folder,
                "confirm_required": true,
                "hint": "re-run with confirm=true to add",
            }));
        }
        let body = build_add_body(&first, profile_id, root_folder);
        let added = self
            .client_ref()
            .post_json(
                config,
                &arr_path(config.kind, arr_resource_noun(config.kind)),
                body,
            )
            .await?;
        Ok(json!({
            "added": title,
            "id": added.get("id").cloned().unwrap_or(Value::Null),
            "quality_profile": quality_profile,
        }))
    }

    /// Delete an item by id via `DELETE /<res>/{id}?deleteFiles=<bool>`. File
    /// deletion is opt-in (`delete_files`); always confirm-gated and (because it
    /// is destructive) previews before applying.
    pub async fn arr_delete(
        &self,
        service: &str,
        id: i64,
        delete_files: bool,
        confirm: bool,
    ) -> Result<Value> {
        let config = self.arr_write_context(service)?;
        let path = format!(
            "{}/{id}?deleteFiles={delete_files}",
            arr_path(config.kind, arr_resource_noun(config.kind))
        );
        if !confirm {
            return Ok(json!({
                "would_do": "delete",
                "service": service,
                "id": id,
                "delete_files": delete_files,
                "destructive": true,
                "confirm_required": true,
                "hint": "re-run with confirm=true to delete",
            }));
        }
        self.client_ref().delete_json(config, &path, None).await?;
        Ok(json!({ "deleted": id, "delete_files": delete_files }))
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
#[path = "write_tests.rs"]
mod tests;
