//! CLI router: disambiguates `token1` and produces a [`Command`].
//!
//! # Grammar (architecture F3-a)
//!
//! ```text
//! rustarr <token1> [rest...]
//! ```
//!
//! `token1` is resolved by **disjoint sets**:
//!
//!   1. If `token1` in [`INFRA_VERBS`] → parse an **infra, service-less** command
//!      (`help`, `codemode`, `snippet`, `doctor`, `watch`, `setup`). `serve`/`mcp` are
//!      also infra verbs but are intercepted as run *modes* in `main.rs` before
//!      `parse_args` runs — they never reach this router, yet are listed in
//!      [`INFRA_VERBS`] so they cannot be shadowed by a service name.
//!   2. Else resolve `token1` as a [`ServiceKind`] via `FromStr` and parse the
//!      remaining `<command> [flags]` through [`parse_capability_command`].
//!   3. Unknown `token1` → an error listing valid services + infra verbs.
//!
//! The infra-verb set and the `ServiceKind` name set are **disjoint** by
//! construction; `router_tests::infra_verbs_disjoint_from_service_kinds`
//! asserts this so a future kind can never collide with an infra verb.

use anyhow::{Result, anyhow};

use super::command::Command;
use super::parse::{parse_bool_flag, parse_passthrough_flags, parse_watch_flags, reject_args};
use super::setup::SetupCommand;
use crate::capability::Capability;
use crate::config::ServiceKind;

/// Infra verbs — service-less top-level commands. Disjoint from `ServiceKind`
/// names (asserted in tests). `serve`/`mcp` are handled as run modes in
/// `main.rs` but kept here to reserve the names.
pub const INFRA_VERBS: &[&str] = &[
    "help", "codemode", "snippet", "doctor", "watch", "setup", "serve", "mcp",
];

/// True if `token` is an infra verb.
pub fn is_infra_verb(token: &str) -> bool {
    INFRA_VERBS.contains(&token)
}

/// Resolve a full argv (already stripped of the binary name) into a [`Command`].
///
/// Returns `Ok(None)` only for the empty argv (no subcommand) so `main.rs` can
/// fall through to its "Unknown command" path, matching prior behaviour.
pub fn route(args: &[String]) -> Result<Option<Command>> {
    let [token1, rest @ ..] = args else {
        return Ok(None);
    };

    if is_infra_verb(token1) {
        return parse_infra_command(token1, rest).map(Some);
    }

    match token1.parse::<ServiceKind>() {
        Ok(kind) => {
            let [verb, verb_rest @ ..] = rest else {
                return Err(anyhow!(
                    "service `{token1}` requires a command (e.g. status, get, post)"
                ));
            };
            parse_capability_command(kind, kind.capability(), verb, verb_rest).map(Some)
        }
        Err(_) => Err(unknown_token1_error(token1)),
    }
}

// ── infra branch ────────────────────────────────────────────────────────────────

/// Parse a service-less infra command. `serve`/`mcp` never arrive here (handled
/// as run modes in `main.rs`); they are rejected defensively.
fn parse_infra_command(verb: &str, rest: &[String]) -> Result<Command> {
    match verb {
        "help" => {
            reject_args(rest, "help")?;
            Ok(Command::Help)
        }
        "codemode" => parse_codemode_command(rest),
        "snippet" => parse_snippet_command(rest),
        "doctor" => {
            let json = parse_bool_flag(rest, "doctor", "--json")?;
            Ok(Command::Doctor { json })
        }
        "watch" => {
            let (url, interval_arg) = parse_watch_flags(rest)?;
            let interval = match interval_arg {
                Some(v) => v.parse().map_err(|_| {
                    anyhow!("watch --interval must be a positive integer number of seconds")
                })?,
                None => 10,
            };
            if interval == 0 {
                return Err(anyhow!(
                    "watch --interval must be a positive integer number of seconds"
                ));
            }
            Ok(Command::Watch { url, interval })
        }
        "setup" => parse_setup_command(rest),
        "serve" | "mcp" => Err(anyhow!(
            "`{verb}` is a run mode handled before CLI parsing; this should be unreachable"
        )),
        other => Err(anyhow!("unknown infra command `{other}`")),
    }
}

/// Parse `codemode --code <JS>` or `codemode --file <PATH>` (mutually exclusive).
/// The script is read from `--file` here so the rest of the pipeline only ever
/// sees the resolved code string.
fn parse_codemode_command(rest: &[String]) -> Result<Command> {
    let mut code: Option<String> = None;
    let mut file: Option<String> = None;
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--code" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("codemode --code requires a JavaScript argument"))?;
                code = Some(value.clone());
            }
            "--file" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("codemode --file requires a path argument"))?;
                file = Some(value.clone());
            }
            other => {
                return Err(anyhow!(
                    "unknown codemode flag `{other}` (use --code or --file)"
                ));
            }
        }
    }
    let code = match (code, file) {
        (Some(_), Some(_)) => return Err(anyhow!("codemode: pass only one of --code or --file")),
        (Some(code), None) => code,
        (None, Some(path)) => std::fs::read_to_string(&path)
            .map_err(|e| anyhow!("codemode --file: could not read `{path}`: {e}"))?,
        (None, None) => return Err(anyhow!("codemode requires --code <JS> or --file <PATH>")),
    };
    Ok(Command::CodeMode { code })
}

