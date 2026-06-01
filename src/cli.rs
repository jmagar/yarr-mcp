//! CLI — thin shim that parses args, calls `RustarrService`, formats output.
//!
//! The CLI uses the same service layer as the MCP server. No business logic lives here.
//!
//! **Template**: add subcommands to match your service's operations.
//!
//! # Usage
//!
//! ```text
//! rustarr integrations
//! rustarr status --service sonarr
//! rustarr get --service sonarr --path /api/v3/system/status
//! rustarr doctor [--json]
//! ```

use crate::{
    actions::rest_help, app::RustarrService, config::RustarrConfig, rustarr::RustarrClient,
};
use anyhow::{anyhow, Result};

// TEMPLATE: The doctor module is the §48 reference implementation.
//           Import it from here and wire into run() below.
pub mod doctor;
pub mod setup;
pub mod watch;

pub use setup::{apply_plugin_options, run_setup, SetupCommand};

pub const USAGE: &str = "Usage:
  rustarr [serve]          Start MCP HTTP server (default)
  rustarr mcp              Start MCP stdio transport

  rustarr integrations              List supported and configured services
  rustarr status --service NAME     Show upstream service status
  rustarr get --service NAME --path PATH
  rustarr post --service NAME --path PATH --body JSON --confirm
  rustarr help                      Show JSON action reference
  rustarr doctor [--json]           Run environment pre-flight checks
  rustarr watch [--url URL] [--interval N]  Poll server health and emit on state change
  rustarr setup check               Check plugin setup without mutating appdata
  rustarr setup repair              Create missing appdata/env setup files
  rustarr setup plugin-hook [--no-repair]  Plugin hook JSON contract

  rustarr --help                    Show this help
  rustarr --version                 Show version

Environment:
  RUSTARR_SERVICES         Comma-separated configured service names
  RUSTARR_<NAME>_URL       Upstream service URL
  RUSTARR_<NAME>_API_KEY   Upstream service API key
  RUSTARR_MCP_HOST         Bind host (default 127.0.0.1)
  RUSTARR_MCP_PORT         Bind port (default 40070)
  RUSTARR_MCP_NO_AUTH      Disable auth (loopback only)
  RUSTARR_MCP_TOKEN        Static bearer token
  RUST_LOG                 Log filter (e.g. info,rmcp=warn)";

pub fn usage() -> &'static str {
    USAGE
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Integrations,
    Status {
        service: String,
    },
    Get {
        service: String,
        path: String,
    },
    Post {
        service: String,
        path: String,
        body: serde_json::Value,
        confirm: bool,
    },
    Help,
    /// Pre-flight environment validation (§48).
    ///
    /// TEMPLATE: Always keep this command. It is the operator's first stop
    /// when setting up or debugging the service.
    Doctor {
        /// Output JSON instead of human-readable text.
        json: bool,
    },
    /// Poll the MCP server health endpoint and emit a line on every state change.
    ///
    /// Designed to be run as a plugin monitor — stdout is the event stream,
    /// stderr is debug output. Exits only on CTRL+C.
    Watch {
        /// Base URL or /health URL of the MCP server (default: http://localhost:{RUSTARR_MCP_PORT}).
        url: Option<String>,
        /// Poll interval in seconds (default: 10).
        interval: u64,
    },
    Setup(SetupCommand),
}

/// Parse CLI arguments from `std::env::args()`.
///
/// Returns `None` if the first argument is not a known subcommand.
/// **Template**: extend this to use clap or another arg parser for a real CLI.
/// This is intentionally minimal so the template compiles without extra deps.
///
/// # TEMPLATE: Adding a new subcommand
///
/// 1. Add a variant to `Command` above.
/// 2. Add a match arm here to construct it from args.
/// 3. Add a dispatch arm in `run()` below.
/// 4. Update `USAGE` above.
pub fn parse_args() -> Result<Option<Command>> {
    parse_args_from(std::env::args().skip(1))
}

pub fn parse_args_from<I, S>(args: I) -> Result<Option<Command>>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let command = match args.as_slice() {
        [] => None,
        [subcommand, rest @ ..] => match subcommand.as_str() {
            "integrations" => {
                reject_args(rest, "integrations")?;
                Some(Command::Integrations)
            }
            "status" => {
                let service = parse_required_value_flag(rest, "status", "--service")?
                    .ok_or_else(|| anyhow!("status requires --service"))?;
                Some(Command::Status { service })
            }
            "get" => {
                let (service, path, body, _) = parse_service_path_body_flags(rest, "get", false)?;
                if body.is_some() {
                    return Err(anyhow!("get does not accept --body"));
                }
                Some(Command::Get { service, path })
            }
            "post" => {
                let (service, path, body, confirm) =
                    parse_service_path_body_flags(rest, "post", true)?;
                Some(Command::Post {
                    service,
                    path,
                    body: body.unwrap_or(serde_json::Value::Null),
                    confirm,
                })
            }
            "help" => {
                reject_args(rest, "help")?;
                Some(Command::Help)
            }
            // §48: doctor is always parsed here, dispatched via run_cli in main.rs.
            // TEMPLATE: Keep this arm. It routes to doctor::run_doctor() which needs
            //           the full Config (not just RustarrConfig), so main.rs handles it.
            "doctor" => {
                let json = parse_bool_flag(rest, "doctor", "--json")?;
                Some(Command::Doctor { json })
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
                Some(Command::Watch { url, interval })
            }
            "setup" => match rest {
                [action, flags @ ..] if action == "check" => {
                    reject_args(flags, "setup check")?;
                    Some(Command::Setup(SetupCommand::Check))
                }
                [action, flags @ ..] if action == "repair" => {
                    reject_args(flags, "setup repair")?;
                    Some(Command::Setup(SetupCommand::Repair))
                }
                    [action, flags @ ..] if action == "install" => {
                        reject_args(flags, "setup install")?;
                        Some(Command::Setup(SetupCommand::Install))
                    }
                [action, flags @ ..] if action == "plugin-hook" => {
                    let no_repair = parse_bool_flag(flags, "setup plugin-hook", "--no-repair")?;
                    Some(Command::Setup(SetupCommand::PluginHook { no_repair }))
                }
                _ => None,
            },
            _ => None,
        },
    };
    Ok(command)
}

