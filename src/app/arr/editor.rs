//! Pure (no `self`/network) builders + selectors for the ArrManager write
//! commands (C2). Split out of `write.rs` so the command methods and these
//! testable building blocks each stay well under the 500-LOC cap. Everything
//! here is deterministic and unit-tested directly (see `write_tests.rs`).

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::app::arr::read::arr_resource_noun;
use crate::config::ServiceKind;

/// Maximum number of items a single bulk write may touch without an explicit
/// `bulk=true` override (security S3 / AN-4 count cap). A name-based
/// `set_quality` over an entire large library, or a bulk `delete`/`monitor`, is
/// refused above this until the caller opts in — so a mistyped selector cannot
/// silently rewrite hundreds of items.
pub const MAX_BULK: usize = 100;

// ── case-sensitive /command names (best-practices FACT) ──────────────────────────
//
// The Servarr `/command` names are CASE-SENSITIVE and do NOT follow one uniform
// rule across the family — Sonarr is `SeriesSearch` but Radarr pluralises to
// `MoviesSearch`, while Lidarr/Readarr are `ArtistSearch`/`AuthorSearch`. So the
// search/refresh names are looked up by `resource_noun` in [`COMMAND_NAMES`]
// rather than hardcoded to the v3 (series/movie) pair, which keeps the v1 kinds
// (lidarr `artist`, readarr `author`) driven by the same descriptor seam as the
// paths and id keys (C3).

/// Per-resource-noun `(search, refresh)` `/command` names. Keyed by the
/// descriptor's `resource_noun` so adding an ArrManager kind is a one-line table
/// edit, never a new branch in the command-name selectors.
const COMMAND_NAMES: &[(&str, &str, &str)] = &[
    // (resource_noun, search command, refresh command)
    ("series", "SeriesSearch", "RefreshSeries"),
    ("movie", "MoviesSearch", "RefreshMovie"),
    ("artist", "ArtistSearch", "RefreshArtist"),
    ("author", "AuthorSearch", "RefreshAuthor"),
];

/// Look up the `(search, refresh)` command-name pair for a resource noun,
/// defaulting to the sonarr (series) pair for any unmapped noun.
fn command_names_for(noun: &str) -> (&'static str, &'static str) {
    COMMAND_NAMES
        .iter()
        .find(|(n, _, _)| *n == noun)
        .map(|(_, search, refresh)| (*search, *refresh))
        .unwrap_or(("SeriesSearch", "RefreshSeries"))
}

/// The `{resource_noun}Ids` body key the `/<res>/editor` endpoint expects for an
/// ArrManager kind (`series`→`seriesIds`, `movie`→`movieIds`). Pure for testing.
pub(crate) fn editor_id_key(kind: ServiceKind) -> String {
    format!("{}Ids", arr_resource_noun(kind))
}

/// The singular per-item id key for `/command` payloads (`seriesId`/`movieId`).
pub(crate) fn editor_id_key_singular(kind: ServiceKind) -> String {
    format!("{}Id", arr_resource_noun(kind))
}

/// The case-sensitive `/command` search name for a kind, resolved from the
/// descriptor's resource noun via [`COMMAND_NAMES`]. Pure for testing.
pub(crate) fn search_command_name(kind: ServiceKind) -> &'static str {
    command_names_for(arr_resource_noun(kind)).0
}

/// The case-sensitive `/command` refresh name for a kind, resolved from the
/// descriptor's resource noun via [`COMMAND_NAMES`]. Pure for testing.
pub(crate) fn refresh_command_name(kind: ServiceKind) -> &'static str {
    command_names_for(arr_resource_noun(kind)).1
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

/// True when this kind's `/command` search/refresh accepts a PLURAL `{noun}Ids`
/// batch in a single POST. ONLY Radarr does — its `MoviesSearch`/`RefreshMovie`
/// take `movieIds:[...]`. Sonarr (`SeriesSearch`/`RefreshSeries`),
/// Lidarr (`ArtistSearch`), and Readarr (`AuthorSearch`) have NO plural form and
/// take a SINGULAR `{noun}Id` per command, so multiple ids require one POST each.
pub(crate) fn kind_command_supports_plural_ids(kind: ServiceKind) -> bool {
    matches!(kind, ServiceKind::Radarr)
}

