//! Pure (no `self`/network) builders + selectors for the ArrManager write
//! commands (C2). Split out of `write.rs` so the command methods and these
//! testable building blocks each stay well under the 500-LOC cap. Everything
//! here is deterministic and unit-tested directly (see `write_tests.rs`).

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use crate::app::arr::read::arr_resource_noun;
use crate::config::ServiceKind;

/// Maximum number of items a single bulk write may touch without an explicit
/// `bulk=true` override (security S3 / AN-4 count cap). A name-based
/// `set_quality` over an entire large library, or a bulk `delete`/`monitor`, is
/// refused above this until the caller opts in — so a mistyped selector cannot
/// silently rewrite hundreds of items.
pub const MAX_BULK: usize = 100;

// ── case-sensitive /command names (best-practices FACT) ──────────────────────────

/// Sonarr full-library / per-series search command name.
const CMD_SERIES_SEARCH: &str = "SeriesSearch";
/// Radarr per-movie search command name.
const CMD_MOVIES_SEARCH: &str = "MoviesSearch";
/// Sonarr per-series refresh command name.
const CMD_REFRESH_SERIES: &str = "RefreshSeries";
/// Radarr per-movie refresh command name.
const CMD_REFRESH_MOVIE: &str = "RefreshMovie";

/// The `{resource_noun}Ids` body key the `/<res>/editor` endpoint expects for an
/// ArrManager kind (`series`→`seriesIds`, `movie`→`movieIds`). Pure for testing.
pub(crate) fn editor_id_key(kind: ServiceKind) -> String {
    format!("{}Ids", arr_resource_noun(kind))
}

/// The singular per-item id key for `/command` payloads (`seriesId`/`movieId`).
pub(crate) fn editor_id_key_singular(kind: ServiceKind) -> String {
    format!("{}Id", arr_resource_noun(kind))
}

/// The case-sensitive `/command` search name for a kind. Pure for testing.
pub(crate) fn search_command_name(kind: ServiceKind) -> &'static str {
    if arr_resource_noun(kind) == "movie" {
        CMD_MOVIES_SEARCH
    } else {
        CMD_SERIES_SEARCH
    }
}

/// The case-sensitive `/command` refresh name for a kind. Pure for testing.
pub(crate) fn refresh_command_name(kind: ServiceKind) -> &'static str {
    if arr_resource_noun(kind) == "movie" {
        CMD_REFRESH_MOVIE
    } else {
        CMD_REFRESH_SERIES
    }
}

/// Build the `PUT /<res>/editor` body for a bulk quality-profile change. Pure (no
/// `self`/network) so the body shape — correct id key per resource +
/// `qualityProfileId` — is unit-testable without a live service.
pub(crate) fn editor_quality_body(kind: ServiceKind, ids: &[i64], profile_id: i64) -> Value {
    let mut body = Map::new();
    body.insert(editor_id_key(kind), json!(ids));
    body.insert("qualityProfileId".to_string(), json!(profile_id));
    Value::Object(body)
}

/// Build the `PUT /<res>/editor` body for a bulk monitor toggle. Pure for testing.
pub(crate) fn editor_monitor_body(kind: ServiceKind, ids: &[i64], monitored: bool) -> Value {
    let mut body = Map::new();
    body.insert(editor_id_key(kind), json!(ids));
    body.insert("monitored".to_string(), json!(monitored));
    Value::Object(body)
}

/// Build a `/command` body. For a single id, *arr commands take `{name, <res>Id}`;
/// for multiple/none, just `{name}` (whole-library). Pure for testing.
pub(crate) fn command_body(name: &str, id_key: &str, ids: &[i64]) -> Value {
    let mut body = Map::new();
    body.insert("name".to_string(), json!(name));
    if ids.len() == 1 {
        body.insert(id_key.to_string(), json!(ids[0]));
    } else if !ids.is_empty() {
        // Plural form for the *arr search commands that accept *Ids batches.
        body.insert(format!("{id_key}s"), json!(ids));
    }
    Value::Object(body)
}

/// Minimal `POST /<res>` add body: carry the lookup match forward and set the
/// chosen profile + root folder + monitored. `*arr` accepts the full lookup
/// object back with these fields filled in.
pub(crate) fn build_add_body(lookup: &Value, profile_id: i64, root_folder: &str) -> Value {
    let mut obj = lookup.as_object().cloned().unwrap_or_default();
    obj.insert("qualityProfileId".to_string(), json!(profile_id));
    obj.insert("rootFolderPath".to_string(), json!(root_folder));
    obj.insert("monitored".to_string(), json!(true));
    obj.insert(
        "addOptions".to_string(),
        json!({ "searchForMissingEpisodes": false }),
    );
    Value::Object(obj)
}

