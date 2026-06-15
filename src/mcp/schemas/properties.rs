//! Generated property definitions for the `rustarr` tool input schema.
//!
//! The property set is the UNION of the always-present generic params
//! (`action`/`service`/`path`/`body`/`confirm`) plus the response-verbosity
//! opt-ins (`verbose`/`fields`, AN-6) plus every param declared by a curated
//! command descriptor (`curated_param_names`). `additionalProperties:false` stays
//! strict, so curated params must be declared here or calls would be rejected —
//! generating them from the registry keeps that automatic.

use serde_json::{json, Map, Value};

use crate::actions::{all_action_names, curated_param_names};

/// Build the `properties` object for the tool input schema.
pub(super) fn properties() -> Value {
    let mut props = Map::new();

    props.insert(
        "action".into(),
        json!({
            "type": "string",
            "description": "The operation to perform.",
            "enum": all_action_names()
        }),
    );
    props.insert(
        "service".into(),
        json!({
            "type": "string",
            "description": "Configured service name or kind, e.g. sonarr, radarr, plex."
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
        "confirm".into(),
        json!({
            "type": "boolean",
            "description": "Required true for action=api_post/api_put/api_delete because generic upstream writes can mutate services."
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
            "description": "Restrict the response to these field names (response-shaping opt-in)."
        }),
    );

    // Curated-command params (registry-derived). Each is an optional string by
    // default; descriptors that need richer typing can be enriched later. Skipped
    // entirely when no curated commands are registered (F4 state).
    for param in curated_param_names() {
        props
            .entry(param.to_string())
            .or_insert_with(|| json!({ "type": "string" }));
    }

    Value::Object(props)
}

/// Number of distinct top-level properties advertised. Exposed for tests so the
/// union invariant can be asserted without re-deriving it.
#[cfg(test)]
pub(super) fn property_count() -> usize {
    properties().as_object().map(Map::len).unwrap_or(0)
}

/// The base (always-present) property names, in declaration order. Used by tests
/// and to keep the action-enum source explicit.
#[cfg(test)]
pub(super) const BASE_PROPERTIES: &[&str] = &[
    "action", "service", "path", "body", "confirm", "verbose", "fields",
];

#[cfg(test)]
#[path = "properties_tests.rs"]
mod tests;