/// Build a SINGLE-id (or whole-library) `/command` body: `{name, <noun>Id}` for
/// one id, or `{name}` alone when `id` is `None` (whole monitored library). This
/// is the universal shape — every arr kind accepts the singular per-item key.
/// Pure for testing.
pub(crate) fn command_body_single(name: &str, id_key_singular: &str, id: Option<i64>) -> Value {
    let mut body = Map::new();
    body.insert("name".to_string(), json!(name));
    if let Some(id) = id {
        body.insert(id_key_singular.to_string(), json!(id));
    }
    Value::Object(body)
}

/// Build a PLURAL `/command` body: `{name, <noun>Ids:[...]}`. ONLY valid for kinds
/// where [`kind_command_supports_plural_ids`] is true (Radarr). Pure for testing.
pub(crate) fn command_body_plural(name: &str, id_key_singular: &str, ids: &[i64]) -> Value {
    let mut body = Map::new();
    body.insert("name".to_string(), json!(name));
    body.insert(format!("{id_key_singular}s"), json!(ids));
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

/// Select rows by explicit id. Mirrors [`select_by_titles`]: ids with no matching
/// row are collected and surfaced as a teaching error rather than copied verbatim
/// (which would push empty-title ghost rows into the selection and act on ids that
/// do not exist on the service).
pub(crate) fn select_by_ids(rows: &[Value], ids: &[i64]) -> Result<Selection> {
    let mut matched_ids = Vec::new();
    let mut titles = Vec::new();
    let mut misses = Vec::new();
    for id in ids {
        match rows.iter().find(|r| row_id(r) == Some(*id)) {
            Some(row) => {
                matched_ids.push(*id);
                titles.push(row_title(row));
            }
            None => misses.push(id.to_string()),
        }
    }
    if !misses.is_empty() {
        return Err(anyhow!("no items found for ids: [{}]", misses.join(", ")));
    }
    Ok(Selection {
        ids: matched_ids,
        titles,
    })
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
        if row.get("qualityProfileId").and_then(Value::as_i64) == Some(from_id)
            && let Some(id) = row_id(row)
        {
            ids.push(id);
            titles.push(row_title(row));
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

/// Build the apply summary for a `PUT /<res>/editor` mutation from the upstream
/// response. The *arr `/editor` endpoint echoes the updated resource array, so
/// `changed` reports the upstream-confirmed length and `attempted` the selection
/// size; when both agree the change is fully confirmed. If the response is NOT an
/// array (unexpected shape) we fall back to `attempted` for `changed` and mark
/// `confirmed: false` so the caller knows the count is not server-verified. The
/// extra `fields` (e.g. `{from,to}` or `{monitored}`) are merged in. Pure for
/// testing — no `self`/network.
pub(crate) fn editor_apply_summary(response: &Value, attempted: usize, fields: Value) -> Value {
    let mut out = match fields {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    out.insert("attempted".to_string(), json!(attempted));
    match response.as_array() {
        Some(rows) => {
            out.insert("changed".to_string(), json!(rows.len()));
            out.insert("confirmed".to_string(), json!(true));
        }
        None => {
            // Upstream did not echo an array; we cannot confirm the count.
            out.insert("changed".to_string(), json!(attempted));
            out.insert("confirmed".to_string(), json!(false));
        }
    }
    Value::Object(out)
}

/// A short human label for an unexpected JSON value's shape, for error messages.
pub(crate) fn value_shape(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// A short, length-capped preview of an unexpected JSON value, for error
/// messages (so the caller can see what the upstream actually returned).
pub(crate) fn value_preview(value: &Value) -> String {
    const MAX: usize = 200;
    let text = value.to_string();
    if text.len() > MAX {
        format!("{}...", &text[..MAX])
    } else {
        text
    }
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;