/// Minimal percent-encode for a lookup term query value.
pub(crate) fn urlencode(term: &str) -> String {
    let mut out = String::with_capacity(term.len());
    for b in term.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Enforce the bulk count cap (S3/AN-4): refuse > [`MAX_BULK`] items unless the
/// caller passed an explicit `bulk=true` override.
pub(crate) fn guard_count(count: usize, bulk: bool) -> Result<()> {
    if count > MAX_BULK && !bulk {
        return Err(anyhow!(
            "refusing to act on {count} items (> {MAX_BULK}); pass bulk=true (CLI --bulk) to override"
        ));
    }
    Ok(())
}

// ── selection ────────────────────────────────────────────────────────────────────

/// A resolved selection of items to act on: their ids plus their titles (titles
/// kept only for building a compact preview / summary, never echoed wholesale).
#[derive(Debug)]
pub(crate) struct Selection {
    pub ids: Vec<i64>,
    pub titles: Vec<String>,
}

impl Selection {
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Up to `n` sample titles for a preview, so the agent can sanity-check the
    /// selection without the response carrying every row.
    pub fn sample(&self, n: usize) -> Vec<String> {
        self.titles.iter().take(n).cloned().collect()
    }
}

/// Extract `id` from a resource row.
pub(crate) fn row_id(row: &Value) -> Option<i64> {
    row.get("id").and_then(Value::as_i64)
}

/// Extract `title` from a resource row (empty when absent).
pub(crate) fn row_title(row: &Value) -> String {
    row.get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

pub(crate) fn select_by_ids(rows: &[Value], ids: &[i64]) -> Selection {
    let mut titles = Vec::new();
    for id in ids {
        if let Some(row) = rows.iter().find(|r| row_id(r) == Some(*id)) {
            titles.push(row_title(row));
        } else {
            titles.push(String::new());
        }
    }
    Selection {
        ids: ids.to_vec(),
        titles,
    }
}

pub(crate) fn select_by_titles(rows: &[Value], titles: &[String]) -> Result<Selection> {
    let mut ids = Vec::new();
    let mut matched_titles = Vec::new();
    let mut misses = Vec::new();
    for wanted in titles {
        let needle = wanted.trim().to_ascii_lowercase();
        match rows
            .iter()
            .find(|r| row_title(r).trim().to_ascii_lowercase() == needle)
        {
            Some(row) => {
                if let Some(id) = row_id(row) {
                    ids.push(id);
                    matched_titles.push(row_title(row));
                }
            }
            None => misses.push(wanted.clone()),
        }
    }
    if !misses.is_empty() {
        return Err(anyhow!("no item matched title(s): [{}]", misses.join(", ")));
    }
    Ok(Selection {
        ids,
        titles: matched_titles,
    })
}

pub(crate) fn select_by_profile(rows: &[Value], from_id: i64) -> Selection {
    let mut ids = Vec::new();
    let mut titles = Vec::new();
    for row in rows {
        if row.get("qualityProfileId").and_then(Value::as_i64) == Some(from_id) {
            if let Some(id) = row_id(row) {
                ids.push(id);
                titles.push(row_title(row));
            }
        }
    }
    Selection { ids, titles }
}

pub(crate) fn select_all(rows: &[Value]) -> Selection {
    let mut ids = Vec::new();
    let mut titles = Vec::new();
    for row in rows {
        if let Some(id) = row_id(row) {
            ids.push(id);
            titles.push(row_title(row));
        }
    }
    Selection { ids, titles }
}

/// Build the structured `set_quality` dry-run preview (S3/AN-4). Pure (no
/// `self`/network) so the preview contract — `would_do`, `target_profile`,
/// `from_profile`, `count`, `sample_titles`, `confirm_required` — is unit-testable
/// without a live service, and so the dry-run path provably constructs a preview
/// rather than issuing the PUT.
pub(crate) fn set_quality_preview(
    service: &str,
    from: Option<&str>,
    from_id: Option<i64>,
    to: &str,
    to_id: i64,
    selection: &Selection,
) -> Value {
    json!({
        "would_do": "set_quality",
        "service": service,
        "target_profile": { "name": to, "id": to_id },
        "from_profile": from.map(|n| json!({ "name": n, "id": from_id })),
        "count": selection.len(),
        "sample_titles": selection.sample(10),
        "confirm_required": true,
        "hint": "re-run with confirm=true (CLI --confirm/--yes) to apply",
    })
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;
