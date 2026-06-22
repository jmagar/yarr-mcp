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
//!   * **Count cap** — refuse > [`super::editor::MAX_BULK`] items per call unless
//!     `bulk=true` (enforced in the business layer, not the shim).
//!   * **Destructive deletes** (`delete`) are always confirm-gated and preview by
//!     default; non-destructive writes (`set_quality`, `add`, `monitor`,
//!     `search`, `refresh`) run immediately. In rustarr terminology,
//!     "destructive" is reserved for permanent loss of hard-to-recreate data.
//!
//! `*arr` API facts (best-practices FACT, bead rustarr-zha.7): there is NO bulk
//! `PUT /qualityprofile/editor` — resolve name→id then `PUT /<res>/editor` with
//! `{seriesIds|movieIds, qualityProfileId}`; `/command` is async fire-and-forget
//! with CASE-SENSITIVE command names; the editor id key is `{resource_noun}Ids`.

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::app::RustarrService;
use crate::app::arr::editor::{
    Selection, build_add_body, editor_apply_summary, editor_monitor_body, editor_quality_body,
    guard_count, row_title, select_all, select_by_ids, select_by_profile, select_by_titles,
    value_preview, value_shape,
};
use crate::app::arr::read::{arr_path, arr_resource_noun};
use crate::app::arr::resolve::match_quality_profile_id;
use crate::config::ServiceConfig;
use crate::rustarr::query_get;

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
    /// Override the count cap for > `MAX_BULK` items.
    pub bulk: bool,
}

impl RustarrService {
    /// Fetch the primary resource collection (`series`/`movie`) for selection.
    ///
    /// The `*arr` resource endpoints return a JSON array. A non-array shape means
    /// the upstream returned something unexpected (an error envelope, an HTML
    /// login page, a single object, ...) — we surface that explicitly rather than
    /// coercing it to an empty list, which would masquerade as "nothing in your
    /// whole library matched" and silently no-op a bulk write.
    pub(crate) async fn arr_resource_rows(&self, config: &ServiceConfig) -> Result<Vec<Value>> {
        let path = arr_path(config.kind, arr_resource_noun(config.kind));
        let raw = self.client_ref().get_json(config, &path).await?;
        match raw {
            Value::Array(rows) => Ok(rows),
            other => Err(anyhow!(
                "unexpected response from {} {path}: expected a JSON array of resources, got {}: {}",
                config.name,
                value_shape(&other),
                value_preview(&other),
            )),
        }
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
            return select_by_ids(&rows, ids).map_err(|e| anyhow!("{e} on {}", config.name));
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
    /// the C1 resolver, selects items, and applies via `PUT /<res>/editor`,
    /// returning a concise summary. Mutating but not destructive — runs
    /// immediately (no confirm gate).
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
            bulk,
        } = req;
        let config = self.arr_context(service)?;
        // Resolve target (required) and optional source profile names → ids from a
        // SINGLE fetch of the profile list (P2-5): both names are matched in memory
        // against the same payload instead of issuing one GET /qualityprofile per
        // name.
        let profiles = self.arr_quality_profiles(service).await?;
        let to_id = match_quality_profile_id(&profiles, to)?;
        let from_id = match from {
            Some(name) => Some(match_quality_profile_id(&profiles, name)?),
            None => None,
        };
        let selection = self
            .select_quality_items(config, ids, titles, from_id)
            .await?;
        guard_count(selection.len(), bulk)?;

        if selection.is_empty() {
            return Ok(json!({
                "changed": 0,
                "from": from,
                "to": to,
                "note": "no items matched the selection",
            }));
        }
        let body = editor_quality_body(config.kind, &selection.ids, to_id);
        // Apply. The *arr `/editor` endpoint echoes the updated resource array, so
        // report the upstream-confirmed count rather than fabricating it from the
        // selection length (which would over-report if the server changed fewer).
        let response = self
            .client_ref()
            .put_json(config, &Self::editor_path(config), body)
            .await?;
        let attempted = selection.len();
        Ok(editor_apply_summary(
            &response,
            attempted,
            json!({ "from": from, "to": to }),
        ))
    }

    /// Toggle the `monitored` flag on selected items via `PUT /<res>/editor`.
    /// `monitored` selects monitor vs unmonitor; selection by `ids` or `titles`,
    /// default ALL. Count-capped. Mutating but not destructive — runs immediately
    /// (no confirm gate).
    pub async fn arr_set_monitored(
        &self,
        service: &str,
        ids: &[i64],
        titles: &[String],
        monitored: bool,
        bulk: bool,
    ) -> Result<Value> {
        let config = self.arr_context(service)?;
        let rows = self.arr_resource_rows(config).await?;
        let selection = if !ids.is_empty() {
            select_by_ids(&rows, ids).map_err(|e| anyhow!("{e} on {}", config.name))?
        } else if !titles.is_empty() {
            select_by_titles(&rows, titles)?
        } else {
            select_all(&rows)
        };
        guard_count(selection.len(), bulk)?;
        if selection.is_empty() {
            return Ok(json!({ "changed": 0, "monitored": monitored }));
        }
        let body = editor_monitor_body(config.kind, &selection.ids, monitored);
        let response = self
            .client_ref()
            .put_json(config, &Self::editor_path(config), body)
            .await?;
        let attempted = selection.len();
        Ok(editor_apply_summary(
            &response,
            attempted,
            json!({ "monitored": monitored }),
        ))
    }

    /// Add a new item: look it up by `term`, then `POST /<res>` with the chosen
    /// quality profile (resolved name→id) and root folder. Kept minimal — the
    /// first lookup hit is used. Mutating but not destructive — runs immediately
    /// (no confirm gate).
    pub async fn arr_add(
        &self,
        service: &str,
        term: &str,
        quality_profile: &str,
        root_folder: &str,
    ) -> Result<Value> {
        let config = self.arr_context(service)?;
        let profile_id = self
            .arr_resolve_quality_profile_id(service, quality_profile)
            .await?;
        // S6: route the user-supplied `term` through the percent-encoding
        // `query_get` helper instead of `format!`-ing it into the path/query — a
        // value like `"foo&monitored=false"` must not be able to inject a second
        // query parameter or escape the lookup endpoint.
        let lookup_suffix = format!("{}/lookup", arr_resource_noun(config.kind));
        let lookup_base = arr_path(config.kind, &lookup_suffix);
        let lookup_url = query_get(config, &lookup_base, &[("term", term)])?;
        let hits = self.client_ref().send_get(config, lookup_url, None).await?;
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
    /// deletion is opt-in (`delete_files`); always confirm-gated and previews
    /// before applying.
    pub async fn arr_delete(
        &self,
        service: &str,
        id: i64,
        delete_files: bool,
        confirm: bool,
    ) -> Result<Value> {
        let config = self.arr_context(service)?;
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
                "mutating": true,
                "confirm_required": true,
                "hint": "re-run with confirm=true to delete",
            }));
        }
        self.client_ref().delete_json(config, &path, None).await?;
        Ok(json!({ "deleted": id, "delete_files": delete_files }))
    }
}

#[cfg(test)]
#[path = "write_tests.rs"]
mod tests;
