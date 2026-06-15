//! Generated schema conditionals: action → required-params and
//! action → allowed-service-kinds.
//!
//! Both tables are derived from the action registry + capability map — NOT
//! hand-coded — so every curated descriptor a later bead adds automatically
//! appears in the schema with the right required params and the right
//! allowed-kind set.
//!
//! These are emitted as JSON-Schema `allOf` `if/then` fragments for documentation
//! and client-side hinting. The *authoritative* action×kind enforcement lives in
//! the shared dispatch guard (`crate::actions::dispatch::validate_action_for_service`),
//! per the rmcp 1.7 guidance that most LLM clients reason poorly over `allOf`.

use serde_json::{Value, json};

use crate::actions::{all_action_names, allowed_kind_names_for_action, required_params_for_action};

/// Build the `allOf` array of conditional requirements for the input schema.
///
/// For each action with required params, emits an `if action == X then required
/// [...]` fragment. For each action whose allowed-kind set is a strict subset of
/// all kinds (i.e. a curated, capability-scoped command), emits a documentation
/// fragment recording which `service` kinds are valid (the registry remains the
/// SSOT; the dispatch guard enforces it).
pub(super) fn conditionals() -> Vec<Value> {
    let mut out = Vec::new();
    for action in all_action_names() {
        if let Some(fragment) = required_params_fragment(action) {
            out.push(fragment);
        }
        if let Some(fragment) = allowed_kinds_fragment(action) {
            out.push(fragment);
        }
    }
    out
}

fn required_params_fragment(action: &str) -> Option<Value> {
    let required = required_params_for_action(action);
    if required.is_empty() {
        return None;
    }
    Some(json!({
        "if": {
            "properties": { "action": { "const": action } },
            "required": ["action"]
        },
        "then": { "required": required }
    }))
}

/// Emit an allowed-kind hint only for capability-scoped actions (those whose
/// allowed-kind set is narrower than the full kind list). Generic/infra actions
/// are valid for every kind, so emitting a fragment for them would be noise.
fn allowed_kinds_fragment(action: &str) -> Option<Value> {
    let allowed = allowed_kind_names_for_action(action);
    if allowed.is_empty() {
        return None;
    }
    let total_kinds = crate::config::ServiceKind::ALL.len();
    if allowed.len() >= total_kinds {
        return None; // valid for all kinds — no constraint to document.
    }
    Some(json!({
        "if": {
            "properties": { "action": { "const": action } },
            "required": ["action"]
        },
        "then": {
            "properties": {
                "service": {
                    "description": format!(
                        "action={action} is only valid for service kinds: {}",
                        allowed.join(", ")
                    )
                }
            }
        }
    }))
}

#[cfg(test)]
#[path = "conditionals_tests.rs"]
mod tests;