/// Take the value following a flag, e.g. `--name foo`.
fn flag_value(iter: &mut std::slice::Iter<String>, flag: &str) -> Result<String> {
    iter.next()
        .cloned()
        .ok_or_else(|| anyhow!("{flag} requires a value"))
}

/// Parse `snippet list|save|run|delete ...`.
fn parse_snippet_command(rest: &[String]) -> Result<Command> {
    let [sub, flags @ ..] = rest else {
        return Err(anyhow!(
            "snippet requires a subcommand (list, save, run, delete)"
        ));
    };
    match sub.as_str() {
        "list" => {
            reject_args(flags, "snippet list")?;
            Ok(Command::SnippetList)
        }
        "save" => {
            let (mut name, mut code, mut file, mut description) = (None, None, None, None);
            let mut iter = flags.iter();
            while let Some(flag) = iter.next() {
                match flag.as_str() {
                    "--name" => name = Some(flag_value(&mut iter, "--name")?),
                    "--code" => code = Some(flag_value(&mut iter, "--code")?),
                    "--file" => file = Some(flag_value(&mut iter, "--file")?),
                    "--description" => description = Some(flag_value(&mut iter, "--description")?),
                    other => return Err(anyhow!("unknown snippet save flag `{other}`")),
                }
            }
            let name = name.ok_or_else(|| anyhow!("snippet save requires --name"))?;
            let code = match (code, file) {
                (Some(_), Some(_)) => {
                    return Err(anyhow!("snippet save: pass only one of --code or --file"));
                }
                (Some(code), None) => code,
                (None, Some(path)) => std::fs::read_to_string(&path)
                    .map_err(|e| anyhow!("snippet save --file: could not read `{path}`: {e}"))?,
                (None, None) => {
                    return Err(anyhow!(
                        "snippet save requires --code <JS> or --file <PATH>"
                    ));
                }
            };
            Ok(Command::SnippetSave {
                name,
                code,
                description,
            })
        }
        "run" => {
            let (mut name, mut input_str, mut input_file) = (None, None, None);
            let mut iter = flags.iter();
            while let Some(flag) = iter.next() {
                match flag.as_str() {
                    "--name" => name = Some(flag_value(&mut iter, "--name")?),
                    "--input" => input_str = Some(flag_value(&mut iter, "--input")?),
                    "--input-file" => input_file = Some(flag_value(&mut iter, "--input-file")?),
                    other => return Err(anyhow!("unknown snippet run flag `{other}`")),
                }
            }
            let name = name.ok_or_else(|| anyhow!("snippet run requires --name"))?;
            let input_text = match (input_str, input_file) {
                (Some(_), Some(_)) => {
                    return Err(anyhow!(
                        "snippet run: pass only one of --input or --input-file"
                    ));
                }
                (Some(s), None) => Some(s),
                (None, Some(path)) => Some(std::fs::read_to_string(&path).map_err(|e| {
                    anyhow!("snippet run --input-file: could not read `{path}`: {e}")
                })?),
                (None, None) => None,
            };
            let input = match input_text {
                Some(text) => serde_json::from_str(&text)
                    .map_err(|e| anyhow!("snippet run --input must be valid JSON: {e}"))?,
                None => serde_json::Value::Null,
            };
            Ok(Command::SnippetRun { name, input })
        }
        "delete" => {
            let mut name = None;
            let mut iter = flags.iter();
            while let Some(flag) = iter.next() {
                match flag.as_str() {
                    "--name" => name = Some(flag_value(&mut iter, "--name")?),
                    other => return Err(anyhow!("unknown snippet delete flag `{other}`")),
                }
            }
            let name = name.ok_or_else(|| anyhow!("snippet delete requires --name"))?;
            Ok(Command::SnippetDelete { name })
        }
        other => Err(anyhow!(
            "unknown snippet subcommand `{other}` (list, save, run, delete)"
        )),
    }
}

fn parse_setup_command(rest: &[String]) -> Result<Command> {
    match rest {
        [action, flags @ ..] if action == "check" => {
            reject_args(flags, "setup check")?;
            Ok(Command::Setup(SetupCommand::Check))
        }
        [action, flags @ ..] if action == "repair" => {
            reject_args(flags, "setup repair")?;
            Ok(Command::Setup(SetupCommand::Repair))
        }
        [action, flags @ ..] if action == "install" => {
            reject_args(flags, "setup install")?;
            Ok(Command::Setup(SetupCommand::Install))
        }
        [action, flags @ ..] if action == "plugin-hook" => {
            let no_repair = parse_bool_flag(flags, "setup plugin-hook", "--no-repair")?;
            Ok(Command::Setup(SetupCommand::PluginHook { no_repair }))
        }
        [] => Err(anyhow!(
            "setup requires a subcommand (check, repair, install, plugin-hook)"
        )),
        [other, ..] => Err(anyhow!("unknown setup subcommand `{other}`")),
    }
}

