//! Help payload generation for the `help` action.
//!
//! Two shapes share this module:
//!   * [`rest_help`] — structured JSON used by the CLI `help` command.
//!   * [`help_text`] — generated Markdown used by the MCP `help` action.
//!
//! The Markdown is GENERATED from the action registry + capability map (rather
//! than a static const that drifts) so curated commands appear automatically and
//! a compact capability digest (AN-1/AN-3) aids first-try action selection.

use serde_json::{Value, json};

use super::WRITE_SCOPE;
use super::registry::{
    all_action_names, capability_digest, curated_command, mcp_only_action_names,
    required_params_for_action, required_scope_for_action, rest_action_names,
};

pub fn rest_help() -> Value {
    json!({
        "actions": rest_action_names(),
        "mcp_only_actions": mcp_only_action_names(),
        "usage": "Use the rustarr MCP tool or CLI commands such as `rustarr sonarr get --path /api/v3/system/status`.",
        "examples": {
            "service_status": {"action": "service_status", "service": "sonarr"},
            "api_get": {"action": "api_get", "service": "radarr", "path": "/api/v3/system/status"},
            "api_post": {"action": "api_post", "service": "overseerr", "path": "/api/v1/request", "body": {}},
            "api_put": {"action": "api_put", "service": "sonarr", "path": "/api/v3/series/editor", "body": {}},
            "api_delete": {"action": "api_delete", "service": "sonarr", "path": "/api/v3/series/123?deleteFiles=false", "confirm": true}
        }
    })
}

/// Static one-line descriptions for the generic/infra actions. Curated commands
/// carry their own `description` in their descriptor.
fn generic_description(action: &str) -> &'static str {
    match action {
        "service_status" => {
            "call the default status endpoint for a configured service. Requires `service`."
        }
        "api_get" => {
            "GET a safe relative path. Requires `service` and `path`. Needs `rustarr:write` (not just `rustarr:read`) because it is an arbitrary upstream passthrough — a GET can reach any endpoint, including mutating ones — so a read-only token is intentionally insufficient; use the curated read commands for read-scoped access."
        }
        "api_post" => {
            "POST JSON to a safe relative path. Requires `service` and `path`; optional `body` defaults to `{}`. Non-destructive — runs immediately."
        }
        "api_put" => {
            "PUT JSON to a safe relative path. Requires `service` and `path`; optional `body` defaults to `{}`. Non-destructive — runs immediately."
        }
        "api_delete" => {
            "DELETE a safe relative path. Requires `service` and `path`; optional `body`. Query params go in `path`. DESTRUCTIVE — the MCP client is prompted to confirm (elicitation); pass `confirm=true` to override (or for clients that cannot elicit)."
        }
        "help" => "return this help text.",
        _ => "",
    }
}

/// Render the full Markdown help text for the single `rustarr` tool (MCP `help`).
pub fn help_text() -> String {
    let mut out = String::from("# rustarr MCP Tool\n\nSingle tool: `rustarr`\n\n");

    if let Some(digest) = capability_digest() {
        out.push_str("Capabilities: ");
        out.push_str(&digest);
        out.push_str("\n\n");
    }

    out.push_str("Actions:\n");
    for action in all_action_names() {
        let desc = match curated_command(action) {
            Some(cmd) => cmd.description.to_string(),
            None => generic_description(action).to_string(),
        };
        let mut line = format!("- `{action}`: {desc}");
        let required = required_params_for_action(action);
        if !required.is_empty() {
            line.push_str(&format!(" (params: {})", required.join(", ")));
        }
        if required_scope_for_action(action) == Some(WRITE_SCOPE) {
            line.push_str(" [requires rustarr:write]");
        }
        out.push_str(&line);
        out.push('\n');
    }

    out.push_str(
        "\nCredentials are configured outside tool-call arguments through `RUSTARR_SERVICES`\n\
         and per-service environment variables or config.toml. Do not pass API keys in\n\
         paths or request bodies unless the upstream endpoint itself requires it.\n",
    );
    out
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod tests;
