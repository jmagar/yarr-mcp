//! CLI parse module for Indexer curated commands (prowlarr).
//!
//! Thin shim: it recognises the curated verbs, maps the friendly kebab CLI verb
//! to the snake_case registry/MCP action name (`search` → `indexer_search` so it
//! does not collide with the ArrManager `search` command), assembles the JSON
//! `params` object (positional `service` + any flags) into a [`Command::Curated`],
//! and rejects unknown flags. All business logic lives in `crate::app::indexer`;
//! validation/scope/dispatch flow through the shared `execute_service_action`
//! path, exactly like the MCP shim.

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use crate::actions::curated_command;
use crate::capability::Capability;
use crate::cli::command::Command;
use crate::cli::parse::reject_args;
use crate::config::ServiceKind;

/// Canonical friendly CLI verb → snake_case registry action name, in declaration
/// order. SSOT for USAGE rendering and the mechanical CLI↔MCP parity test
/// (`tests/parity.rs`). One entry per Indexer curated descriptor.
pub const VERBS: &[(&str, &str)] = &[
    ("indexers", "indexers"),
    ("search", "indexer_search"),
    ("stats", "indexer_stats"),
    ("test", "indexer_test"),
];

/// Try to parse `verb [rest]` as an Indexer curated command for `kind`.
///
/// Returns `Ok(Some(cmd))` when `verb` is a known Indexer curated verb,
/// `Ok(None)` when it is not (so the router falls through to its generic
/// passthrough / "unknown command" handling), and `Err` when the verb matched
/// but its flags were invalid.
pub fn parse(kind: ServiceKind, verb: &str, rest: &[String]) -> Result<Option<Command>> {
    // Single verb→action resolution against `VERBS` (the SSOT). `None` => the verb
    // isn't an Indexer curated verb, so fall through to the router.
    let Some(action) = resolve(verb)? else {
        return Ok(None);
    };

    // Branch on the PARSING SHAPE only — keyed by the friendly verb, not a second
    // verb→action mapping.
    match verb {
        "search" => parse_search(kind, action, rest).map(Some),
        "test" => parse_test(kind, action, rest).map(Some),
        // No-flag read verbs (indexers, stats).
        _ => {
            reject_args(rest, verb)?;
            Ok(Some(Command::Curated {
                action,
                params: json!({ "service": kind.as_str() }),
            }))
        }
    }
}

/// `prowlarr search --query X [--id N ...]` → `indexer_search`.
fn parse_search(kind: ServiceKind, action: &'static str, rest: &[String]) -> Result<Command> {
    let mut params = Map::new();
    params.insert("service".into(), json!(kind.as_str()));
    let mut ids: Vec<String> = Vec::new();
    let mut query: Option<String> = None;

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            flag @ ("--query" | "--id") => {
                i += 1;
                let value = rest
                    .get(i)
                    .filter(|v| !v.starts_with("--"))
                    .cloned()
                    .ok_or_else(|| anyhow!("search requires a value after {flag}"))?;
                match flag {
                    "--query" => {
                        if query.replace(value).is_some() {
                            return Err(anyhow!("search received duplicate --query"));
                        }
                    }
                    "--id" => ids.push(value),
                    _ => unreachable!(),
                }
            }
            other => return Err(anyhow!("search does not accept argument `{other}`")),
        }
        i += 1;
    }

    let query = query.ok_or_else(|| anyhow!("search requires --query"))?;
    params.insert("query".into(), json!(query));
    if !ids.is_empty() {
        params.insert("ids".into(), json!(ids));
    }
    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

/// `prowlarr test [--id N] [--confirm]` → `indexer_test`.
fn parse_test(kind: ServiceKind, action: &'static str, rest: &[String]) -> Result<Command> {
    let mut params = Map::new();
    params.insert("service".into(), json!(kind.as_str()));

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--confirm" | "--yes" => {
                params.insert("confirm".into(), json!(true));
            }
            "--id" => {
                i += 1;
                let value = rest
                    .get(i)
                    .filter(|v| !v.starts_with("--"))
                    .cloned()
                    .ok_or_else(|| anyhow!("test requires a value after --id"))?;
                if params.insert("id".into(), json!(value)).is_some() {
                    return Err(anyhow!("test received duplicate --id"));
                }
            }
            other => return Err(anyhow!("test does not accept argument `{other}`")),
        }
        i += 1;
    }

    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

/// Resolve a friendly CLI `verb` against [`VERBS`] (the SSOT) to its Indexer
/// curated action name.
///
/// Returns `Ok(None)` when `verb` is not an Indexer curated verb (the caller
/// falls through), and an `Err` only if the VERBS↔registry wiring is broken — an
/// invariant guarded by `tests/parity.rs`, surfaced here as a clean parse error
/// instead of a panic.
fn resolve(verb: &str) -> Result<Option<&'static str>> {
    let Some((_, action)) = VERBS.iter().find(|(cli_verb, _)| *cli_verb == verb) else {
        return Ok(None);
    };
    curated_command(action)
        .filter(|cmd| cmd.capability == Capability::Indexer)
        .map(|cmd| Some(cmd.name))
        .ok_or_else(|| anyhow!("internal: verb `{verb}` has no Indexer descriptor"))
}

#[cfg(test)]
#[path = "indexer_tests.rs"]
mod tests;
