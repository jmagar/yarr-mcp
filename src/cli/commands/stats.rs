//! CLI parse module for Stats curated commands (tautulli).
//!
//! Thin shim: it recognises the curated verbs, maps the friendly kebab CLI verb to
//! the snake_case registry/MCP action name (`activity` → `stats_activity`,
//! `history` → `stats_history` — `history` would otherwise collide with the
//! ArrManager `history` action, registry action names are globally unique — `users`
//! → `stats_users`, `libraries` → `stats_libraries`), assembles the JSON `params`
//! object (the positional `service` plus any flags) into a [`Command::Curated`],
//! and rejects unknown flags. All business logic lives in `crate::app::stats`;
//! validation, scope, and dispatch flow through the shared `execute_service_action`
//! path, exactly like the MCP shim.

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use crate::actions::curated_command;
use crate::capability::Capability;
use crate::cli::command::Command;
use crate::config::ServiceKind;

/// Canonical friendly CLI verb → snake_case registry action name, in declaration
/// order. SSOT for USAGE rendering and the mechanical CLI↔MCP parity test
/// (`tests/parity.rs`). One entry per Stats curated descriptor.
pub const VERBS: &[(&str, &str)] = &[
    ("activity", "stats_activity"),
    ("history", "stats_history"),
    ("users", "stats_users"),
    ("libraries", "stats_libraries"),
];

/// Try to parse `verb [rest]` as a Stats curated command for `kind`.
///
/// Returns `Ok(Some(cmd))` when `verb` is a known Stats verb, `Ok(None)` when it
/// is not (so the router falls through to its generic passthrough / "unknown
/// command" handling), and `Err` when the verb matched but its flags were invalid.
pub fn parse(kind: ServiceKind, verb: &str, rest: &[String]) -> Result<Option<Command>> {
    // Single verb→action resolution against `VERBS` (the SSOT). `None` => the verb
    // isn't a Stats curated verb, so fall through to the router.
    let Some(action) = resolve(verb)? else {
        return Ok(None);
    };

    // Branch on the PARSING SHAPE only — keyed by the friendly verb, not a second
    // verb→action mapping.
    match verb {
        "history" => parse_history(kind, action, rest).map(Some),
        // No-flag read verbs (activity, users, libraries).
        _ => parse_simple(kind, action, verb, rest).map(Some),
    }
}

/// `tautulli {activity,users,libraries}` → `stats_{...}` (no flags).
fn parse_simple(
    kind: ServiceKind,
    action: &'static str,
    verb: &str,
    rest: &[String],
) -> Result<Command> {
    if let Some(extra) = rest.first() {
        return Err(anyhow!("{verb} does not accept argument `{extra}`"));
    }
    Ok(Command::Curated {
        action,
        params: Value::Object(base_params(kind)),
    })
}

/// `tautulli history [--start N] [--length N] [--user NAME]` → `stats_history`.
fn parse_history(kind: ServiceKind, action: &'static str, rest: &[String]) -> Result<Command> {
    let mut params = base_params(kind);

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            flag @ ("--start" | "--length" | "--user") => {
                let value = take_value(rest, &mut i, flag)?;
                let key = &flag[2..];
                if params.insert(key.into(), json!(value)).is_some() {
                    return Err(anyhow!("history received duplicate {flag}"));
                }
            }
            other => return Err(anyhow!("history does not accept argument `{other}`")),
        }
        i += 1;
    }

    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

/// Initial params map carrying the positional service.
fn base_params(kind: ServiceKind) -> Map<String, Value> {
    let mut params = Map::new();
    params.insert("service".into(), json!(kind.as_str()));
    params
}

/// Advance `i` to the value after a flag, rejecting a missing/flag-like value.
fn take_value(rest: &[String], i: &mut usize, flag: &str) -> Result<String> {
    *i += 1;
    rest.get(*i)
        .filter(|v| !v.starts_with("--"))
        .cloned()
        .ok_or_else(|| anyhow!("{flag} requires a value"))
}

/// Resolve a friendly CLI `verb` against [`VERBS`] (the SSOT) to its Stats
/// curated action name.
///
/// Returns `Ok(None)` when `verb` is not a Stats curated verb (the caller falls
/// through), and an `Err` only if the VERBS↔registry wiring is broken — an
/// invariant guarded by `tests/parity.rs`, surfaced here as a clean parse error
/// instead of a panic.
fn resolve(verb: &str) -> Result<Option<&'static str>> {
    let Some((_, action)) = VERBS.iter().find(|(cli_verb, _)| *cli_verb == verb) else {
        return Ok(None);
    };
    curated_command(action)
        .filter(|cmd| cmd.capability == Capability::Stats)
        .map(|cmd| Some(cmd.name))
        .ok_or_else(|| anyhow!("internal: verb `{verb}` has no Stats descriptor"))
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
