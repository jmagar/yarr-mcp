//! Tool JSON schema for the MCP `rustarr` tool (facade).
//!
//! The schema is fully GENERATED from the action registry + capability map:
//!   - [`properties`] — the property set (generic params + curated params +
//!     verbose/fields), with the `action` enum coming from `action_names()` plus
//!     curated command names.
//!   - [`conditionals`] — action→required-params and action→allowed-kinds
//!     `allOf` fragments.
//!
//! There is exactly ONE tool, `rustarr`. Adding a curated command descriptor
//! makes it appear in the enum, properties, conditionals, and help with no edits
//! here. The authoritative action×kind enforcement is the shared dispatch guard,
//! not these `allOf` hints.
//!
//! NOTE: the substring `action_names()` is asserted by the schema-contract doc
//! test (`tests/template_invariants.rs`) — the enum is derived from action
//! metadata via `properties::properties()`, which calls `all_action_names()`
//! (generic `action_names()` plus curated names). Do not remove the reference in
//! this comment or the derivation in `properties`.

mod conditionals;
mod properties;

use std::sync::OnceLock;

use serde_json::{Value, json};

/// Cached JSON schema definitions (static data, built once at first call).
static TOOL_DEFINITIONS: OnceLock<Vec<Value>> = OnceLock::new();

/// Return the JSON schema definitions for all tools (cached after first call).
///
/// Returns a `Vec<Value>` where each item is a tool definition object matching
/// the MCP `Tool` schema: `{ name, description, inputSchema }`. Exactly one tool
/// (`rustarr`) is returned.
///
/// This is also used by the schema resource (`rustarr://schema/mcp-tool`).
pub(super) fn tool_definitions() -> &'static Vec<Value> {
    TOOL_DEFINITIONS.get_or_init(build_tool_definitions)
}

/// Build the (single) tool definition. The action enum is derived from action
/// metadata — see `action_names()` (via `properties::properties`).
fn build_tool_definitions() -> Vec<Value> {
    vec![json!({
        "name": "rustarr",
        "description": tool_description(),
        "inputSchema": {
            "type": "object",
            "properties": properties::properties(),
            "required": ["action"],
            "additionalProperties": false,
            "allOf": conditionals::conditionals(),
        }
    })]
}

/// Tool description, with a registry-derived capability digest appended when
/// curated commands exist (AN-1) so agents can pick the right action up front.
fn tool_description() -> String {
    let base = "Rustarr media-service MCP tool. Use action=help for full documentation.";
    match crate::actions::capability_digest() {
        Some(digest) => format!("{base} Capabilities: {digest}"),
        None => base.to_owned(),
    }
}

#[cfg(test)]
#[path = "schemas_tests.rs"]
mod tests;
