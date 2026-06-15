//! CLI parse module for MediaServer curated commands (plex, jellyfin).
//!
//! Thin shim: it recognises the curated verbs, maps the friendly kebab CLI verb
//! to the snake_case registry/MCP action name (`sessions` → `media_sessions`,
//! `search` → `media_search` so it does not collide with the ArrManager `search`
//! command — registry action names are globally unique), assembles the JSON
//! `params` object (the positional `service` plus any flags) into a
//! [`Command::Curated`], and rejects unknown flags. All business logic lives in
//! `crate::app::media_server`; validation, scope, and dispatch flow through the
//! shared `execute_service_action` path, exactly like the MCP shim.

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::actions::curated_command;
use crate::capability::Capability;
use crate::cli::command::Command;
use crate::cli::parse::reject_args;
use crate::config::ServiceKind;

/// Canonical friendly CLI verb → snake_case registry action name, in declaration
/// order. SSOT for USAGE rendering and the mechanical CLI↔MCP parity test
/// (`tests/parity.rs`). One entry per MediaServer curated descriptor.
pub const VERBS: &[(&str, &str)] = &[
    ("sessions", "media_sessions"),
    ("libraries", "media_libraries"),
    ("search", "media_search"),
    ("scan", "media_scan"),
];

/// Try to parse `verb [rest]` as a MediaServer curated command for `kind`.
///
/// Returns `Ok(Some(cmd))` when `verb` is a known media verb, `Ok(None)` when it
/// is not (so the router falls through to its generic passthrough / "unknown
/// command" handling), and `Err` when the verb matched but its flags were
/// invalid.
pub fn parse(kind: ServiceKind, verb: &str, rest: &[String]) -> Result<Option<Command>> {
    // Single verb→action resolution against `VERBS` (the SSOT). `None` => the verb
    // isn't a MediaServer curated verb, so fall through to the router.
    let Some(action) = resolve(verb)? else {
        return Ok(None);
    };

    // Branch on the PARSING SHAPE only — keyed by the friendly verb, not a second
    // verb→action mapping.
    match verb {
        "sessions" | "libraries" => simple(kind, action, verb, rest).map(Some),
        "search" => parse_search(kind, action, rest).map(Some),
        "scan" => parse_scan(kind, action, rest).map(Some),
        // `resolve` only returns `Some` for verbs in `VERBS`; all are handled above.
        _ => unreachable!("VERBS verb `{verb}` has no parse arm"),
    }
}

/// `<svc> {sessions,libraries}` → `media_{sessions,libraries}` (no flags).
fn simple(kind: ServiceKind, action: &'static str, verb: &str, rest: &[String]) -> Result<Command> {
    reject_args(rest, verb)?;
    Ok(Command::Curated {
        action,
        params: json!({ "service": kind.as_str() }),
    })
}

/// `<svc> search --query X` → `media_search`.
fn parse_search(kind: ServiceKind, action: &'static str, rest: &[String]) -> Result<Command> {
    let mut params = base_params(kind);
    let mut query: Option<String> = None;

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            flag @ ("--query" | "--term") => {
                let value = take_value(rest, &mut i, flag)?;
                if query.replace(value).is_some() {
                    return Err(anyhow!("search received duplicate --query"));
                }
            }
            other => return Err(anyhow!("search does not accept argument `{other}`")),
        }
        i += 1;
    }

    let query = query.ok_or_else(|| anyhow!("search requires --query (a search term)"))?;
    params.insert("query".into(), json!(query));
    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

/// `<svc> scan [--library ID] [--confirm]` → `media_scan`.
///
/// `--library` (a Plex section id) is required for Plex and ignored by Jellyfin;
/// per-kind enforcement happens in `crate::app::media_server` so this shim stays
/// kind-agnostic.
fn parse_scan(kind: ServiceKind, action: &'static str, rest: &[String]) -> Result<Command> {
    let mut params = base_params(kind);

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--confirm" | "--yes" => {
                params.insert("confirm".into(), json!(true));
            }
            flag @ ("--library" | "--section") => {
                let value = take_value(rest, &mut i, flag)?;
                if params.insert("library".into(), json!(value)).is_some() {
                    return Err(anyhow!("scan received duplicate {flag}"));
                }
            }
            other => return Err(anyhow!("scan does not accept argument `{other}`")),
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

/// Resolve a friendly CLI `verb` against [`VERBS`] (the SSOT) to its MediaServer
/// curated action name.
///
/// Returns `Ok(None)` when `verb` is not a MediaServer curated verb (the caller
/// falls through), and an `Err` only if the VERBS↔registry wiring is broken — an
/// invariant guarded by `tests/parity.rs`, surfaced here as a clean parse error
/// instead of a panic.
fn resolve(verb: &str) -> Result<Option<&'static str>> {
    let Some((_, action)) = VERBS.iter().find(|(cli_verb, _)| *cli_verb == verb) else {
        return Ok(None);
    };
    curated_command(action)
        .filter(|cmd| cmd.capability == Capability::MediaServer)
        .map(|cmd| Some(cmd.name))
        .ok_or_else(|| anyhow!("internal: verb `{verb}` has no MediaServer descriptor"))
}

#[cfg(test)]
#[path = "media_server_tests.rs"]
mod tests;
