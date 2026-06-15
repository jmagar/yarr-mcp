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
//!      (`integrations`, `help`, `doctor`, `watch`, `setup`). `serve`/`mcp` are
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
    "integrations",
    "help",
    "doctor",
    "watch",
    "setup",
    "serve",
    "mcp",
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
        "integrations" => {
            reject_args(rest, "integrations")?;
            Ok(Command::Integrations)
        }
        "help" => {
            reject_args(rest, "help")?;
            Ok(Command::Help)
        }
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
///      command for <service>" error.
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
        Capability::ArrManager => super::commands::arr::parse(kind, verb, rest)?,
        Capability::Indexer => super::commands::indexer::parse(kind, verb, rest)?,
        Capability::DownloadClient => super::commands::download::parse(kind, verb, rest)?,
        Capability::MediaServer => super::commands::media_server::parse(kind, verb, rest)?,
        Capability::Requests => super::commands::requests::parse(kind, verb, rest)?,
        Capability::Stats => super::commands::stats::parse(kind, verb, rest)?,
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
            let flags = parse_passthrough_flags(rest, "post", true, true)?;
            Ok(Command::Post {
                service,
                path: flags.path,
                body: flags.body.unwrap_or(serde_json::Value::Null),
                confirm: flags.confirm,
            })
        }
        "put" => {
            let flags = parse_passthrough_flags(rest, "put", true, true)?;
            Ok(Command::Put {
                service,
                path: flags.path,
                body: flags.body.unwrap_or(serde_json::Value::Null),
                confirm: flags.confirm,
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
        // Unknown verb: not a generic passthrough verb and not claimed by the
        // capability's curated parser above.
        other => Err(anyhow!(
            "unknown command `{other}` for service `{}`",
            kind.as_str()
        )),
    }
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
