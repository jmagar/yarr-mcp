//! CLI parse module for ArrManager curated commands (sonarr/radarr).
//!
//! Thin shim: it only recognises the curated verbs, maps the kebab-case CLI verb
//! to the snake_case registry/MCP action name, and assembles the JSON `params`
//! object (positional `service` + any flags) into a [`Command::Curated`]. All
//! business logic lives in `crate::app::arr`; validation/scope/dispatch flow
//! through the shared `execute_service_action` path, exactly like the MCP shim.

use anyhow::Result;
use serde_json::json;

use crate::actions::curated_command;
use crate::capability::Capability;
use crate::cli::command::Command;
use crate::cli::parse::reject_args;
use crate::config::ServiceKind;

/// Try to parse `verb [rest]` as an ArrManager curated command for `kind`.
///
/// Returns `Ok(Some(cmd))` when `verb` is a known ArrManager curated verb,
/// `Ok(None)` when it is not (so the router can fall through to its
/// "unknown command" error), and `Err` when the verb matched but its flags were
/// invalid. The C1 read commands take only the positional service (no flags).
pub fn parse(kind: ServiceKind, verb: &str, rest: &[String]) -> Result<Option<Command>> {
    let action = match verb {
        // C1 read verbs (kebab-case CLI ↔ snake_case registry name).
        "quality-profiles" => "quality_profiles",
        "list" => "list",
        "wanted" => "wanted",
        "queue" => "queue",
        "history" => "history",
        "rootfolders" => "rootfolders",
        "health" => "health",
        _ => return Ok(None),
    };

    // Resolve to the static registry name and confirm capability wiring is intact
    // (the descriptor must be ArrManager-scoped).
    let descriptor = curated_command(action)
        .filter(|cmd| cmd.capability == Capability::ArrManager)
        .expect("ArrManager curated verb must resolve to an ArrManager descriptor");

    // C1 commands accept no flags beyond the positional service.
    reject_args(rest, verb)?;

    let params = json!({ "service": kind.as_str() });
    Ok(Some(Command::Curated {
        action: descriptor.name,
        params,
    }))
}

#[cfg(test)]
#[path = "arr_tests.rs"]
mod tests;
