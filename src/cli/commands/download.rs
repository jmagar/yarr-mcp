//! CLI parse module for DownloadClient curated commands (sabnzbd, qbittorrent).
//!
//! Thin shim: it recognises the curated verbs, maps the friendly kebab CLI verb
//! to the snake_case registry/MCP action name (`queue` → `download_queue` so it
//! does not collide with the ArrManager `queue` command — registry action names
//! are globally unique), assembles the JSON `params` object (the positional
//! `service` plus any flags) into a [`Command::Curated`], and rejects unknown
//! flags. All business logic lives in `crate::app::download`; validation, scope,
//! and dispatch flow through the shared `execute_service_action` path, exactly
//! like the MCP shim.

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::actions::curated_command;
use crate::capability::Capability;
use crate::cli::command::Command;
use crate::cli::parse::reject_args;
use crate::config::ServiceKind;

/// Canonical friendly CLI verb → snake_case registry action name, in declaration
/// order. SSOT for USAGE rendering and the mechanical CLI↔MCP parity test
/// (`tests/parity.rs`). One entry per DownloadClient curated descriptor.
pub const VERBS: &[(&str, &str)] = &[
    ("queue", "download_queue"),
    ("add", "download_add"),
    ("pause", "download_pause"),
    ("resume", "download_resume"),
    ("remove", "download_remove"),
];

/// Try to parse `verb [rest]` as a DownloadClient curated command for `kind`.
///
/// Returns `Ok(Some(cmd))` when `verb` is a known download verb, `Ok(None)` when
/// it is not (so the router falls through to its generic passthrough / "unknown
/// command" handling), and `Err` when the verb matched but its flags were
/// invalid.
pub fn parse(kind: ServiceKind, verb: &str, rest: &[String]) -> Result<Option<Command>> {
    match verb {
        "queue" => {
            let descriptor = resolve("download_queue");
            reject_args(rest, verb)?;
            Ok(Some(Command::Curated {
                action: descriptor,
                params: json!({ "service": kind.as_str() }),
            }))
        }
        "add" => parse_add(kind, rest).map(Some),
        "pause" => parse_state(kind, "download_pause", "pause", rest).map(Some),
        "resume" => parse_state(kind, "download_resume", "resume", rest).map(Some),
        "remove" => parse_remove(kind, rest).map(Some),
        _ => Ok(None),
    }
}

/// `<svc> add --url X [--confirm]` → `download_add`.
fn parse_add(kind: ServiceKind, rest: &[String]) -> Result<Command> {
    let descriptor = resolve("download_add");
    let mut params = base_params(kind);
    let mut url: Option<String> = None;

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--confirm" | "--yes" => {
                params.insert("confirm".into(), json!(true));
            }
            flag @ ("--url" | "--magnet") => {
                let value = take_value(rest, &mut i, flag)?;
                if url.replace(value).is_some() {
                    return Err(anyhow!("add received duplicate --url"));
                }
            }
            other => return Err(anyhow!("add does not accept argument `{other}`")),
        }
        i += 1;
    }

    let url = url.ok_or_else(|| anyhow!("add requires --url (a URL or magnet link)"))?;
    params.insert("url".into(), json!(url));
    Ok(Command::Curated {
        action: descriptor,
        params: Value::Object(params),
    })
}

/// `<svc> {pause,resume} [--id N | --hash H] [--confirm]` → `download_{pause,resume}`.
fn parse_state(kind: ServiceKind, action: &str, verb: &str, rest: &[String]) -> Result<Command> {
    let descriptor = resolve(action);
    let mut params = base_params(kind);

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--confirm" | "--yes" => {
                params.insert("confirm".into(), json!(true));
            }
            flag @ ("--id" | "--hash") => {
                let key = if flag == "--hash" { "hash" } else { "id" };
                let value = take_value(rest, &mut i, flag)?;
                if params.insert(key.into(), json!(value)).is_some() {
                    return Err(anyhow!("{verb} received duplicate {flag}"));
                }
            }
            other => return Err(anyhow!("{verb} does not accept argument `{other}`")),
        }
        i += 1;
    }

    Ok(Command::Curated {
        action: descriptor,
        params: Value::Object(params),
    })
}

/// `<svc> remove (--id N | --hash H) [--delete-files] [--confirm]` → `download_remove`.
fn parse_remove(kind: ServiceKind, rest: &[String]) -> Result<Command> {
    let descriptor = resolve("download_remove");
    let mut params = base_params(kind);

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--confirm" | "--yes" => {
                params.insert("confirm".into(), json!(true));
            }
            "--delete-files" => {
                params.insert("delete_files".into(), json!(true));
            }
            flag @ ("--id" | "--hash") => {
                let key = if flag == "--hash" { "hash" } else { "id" };
                let value = take_value(rest, &mut i, flag)?;
                if params.insert(key.into(), json!(value)).is_some() {
                    return Err(anyhow!("remove received duplicate {flag}"));
                }
            }
            other => return Err(anyhow!("remove does not accept argument `{other}`")),
        }
        i += 1;
    }

    if !params.contains_key("id") && !params.contains_key("hash") {
        return Err(anyhow!(
            "remove requires --id (nzo_id) or --hash (torrent hash)"
        ));
    }
    Ok(Command::Curated {
        action: descriptor,
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

/// Resolve a registry name to its static descriptor name, asserting the
/// DownloadClient-capability wiring is intact.
fn resolve(action: &str) -> &'static str {
    curated_command(action)
        .filter(|cmd| cmd.capability == Capability::DownloadClient)
        .expect("DownloadClient curated verb must resolve to a DownloadClient descriptor")
        .name
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod tests;