// ── capability branch ─────────────────────────────────────────────────────────

/// Per-capability command-parse HOOK. Two-stage:
///
///   1. **Curated dispatch first** — `capability` selects the matching
///      `src/cli/commands/<cap>.rs` parser, which returns `Some(Command)` for a
///      verb it owns or `None` to fall through.
///   2. **Generic passthrough fallback** — if no curated parser claimed the verb,
///      the generic surface common to every capability handles it: `status`,
///      `get`, `post`, `put`, `delete`. Any other verb yields a clear "unknown
///      command for `<service>` error.
pub fn parse_capability_command(
    kind: ServiceKind,
    capability: Capability,
    verb: &str,
    rest: &[String],
) -> Result<Command> {
    // Curated, capability-scoped verbs first. Each capability bead adds a parse
    // module under `cli/commands/<cap>.rs` and a dispatch arm here; a module
    // returns `Ok(None)` for a verb it doesn't own so we fall through to the
    // generic passthrough verbs below.
    if let Some(command) = match capability {
        // Spec-backed capabilities have no curated CLI verbs (generated ops are
        // MCP/Code-Mode only); they fall through to the generic passthrough verbs.
        Capability::DownloadClient => super::commands::download::parse(kind, verb, rest)?,
        Capability::Stats => super::commands::stats::parse(kind, verb, rest)?,
        Capability::Subtitles => super::commands::subtitles::parse(kind, verb, rest)?,
        Capability::Trace => super::commands::trace::parse(kind, verb, rest)?,
        _ => None,
    } {
        return Ok(command);
    }

    let service = kind.as_str().to_string();
    match verb {
        "status" => {
            reject_args(rest, "status")?;
            Ok(Command::Status { service })
        }
        "get" => {
            let flags = parse_passthrough_flags(rest, "get", false, false)?;
            if flags.body.is_some() {
                return Err(anyhow!("get does not accept --body"));
            }
            Ok(Command::Get {
                service,
                path: flags.path,
            })
        }
        "post" => {
            // `post` is non-destructive and runs immediately. `--confirm`/`--yes`
            // are still accepted (allow_confirm=true) as a harmless no-op so
            // existing scripts that passed them don't break; the flag is ignored.
            let flags = parse_passthrough_flags(rest, "post", true, true)?;
            Ok(Command::Post {
                service,
                path: flags.path,
                body: flags.body.unwrap_or(serde_json::Value::Null),
            })
        }
        "put" => {
            // See `post`: non-destructive, `--confirm` accepted as a no-op.
            let flags = parse_passthrough_flags(rest, "put", true, true)?;
            Ok(Command::Put {
                service,
                path: flags.path,
                body: flags.body.unwrap_or(serde_json::Value::Null),
            })
        }
        "delete" => {
            let flags = parse_passthrough_flags(rest, "delete", false, true)?;
            Ok(Command::Delete {
                service,
                path: flags.path,
                body: flags.body,
                confirm: flags.confirm,
            })
        }
        // `op <name> [--args JSON] [--confirm]` — invoke a generated operation for a
        // spec-backed kind directly (the test-harness / operator path; reads run
        // immediately, DELETE ops require --confirm).
        "op" => parse_op_command(service, rest),
        // Unknown verb: not a generic passthrough verb and not claimed by the
        // capability's curated parser above.
        other => Err(anyhow!(
            "unknown command `{other}` for service `{}`",
            kind.as_str()
        )),
    }
}

/// Parse `op <name> [--args JSON] [--confirm]`. The first positional is the
/// operation name; `--args` carries a JSON object of path/query params + `body`.
fn parse_op_command(service: String, rest: &[String]) -> Result<Command> {
    let [op, flags @ ..] = rest else {
        return Err(anyhow!(
            "op requires an operation name (e.g. `rustarr {service} op get_series`)"
        ));
    };
    let mut args = serde_json::Value::Object(serde_json::Map::new());
    let mut confirm = false;
    let mut iter = flags.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--args" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| anyhow!("op --args requires a JSON object argument"))?;
                args = serde_json::from_str(raw)
                    .map_err(|e| anyhow!("op --args must be valid JSON: {e}"))?;
                if !args.is_object() {
                    return Err(anyhow!("op --args must be a JSON object"));
                }
            }
            "--confirm" | "--yes" => confirm = true,
            other => {
                return Err(anyhow!(
                    "unknown op flag `{other}` (use --args or --confirm)"
                ));
            }
        }
    }
    Ok(Command::Op {
        service,
        op: op.clone(),
        args,
        confirm,
    })
}

// ── errors ──────────────────────────────────────────────────────────────────────

fn unknown_token1_error(token1: &str) -> anyhow::Error {
    let services = ServiceKind::ALL
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let infra = INFRA_VERBS.join(", ");
    anyhow!("unknown command `{token1}`.\n  services: {services}\n  infra commands: {infra}")
}

#[cfg(test)]
#[path = "router_tests.rs"]
mod tests;
