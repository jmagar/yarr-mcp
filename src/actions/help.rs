//! Help payload generation for the `help` action (REST + MCP structured JSON).

use serde_json::{json, Value};

use super::registry::{mcp_only_action_names, rest_action_names};

pub fn rest_help() -> Value {
    json!({
        "actions": rest_action_names(),
        "mcp_only_actions": mcp_only_action_names(),
        "usage": "Use the rustarr MCP tool or CLI commands such as `rustarr get --service sonarr --path /api/v3/system/status`.",
        "examples": {
            "integrations": {"action": "integrations"},
            "service_status": {"action": "service_status", "service": "sonarr"},
            "api_get": {"action": "api_get", "service": "radarr", "path": "/api/v3/system/status"},
            "api_post": {"action": "api_post", "service": "overseerr", "path": "/api/v1/request", "body": {}, "confirm": true},
            "api_put": {"action": "api_put", "service": "sonarr", "path": "/api/v3/series/editor", "body": {}, "confirm": true},
            "api_delete": {"action": "api_delete", "service": "sonarr", "path": "/api/v3/series/123?deleteFiles=false", "confirm": true}
        }
    })
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod tests;
