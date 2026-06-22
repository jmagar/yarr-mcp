//! Generated property definitions for service-named MCP tool input schemas.
//!
//! The property set is the UNION of the always-present generic params
//! (`action`/`service`/`path`/`body`/`confirm`) plus the response-verbosity
//! opt-ins (`verbose`/`fields`, AN-6) plus every param declared by a curated
//! command descriptor (`curated_param_names`). `additionalProperties:false` stays
//! strict, so curated params must be declared here or calls would be rejected —
//! generating them from the registry keeps that automatic.

use serde_json::{Map, Value, json};

use crate::actions::registry::{actions_for_curated_param, curated_param_type};
use crate::actions::{curated_param_names, valid_actions_for_kind};
use crate::config::ServiceKind;

/// Build the `properties` object for the tool input schema.
pub(super) fn properties(kind: ServiceKind) -> Value {
    let mut props = Map::new();

    props.insert(
        "action".into(),
        json!({
            "type": "string",
            "description": "The operation to perform.",
            "enum": valid_actions_for_kind(kind)
        }),
    );
    props.insert(
        "path".into(),
        json!({
            "type": "string",
            "minLength": 1,
            "description": "Safe relative upstream path, e.g. /api/v3/system/status."
        }),
    );
    props.insert(
        "body".into(),
        json!({
            "description": "JSON body for action=api_post/api_put, or optional body for action=api_delete."
        }),
    );
    props.insert(
        "code".into(),
        json!({
            "type": "string",
            "description": "For action=codemode: a JavaScript async arrow function that orchestrates rustarr actions via callTool(action, params) or tools.<action>(params). Returns { result, calls, logs }. For action=snippet_save: the snippet source."
        }),
    );
    props.insert(
        "name".into(),
        json!({
            "type": "string",
            "description": "Snippet name for action=snippet_save/snippet_run/snippet_delete ([A-Za-z0-9._-], no leading dot)."
        }),
    );
    props.insert(
        "description".into(),
        json!({
            "type": "string",
            "description": "Optional human description for action=snippet_save."
        }),
    );
    props.insert(
        "input".into(),
        // Free-form: a snippet's `input` may be any JSON value (object/array/scalar),
        // so no `type` constraint.
        json!({
            "description": "For action=snippet_run: arbitrary JSON bound as globalThis.input inside the snippet."
        }),
    );
    props.insert(
        "confirm".into(),
        json!({
            "type": "boolean",
            "description": "Confirmation for DESTRUCTIVE deletes only (api_delete, delete, download_remove, stats_delete_image_cache). MCP clients are normally prompted via elicitation; pass confirm=true to override the prompt (or when the client cannot elicit). Ignored by non-destructive writes."
        }),
    );
    // AN-6: response-verbosity opt-ins. Default is slim; agents opt in to fuller
    // payloads with verbose=true or request a specific field subset with fields.
    props.insert(
        "verbose".into(),
        json!({
            "type": "boolean",
            "description": "Opt in to a fuller response payload. Defaults to a slim response to conserve context."
        }),
    );
    props.insert(
        "fields".into(),
        json!({
            "type": "array",
            "items": { "type": "string" },
            "description": "Restrict returned item rows to these field names (for action=list, summary counts still use the full upstream rows)."
        }),
    );

    // Curated-command params (registry-derived). Each is an optional string by
    // default; descriptors that need richer typing can be enriched later. Skipped
    // entirely when no curated commands are registered (F4 state).
    for param in curated_param_names() {
        if param == "service" {
            continue;
        }
        props
            .entry(param.to_string())
            .or_insert_with(|| curated_param_schema(param));
    }

    Value::Object(props)
}

/// Schema fragment for a curated-command param.
///
/// P2-4: the JSON `type` (and array `items`) is derived from the param's
/// [`crate::actions::registry::ParamType`] declared on the `CommandDescriptor`
/// (`curated_param_type`), NOT a hand-written match — so a new non-string param
/// can no longer silently fall back to `string` under `additionalProperties:false`.
/// Params no command declares a type for fall back to string (the previous
/// behaviour for plain string params). The human-readable description is still
/// looked up per-param so agents get useful guidance; the type and the
/// description are independent concerns.
fn curated_param_schema(param: &str) -> Value {
    let mut schema = curated_param_type(param)
        .map(|ty| ty.json_schema_type())
        .unwrap_or_else(|| json!({ "type": "string" }));

    if let Some(obj) = schema.as_object_mut() {
        if let Some(desc) = curated_param_description(param) {
            obj.insert("description".into(), Value::String(desc.to_owned()));
        }
        let actions = actions_for_curated_param(param);
        if !actions.is_empty() {
            obj.insert("x-rustarr-actions".into(), json!(actions));
        }
    }
    schema
}

/// Human-readable description for a curated param. Type is supplied separately by
/// [`curated_param_schema`]; this only carries prose. Params without a tailored
/// description get none (the type fragment alone is still emitted).
fn curated_param_description(param: &str) -> Option<&'static str> {
    Some(match param {
        "ids" => "Resource ids to act on (selector).",
        "title" => "Resource titles to act on (selector).",
        "bulk" => "Override the bulk count cap to act on more than 100 items in one call.",
        "delete_files" => "For action=delete: also delete files on disk (opt-in).",
        "id" => {
            "Resource identifier as a string. A numeric id for arr/requests \
                 (action=delete, request_approve/request_decline) or a download-client \
                 handle (qBittorrent hash / SABnzbd nzo_id) for download_pause/resume/remove."
        }
        "media_id" => "TMDB media id to request (action=request_create).",
        "seasons" => "TV season numbers to request (action=request_create; selector).",
        "take" | "skip" => "Pagination knob for action=requests (take=page size, skip=offset).",
        "limit" => "Maximum number of item rows to return for action=list. Use 0 for summary only.",
        "offset" => "Number of item rows to skip before returning the action=list page.",
        "fields" => {
            "Item field names to include for action=list. Summary counts always use the full upstream rows."
        }
        "start" | "length" => {
            "Pagination knob for action=stats_history (start=offset, length=page size)."
        }
        _ => return None,
    })
}

/// Number of distinct top-level properties advertised. Exposed for tests so the
/// union invariant can be asserted without re-deriving it.
#[cfg(test)]
pub(super) fn property_count() -> usize {
    properties(ServiceKind::Sonarr)
        .as_object()
        .map(Map::len)
        .unwrap_or(0)
}

/// The base (always-present) property names, in declaration order. Used by tests
/// and to keep the action-enum source explicit.
#[cfg(test)]
pub(super) const BASE_PROPERTIES: &[&str] = &[
    "action",
    "path",
    "body",
    "code",
    "name",
    "description",
    "input",
    "confirm",
    "verbose",
    "fields",
];

#[cfg(test)]
#[path = "properties_tests.rs"]
mod tests;
