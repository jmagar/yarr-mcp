//! Tool JSON schema for the service-named MCP tools.
//!
//! The schema is fully GENERATED from the action registry + capability map:
//!   - [`properties`] — the property set (generic params + curated params +
//!     verbose/fields), with the `action` enum coming from `action_names()` plus
//!     curated command names.
//!   - [`conditionals`] — action→required-params and action→allowed-kinds
//!     `allOf` fragments.
//!
//! Each advertised tool is named after one configured media-service kind
//! (`sonarr`, `radarr`, ...). Adding a curated command descriptor makes it
//! appear on every compatible service tool with no hand-maintained schema list.
//! The authoritative action×kind enforcement is the shared dispatch guard, not
//! these schema hints.
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

use crate::config::ServiceKind;

/// Cached JSON schema definitions (static data, built once at first call).
static TOOL_DEFINITIONS: OnceLock<Vec<Value>> = OnceLock::new();

pub(super) const SERVICE_TOOL_KINDS: &[ServiceKind] = &[
    ServiceKind::Sonarr,
    ServiceKind::Radarr,
    ServiceKind::Prowlarr,
    ServiceKind::Overseerr,
    ServiceKind::Tautulli,
    ServiceKind::Plex,
    ServiceKind::Tracearr,
    ServiceKind::Sabnzbd,
    ServiceKind::Qbittorrent,
    ServiceKind::Jellyfin,
    ServiceKind::Bazarr,
];

/// Return the JSON schema definitions for all tools (cached after first call).
///
/// Returns a `Vec<Value>` where each item is a tool definition object matching
/// the MCP `Tool` schema: `{ name, description, inputSchema }`.
///
/// This is also used by the schema resource (`rustarr://schema/mcp-tool`).
pub(super) fn tool_definitions() -> &'static Vec<Value> {
    TOOL_DEFINITIONS.get_or_init(build_tool_definitions)
}

/// Build the tool definitions. The action enum for each service is derived from
/// action metadata — see `action_names()` (via `properties::properties`).
fn build_tool_definitions() -> Vec<Value> {
    SERVICE_TOOL_KINDS
        .iter()
        .map(|kind| {
            json!({
            "name": kind.as_str(),
            "description": tool_description(*kind),
            "inputSchema": {
                "type": "object",
                "properties": properties::properties(*kind),
                "required": ["action"],
                "additionalProperties": false,
                "allOf": conditionals::conditionals(*kind),
            }
            })
        })
        .collect()
}

/// Tool description, with a registry-derived capability digest appended when
/// curated commands exist (AN-1) so agents can pick the right action up front.
fn tool_description(kind: ServiceKind) -> String {
    let actions = crate::actions::valid_actions_for_kind(kind).join(", ");
    format!(
        "{} media-service MCP tool. The service is implicit; pass action plus action-specific params. Valid actions: {actions}",
        kind.as_str()
    )
}

pub(super) fn service_tool_kind(name: &str) -> Option<ServiceKind> {
    SERVICE_TOOL_KINDS
        .iter()
        .copied()
        .find(|kind| kind.as_str() == name)
}

#[cfg(test)]
#[path = "schemas_tests.rs"]
mod tests;
