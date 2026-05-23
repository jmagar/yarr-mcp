//! Tool JSON schemas for the MCP rustarr tool.
//!
//! This file defines the action list and input schema for the `rustarr` tool.
//! MCP clients inspect this schema to know what arguments are valid.
//!
//! **Template**: rename `rustarr` to your tool name. Add/remove actions and
//! parameters to match your service. Use `"required": [...]` for mandatory args.

use std::sync::OnceLock;

use serde_json::{json, Value};

use crate::actions::action_names;

/// Cached JSON schema definitions (static data, built once at first call).
static TOOL_DEFINITIONS: OnceLock<Vec<Value>> = OnceLock::new();

/// Return the JSON schema definitions for all tools (cached after first call).
///
/// Returns a `Vec<Value>` where each item is a tool definition object matching
/// the MCP `Tool` schema: `{ name, description, inputSchema }`.
///
/// This is also used by the schema resource (`rustarr://schema/mcp-tool`).
pub(super) fn tool_definitions() -> &'static Vec<Value> {
    TOOL_DEFINITIONS.get_or_init(build_tool_definitions)
}

fn build_tool_definitions() -> Vec<Value> {
    vec![json!({
        "name": "rustarr",
        "description": "Rustarr media-service MCP tool. Use action=help for full documentation.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "The operation to perform.",
                    "enum": action_names()
                },
                "service": {
                    "type": "string",
                    "description": "Configured service name or kind, e.g. sonarr, radarr, plex."
                },
                "path": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Safe relative upstream path, e.g. /api/v3/system/status."
                },
                "body": {
                    "description": "JSON body for action=api_post."
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Required true for action=api_post because generic upstream POST can mutate services."
                }
            },
            "required": ["action"],
            "additionalProperties": false,
            "allOf": [
                {
                    "if": {
                        "properties": { "action": { "enum": ["service_status"] } },
                        "required": ["action"]
                    },
                    "then": { "required": ["service"] }
                },
                {
                    "if": {
                        "properties": { "action": { "enum": ["api_get", "api_post"] } },
                        "required": ["action"]
                    },
                    "then": { "required": ["service", "path"] }
                },
                {
                    "if": {
                        "properties": { "action": { "enum": ["api_post"] } },
                        "required": ["action"]
                    },
                    "then": { "required": ["confirm"] }
                },
                {
                    "if": {
                        "properties": {
                            "action": { "enum": ["elicit_name", "scaffold_intent"] }
                        },
                        "required": ["action"]
                    },
                    "then": {
                        "description": "This action uses MCP elicitation. Input fields are requested through the client-rendered elicitation form, not through tool-call arguments."
                    }
                }
            ]
        }
    })]
}

#[cfg(test)]
#[path = "schemas_tests.rs"]
mod tests;
