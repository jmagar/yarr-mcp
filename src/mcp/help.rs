//! Generated help text for the MCP `help` action.
//!
//! The previous static `HELP_TEXT` const drifted from the registry whenever an
//! action was added. This module GENERATES the help from the action registry +
//! capability map so curated commands a later bead adds appear automatically,
//! and includes a compact capability digest (AN-1/AN-3) for first-try action
//! selection.

use crate::actions::{
    all_action_names, capability_digest, curated_command, required_params_for_action,
    required_scope_for_action, WRITE_SCOPE,
};

/// Static one-line descriptions for the generic/infra actions. Curated commands
/// carry their own `description` in their descriptor.
fn generic_description(action: &str) -> &'static str {
    match action {
        "integrations" => "list supported and configured integrations, with per-service capability and available actions.",
        "service_status" => "call the default status endpoint for a configured service. Requires `service`.",
        "api_get" => "GET a safe relative path. Requires `service` and `path`.",
        "api_post" => "POST JSON to a safe relative path. Requires `service`, `path`, and `confirm=true`; optional `body` defaults to `{}`.",
        "api_put" => "PUT JSON to a safe relative path. Requires `service`, `path`, and `confirm=true`; optional `body` defaults to `{}`.",
        "api_delete" => "DELETE a safe relative path. Requires `service`, `path`, and `confirm=true`; optional `body`. Query params go in `path`.",
        "help" => "return this help text.",
        _ => "",
    }
}

/// Render the full Markdown help text for the single `rustarr` tool.
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
