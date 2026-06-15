//! CLI parse module for Requests curated commands (overseerr).
//!
//! Thin shim: it recognises the curated verbs, maps the friendly kebab CLI verb
//! to the snake_case registry/MCP action name (`request` → `request_create`,
//! `approve` → `request_approve`, `decline` → `request_decline`, `search` →
//! `request_search` so they do not collide with the ArrManager `search`/`add`
//! commands — registry action names are globally unique), assembles the JSON
//! `params` object (the positional `service` plus any flags) into a
//! [`Command::Curated`], and rejects unknown flags. All business logic lives in
//! `crate::app::requests`; validation, scope, and dispatch flow through the shared
//! `execute_service_action` path, exactly like the MCP shim.

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use crate::actions::curated_command;
use crate::capability::Capability;
use crate::cli::command::Command;
use crate::config::ServiceKind;

/// Canonical friendly CLI verb → snake_case registry action name, in declaration
/// order. SSOT for USAGE rendering and the mechanical CLI↔MCP parity test
/// (`tests/parity.rs`). One entry per Requests curated descriptor.
pub const VERBS: &[(&str, &str)] = &[
    ("requests", "requests"),
    ("request", "request_create"),
    ("approve", "request_approve"),
    ("decline", "request_decline"),
    ("search", "request_search"),
];

/// Try to parse `verb [rest]` as a Requests curated command for `kind`.
///
/// Returns `Ok(Some(cmd))` when `verb` is a known Requests verb, `Ok(None)` when
/// it is not (so the router falls through to its generic passthrough / "unknown
/// command" handling), and `Err` when the verb matched but its flags were
/// invalid.
pub fn parse(kind: ServiceKind, verb: &str, rest: &[String]) -> Result<Option<Command>> {
    // Single verb→action resolution against `VERBS` (the SSOT). `None` => the verb
    // isn't a Requests curated verb, so fall through to the router.
    let Some(action) = resolve(verb)? else {
        return Ok(None);
    };

    // Branch on the PARSING SHAPE only — keyed by the friendly verb, not a second
    // verb→action mapping.
    match verb {
        "requests" => parse_requests(kind, action, rest).map(Some),
        "request" => parse_create(kind, action, rest).map(Some),
        "approve" => parse_id_action(kind, action, "approve", rest).map(Some),
        "decline" => parse_id_action(kind, action, "decline", rest).map(Some),
        "search" => parse_search(kind, action, rest).map(Some),
        // `resolve` only returns `Some` for verbs in `VERBS`; all are handled above.
        _ => unreachable!("VERBS verb `{verb}` has no parse arm"),
    }
}

/// `overseerr requests [--filter F] [--take N] [--skip N]` → `requests`.
fn parse_requests(kind: ServiceKind, action: &'static str, rest: &[String]) -> Result<Command> {
    let mut params = base_params(kind);

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            flag @ ("--filter" | "--take" | "--skip") => {
                let value = take_value(rest, &mut i, flag)?;
                let key = &flag[2..];
                if params.insert(key.into(), json!(value)).is_some() {
                    return Err(anyhow!("requests received duplicate {flag}"));
                }
            }
            other => return Err(anyhow!("requests does not accept argument `{other}`")),
        }
        i += 1;
    }

    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

/// `overseerr request --media-type T --media-id ID [--season N ...] [--confirm]`
/// → `request_create`.
fn parse_create(kind: ServiceKind, action: &'static str, rest: &[String]) -> Result<Command> {
    let mut params = base_params(kind);
    let mut seasons: Vec<String> = Vec::new();

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--confirm" | "--yes" => {
                params.insert("confirm".into(), json!(true));
            }
            "--season" | "--seasons" => {
                let value = take_value(rest, &mut i, "--season")?;
                seasons.push(value);
            }
            flag @ ("--media-type" | "--type") => {
                let value = take_value(rest, &mut i, flag)?;
                if params.insert("media_type".into(), json!(value)).is_some() {
                    return Err(anyhow!("request received duplicate --media-type"));
                }
            }
            flag @ ("--media-id" | "--id") => {
                let value = take_value(rest, &mut i, flag)?;
                if params.insert("media_id".into(), json!(value)).is_some() {
                    return Err(anyhow!("request received duplicate --media-id"));
                }
            }
            other => return Err(anyhow!("request does not accept argument `{other}`")),
        }
        i += 1;
    }

    if !params.contains_key("media_type") {
        return Err(anyhow!("request requires --media-type (movie|tv)"));
    }
    if !params.contains_key("media_id") {
        return Err(anyhow!("request requires --media-id (a TMDB id)"));
    }
    if !seasons.is_empty() {
        params.insert("seasons".into(), json!(seasons));
    }

    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

/// `overseerr {approve,decline} --id N [--confirm]` → `request_{approve,decline}`.
fn parse_id_action(
    kind: ServiceKind,
    action: &'static str,
    verb: &str,
    rest: &[String],
) -> Result<Command> {
    let mut params = base_params(kind);

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--confirm" | "--yes" => {
                params.insert("confirm".into(), json!(true));
            }
            "--id" => {
                let value = take_value(rest, &mut i, "--id")?;
                if params.insert("id".into(), json!(value)).is_some() {
                    return Err(anyhow!("{verb} received duplicate --id"));
                }
            }
            other => return Err(anyhow!("{verb} does not accept argument `{other}`")),
        }
        i += 1;
    }

    if !params.contains_key("id") {
        return Err(anyhow!("{verb} requires --id (a request id)"));
    }

    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

/// `overseerr search --query X` → `request_search`.
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

/// Resolve a friendly CLI `verb` against [`VERBS`] (the SSOT) to its Requests
/// curated action name.
///
/// Returns `Ok(None)` when `verb` is not a Requests curated verb (the caller
/// falls through), and an `Err` only if the VERBS↔registry wiring is broken — an
/// invariant guarded by `tests/parity.rs`, surfaced here as a clean parse error
/// instead of a panic.
fn resolve(verb: &str) -> Result<Option<&'static str>> {
    let Some((_, action)) = VERBS.iter().find(|(cli_verb, _)| *cli_verb == verb) else {
        return Ok(None);
    };
    curated_command(action)
        .filter(|cmd| cmd.capability == Capability::Requests)
        .map(|cmd| Some(cmd.name))
        .ok_or_else(|| anyhow!("internal: verb `{verb}` has no Requests descriptor"))
}

#[cfg(test)]
#[path = "requests_tests.rs"]
mod tests;
