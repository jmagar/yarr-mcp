//! CLI facade — thin shim that parses args, calls `RustarrService`, formats output.
//!
//! The CLI uses the same service layer as the MCP server. **No business logic
//! lives here** (the thin-shim rule): every arm parses input, makes exactly one
//! `RustarrService` call, and prints. Logic belongs in `app.rs`.
//!
//! # Grammar
//!
//! ```text
//! rustarr <service> status                       Upstream status for one service
//! rustarr <service> get --path PATH              Passthrough GET
//! rustarr <service> post --path PATH [--body JSON]
//! rustarr <service> put  --path PATH [--body JSON]
//! rustarr <service> delete --path PATH [--body JSON] --confirm
//!
//! rustarr integrations             List supported and configured services
//! rustarr help                     JSON action reference
//! rustarr doctor [--json]          Pre-flight checks
//! rustarr watch [--url URL] [--interval N]
//! rustarr setup check|repair|install|plugin-hook
//! rustarr [serve] | rustarr mcp    Run modes (intercepted in main.rs)
//! ```
//!
//! # Module map
//!
//!   - [`command`] — the parsed [`Command`] enum (pure data).
//!   - [`router`]  — `token1` → infra verb OR service→kind→capability dispatch.
//!   - [`parse`]   — shared flag parsers (path/body/confirm + selectors).
//!   - [`usage`](mod@self::usage)   — USAGE generated from the action registry + capability map.
//!   - [`doctor`] / [`setup`] / [`watch`] — infra-command implementations.
//!
//! Capability beads add curated commands as parse-only modules under
//! `src/cli/commands/<capability>.rs` and extend
//! [`router::parse_capability_command`].

use crate::{
    actions::rest_help, app::RustarrService, config::RustarrConfig, rustarr::RustarrClient,
};
use anyhow::Result;

pub mod command;
pub mod commands;
pub mod doctor;
pub mod parse;
pub mod router;
pub mod setup;
pub mod usage;
pub mod watch;

pub use command::Command;
pub use commands::capability_verb_tables;
pub use doctor::run_doctor;
pub use setup::{SetupCommand, apply_plugin_options, run_setup};
pub use usage::usage;
pub use watch::run_watch;

/// Parse CLI arguments from `std::env::args()`.
///
/// Returns `Ok(None)` when no subcommand was given so `main.rs` can print the
/// "Unknown command" hint.
pub fn parse_args() -> Result<Option<Command>> {
    parse_args_from(std::env::args().skip(1))
}

/// Parse CLI arguments from an arbitrary iterator (used by tests).
pub fn parse_args_from<I, S>(args: I) -> Result<Option<Command>>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    router::route(&args)
}

/// Run a CLI command, print the result, and exit.
///
/// `Doctor`, `Watch`, and `Setup` are handled directly in `main.rs::run_cli`
/// (they need the full `Config`); they are unreachable here.
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
        } => service.api_post(name, path, body.clone()).await?,
        Command::Put {
            service: name,
            path,
            body,
        } => service.api_put(name, path, body.clone()).await?,
        Command::Delete {
            service: name,
            path,
            body,
            confirm,
        } => {
            service
                .api_delete(name, path, body.clone(), *confirm)
                .await?
        }
        Command::Help => rest_help(),
        // Curated commands run through the SAME shared dispatch path as MCP
        // (`execute_service_action`), which applies the action×kind guard and
        // routes to the descriptor handler — so CLI↔MCP parity is automatic.
        Command::Curated { action, params } => {
            let parsed = crate::actions::RustarrAction::Curated {
                name: action,
                params: params.clone(),
            };
            crate::actions::execute_service_action(&service, &parsed).await?
        }
        Command::Doctor { .. } | Command::Watch { .. } | Command::Setup(_) => {
            unreachable!("dispatched directly in main.rs::run_cli")
        }
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