/// Run a CLI command, print the result, and exit.
///
/// # TEMPLATE
/// - `Doctor` is handled specially in `main.rs::run_cli` (needs full `Config`).
/// - All other commands get only `RustarrConfig`; keep it that way.
/// - Add `--json` support to each new command by forwarding a `json` flag.
pub async fn run(cmd: Command, cfg: &RustarrConfig) -> Result<()> {
    let client = RustarrClient::new(cfg)?;
    let service = RustarrService::new(client, cfg.clone());

    let result = match &cmd {
        Command::Integrations => service.integrations(),
        Command::Status { service: name } => service.service_status(name).await?,
        Command::Get {
            service: name,
            path,
        } => service.api_get(name, path).await?,
        Command::Post {
            service: name,
            path,
            body,
            confirm,
        } => service.api_post(name, path, body.clone(), *confirm).await?,
        Command::Help => rest_help(),
        // Doctor, Watch, and Setup are never dispatched via this function — main.rs
        // handles them directly because they need config.mcp fields.
        Command::Doctor { .. } | Command::Watch { .. } | Command::Setup(_) => {
            unreachable!("dispatched directly in main.rs::run_cli")
        }
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

// ── arg parsing helpers ───────────────────────────────────────────────────────

fn reject_args(args: &[String], command: &str) -> Result<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("{command} does not accept argument `{}`", args[0]))
    }
}

fn parse_bool_flag(args: &[String], command: &str, flag: &str) -> Result<bool> {
    let mut found = false;
    for arg in args {
        if arg == flag {
            if found {
                return Err(anyhow!("{command} received duplicate {flag}"));
            }
            found = true;
        } else {
            return Err(anyhow!("{command} does not accept argument `{arg}`"));
        }
    }
    Ok(found)
}

fn parse_optional_value_flag(args: &[String], command: &str, flag: &str) -> Result<Option<String>> {
    match args {
        [] => Ok(None),
        [found_flag, value] if found_flag == flag => {
            if value.starts_with("--") {
                Err(anyhow!("{command} requires a value after {flag}"))
            } else {
                Ok(Some(value.clone()))
            }
        }
        [found_flag] if found_flag == flag => {
            Err(anyhow!("{command} requires a value after {flag}"))
        }
        [found_flag, value, rest @ ..] if found_flag == flag => {
            if value.starts_with("--") {
                Err(anyhow!("{command} requires a value after {flag}"))
            } else if rest.iter().any(|arg| arg == flag) {
                Err(anyhow!("{command} received duplicate {flag}"))
            } else {
                Err(anyhow!("{command} does not accept argument `{}`", rest[0]))
            }
        }
        [unexpected, ..] => Err(anyhow!("{command} does not accept argument `{unexpected}`")),
    }
}

fn parse_required_value_flag(args: &[String], command: &str, flag: &str) -> Result<Option<String>> {
    match parse_optional_value_flag(args, command, flag)? {
        Some(value) => Ok(Some(value)),
        None => Ok(None),
    }
}

fn parse_service_path_body_flags(
    args: &[String],
    command: &str,
    require_body: bool,
) -> Result<(String, String, Option<serde_json::Value>, bool)> {
    let mut service = None;
    let mut path = None;
    let mut body = None;
    let mut confirm = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--service" => {
                i += 1;
                service = Some(
                    args.get(i)
                        .cloned()
                        .ok_or_else(|| anyhow!("{command} requires a value after --service"))?,
                );
            }
            "--path" => {
                i += 1;
                path = Some(
                    args.get(i)
                        .cloned()
                        .ok_or_else(|| anyhow!("{command} requires a value after --path"))?,
                );
            }
            "--body" => {
                i += 1;
                let raw = args
                    .get(i)
                    .ok_or_else(|| anyhow!("{command} requires a value after --body"))?;
                body = Some(serde_json::from_str(raw)?);
            }
            "--confirm" if require_body => {
                confirm = true;
            }
            other => return Err(anyhow!("{command} does not accept argument `{other}`")),
        }
        i += 1;
    }
    let service = service.ok_or_else(|| anyhow!("{command} requires --service"))?;
    let path = path.ok_or_else(|| anyhow!("{command} requires --path"))?;
    if require_body && body.is_none() {
        return Err(anyhow!("{command} requires --body"));
    }
    Ok((service, path, body, confirm))
}

fn parse_watch_flags(args: &[String]) -> Result<(Option<String>, Option<String>)> {
    let mut url = None;
    let mut interval = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let target = match flag {
            "--url" => &mut url,
            "--interval" => &mut interval,
            _ => return Err(anyhow!("watch does not accept argument `{flag}`")),
        };
        if target.is_some() {
            return Err(anyhow!("watch received duplicate {flag}"));
        }
        let Some(value) = args.get(index + 1) else {
            return Err(anyhow!("watch requires a value after {flag}"));
        };
        if value.starts_with("--") {
            return Err(anyhow!("watch requires a value after {flag}"));
        }
        *target = Some(value.clone());
        index += 2;
    }
    Ok((url, interval))
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
