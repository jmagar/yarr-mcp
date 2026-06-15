//! CLI parse module for Indexer curated commands (prowlarr).
//!
//! Thin shim: it recognises the curated verbs, maps the friendly kebab CLI verb
//! to the snake_case registry/MCP action name (`search` → `indexer_search` so it
//! does not collide with the ArrManager `search` command), assembles the JSON
//! `params` object (positional `service` + any flags) into a [`Command::Curated`],
//! and rejects unknown flags. All business logic lives in `crate::app::indexer`;
//! validation/scope/dispatch flow through the shared `execute_service_action`
//! path, exactly like the MCP shim.

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

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
    // Flag-bearing verbs (search, test) have a dedicated path.
    match verb {
        "search" => return parse_search(kind, rest).map(Some),
        "test" => return parse_test(kind, rest).map(Some),
        _ => {}
    }

    let action = match verb {
        // kebab CLI verb ↔ snake_case registry name.
        "indexers" => "indexers",
        "stats" => "indexer_stats",
        _ => return Ok(None),
    };

    let descriptor = resolve(action);
    reject_args(rest, verb)?;
    Ok(Some(Command::Curated {
        action: descriptor,
        params: json!({ "service": kind.as_str() }),
    }))
}

/// `prowlarr search --query X [--id N ...]` → `indexer_search`.
fn parse_search(kind: ServiceKind, rest: &[String]) -> Result<Command> {
    let descriptor = resolve("indexer_search");
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
        action: descriptor,
        params: Value::Object(params),
    })
}

/// `prowlarr test [--id N] [--confirm]` → `indexer_test`.
fn parse_test(kind: ServiceKind, rest: &[String]) -> Result<Command> {
    let descriptor = resolve("indexer_test");
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
        action: descriptor,
        params: Value::Object(params),
    })
}

/// Resolve a registry name to its static descriptor name, asserting the
/// Indexer-capability wiring is intact.
fn resolve(action: &str) -> &'static str {
    curated_command(action)
        .filter(|cmd| cmd.capability == Capability::Indexer)
        .expect("Indexer curated verb must resolve to an Indexer descriptor")
        .name
}

#[cfg(test)]
#[path = "indexer_tests.rs"]
mod tests;
