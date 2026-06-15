//! CLI parse module for ArrManager curated commands (sonarr/radarr).
//!
//! Thin shim: it only recognises the curated verbs, maps the kebab-case CLI verb
//! to the snake_case registry/MCP action name, and assembles the JSON `params`
//! object (positional `service` + any flags) into a [`Command::Curated`]. All
//! business logic lives in `crate::app::arr`; validation/scope/dispatch flow
//! through the shared `execute_service_action` path, exactly like the MCP shim.

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use crate::actions::curated_command;
use crate::actions::registry::CommandDescriptor;
use crate::capability::Capability;
use crate::cli::command::Command;
use crate::cli::parse::reject_args;
use crate::config::ServiceKind;

/// Canonical friendly CLI verb → snake_case registry action name, in declaration
/// order. This is the SSOT the USAGE renderer and the mechanical CLI↔MCP parity
/// test (`tests/parity.rs`) consume; `parse` below recognises these primary verbs
/// (plus a few flag-level aliases). One entry per curated descriptor for this
/// capability.
pub const VERBS: &[(&str, &str)] = &[
    ("quality-profiles", "quality_profiles"),
    ("list", "list"),
    ("wanted", "wanted"),
    ("queue", "queue"),
    ("history", "history"),
    ("rootfolders", "rootfolders"),
    ("health", "health"),
    ("set-quality", "set_quality"),
    ("search", "search"),
    ("refresh", "refresh"),
    ("monitor", "monitor"),
    ("unmonitor", "unmonitor"),
    ("add", "add"),
    ("delete", "delete"),
];

/// Try to parse `verb [rest]` as an ArrManager curated command for `kind`.
///
/// Returns `Ok(Some(cmd))` when `verb` is a known ArrManager curated verb,
/// `Ok(None)` when it is not (so the router can fall through to its
/// "unknown command" error), and `Err` when the verb matched but its flags were
/// invalid. The read commands (`mutates == false`) take only the positional
/// service (no flags); the flag-bearing write/intent verbs are marshalled by
/// [`parse_write`].
pub fn parse(kind: ServiceKind, verb: &str, rest: &[String]) -> Result<Option<Command>> {
    // Single verb→descriptor resolution against `VERBS` (the SSOT). `None` => the
    // verb isn't an ArrManager curated verb, so fall through to the router.
    let Some(descriptor) = resolve(verb)? else {
        return Ok(None);
    };

    // Branch on the PARSING SHAPE only: writes are flag-bearing, reads are not.
    // The read/write split is read straight from the descriptor's `mutates` flag
    // rather than a second verb list.
    if descriptor.mutates {
        return parse_write(kind, verb, descriptor.name, rest).map(Some);
    }

    // Read commands accept no flags beyond the positional service.
    reject_args(rest, verb)?;

    let params = json!({ "service": kind.as_str() });
    Ok(Some(Command::Curated {
        action: descriptor.name,
        params,
    }))
}

/// Resolve a friendly CLI `verb` against [`VERBS`] (the SSOT) to its ArrManager
/// curated descriptor.
///
/// Returns `Ok(None)` when `verb` is not an ArrManager curated verb (the caller
/// falls through), and an `Err` only if the VERBS↔registry wiring is broken — an
/// invariant guarded by `tests/parity.rs`, surfaced here as a clean parse error
/// instead of a panic.
fn resolve(verb: &str) -> Result<Option<&'static CommandDescriptor>> {
    let Some((_, action)) = VERBS.iter().find(|(cli_verb, _)| *cli_verb == verb) else {
        return Ok(None);
    };
    curated_command(action)
        .filter(|cmd| cmd.capability == Capability::ArrManager)
        .map(Some)
        .ok_or_else(|| anyhow!("internal: verb `{verb}` has no ArrManager descriptor"))
}

/// Parse a write verb's flags into the JSON `params` object the registry handler
/// consumes. Thin: it only marshals CLI flags → JSON — all dry-run / selection /
/// count-cap logic lives in `crate::app::arr::write`.
fn parse_write(
    kind: ServiceKind,
    verb: &str,
    action: &'static str,
    rest: &[String],
) -> Result<Command> {
    let mut params = Map::new();
    params.insert("service".into(), json!(kind.as_str()));
    let mut titles: Vec<String> = Vec::new();
    let mut ids: Vec<String> = Vec::new();

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            // bare boolean flags
            "--confirm" | "--yes" => {
                params.insert("confirm".into(), json!(true));
            }
            "--bulk" => {
                params.insert("bulk".into(), json!(true));
            }
            "--delete-files" => {
                params.insert("delete_files".into(), json!(true));
            }
            // value flags
            flag @ ("--from" | "--to" | "--id" | "--title" | "--term" | "--quality-profile"
            | "--root-folder") => {
                i += 1;
                let value = rest
                    .get(i)
                    .filter(|v| !v.starts_with("--"))
                    .cloned()
                    .ok_or_else(|| anyhow!("{verb} requires a value after {flag}"))?;
                match flag {
                    "--from" => insert_unique(&mut params, "from", json!(value), verb, flag)?,
                    "--to" => insert_unique(&mut params, "to", json!(value), verb, flag)?,
                    "--term" => insert_unique(&mut params, "term", json!(value), verb, flag)?,
                    "--quality-profile" => {
                        insert_unique(&mut params, "quality_profile", json!(value), verb, flag)?
                    }
                    "--root-folder" => {
                        insert_unique(&mut params, "root_folder", json!(value), verb, flag)?
                    }
                    // `--title` and `--id` are repeatable selectors.
                    "--title" => titles.push(value),
                    "--id" => ids.push(value),
                    _ => unreachable!(),
                }
            }
            other => return Err(anyhow!("{verb} does not accept argument `{other}`")),
        }
        i += 1;
    }

    if !titles.is_empty() {
        params.insert("title".into(), json!(titles));
    }
    if !ids.is_empty() {
        // `delete` targets a single item and its handler reads the singular `id`
        // key (matching the descriptor's required_params); the other write verbs
        // use `--id` as a repeatable selector under the plural `ids` key.
        if action == "delete" {
            if ids.len() > 1 {
                return Err(anyhow!("{verb} accepts a single --id"));
            }
            params.insert("id".into(), json!(ids[0]));
        } else {
            params.insert("ids".into(), json!(ids));
        }
    }

    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

/// Insert a single-valued flag, rejecting a duplicate.
fn insert_unique(
    params: &mut Map<String, Value>,
    key: &str,
    value: Value,
    verb: &str,
    flag: &str,
) -> Result<()> {
    if params.insert(key.into(), value).is_some() {
        return Err(anyhow!("{verb} received duplicate {flag}"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "arr_tests.rs"]
mod tests;
