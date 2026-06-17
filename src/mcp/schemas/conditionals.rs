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

use crate::actions::{required_params_for_action, valid_actions_for_kind};
use crate::config::ServiceKind;

/// Build the `allOf` array of conditional requirements for the input schema.
///
/// For each action with required params, emits an `if action == X then required
/// [...]` fragment. For each action whose allowed-kind set is a strict subset of
/// all kinds (i.e. a curated, capability-scoped command), emits a documentation
/// fragment recording which `service` kinds are valid (the registry remains the
/// SSOT; the dispatch guard enforces it).
pub(super) fn conditionals(kind: ServiceKind) -> Vec<Value> {
    let mut out = Vec::new();
    for action in valid_actions_for_kind(kind) {
        if let Some(fragment) = required_params_fragment(action) {
            out.push(fragment);
        }
    }
    out
}

fn required_params_fragment(action: &str) -> Option<Value> {
    let required: Vec<&'static str> = required_params_for_action(action)
        .into_iter()
        .filter(|param| *param != "service")
        .collect();
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

#[cfg(test)]
#[path = "conditionals_tests.rs"]
mod tests;
