//! The parsed-CLI `Command` enum — pure data, no logic.
//!
//! This is the output of [`crate::cli::parse_args`] and the input to
//! [`crate::cli::run`]. Variants fall into two groups:
//!
//!   - **Service-grouped** (`Status`/`Get`/`Post`/`Put`/`Delete`) — produced by
//!     the per-capability parse hook in [`crate::cli::router`]. Today these are
//!     the generic passthrough verbs; curated `<service> <verb>` commands added
//!     by later capability beads land here too (as new variants or a future
//!     `Curated { .. }` carrier — see `src/cli/commands/<cap>.rs`).
//!   - **Infra, service-less** (`Integrations`/`Help`/`Doctor`/`Watch`/`Setup`)
//!     — produced directly by the router's infra branch.
//!
//! `Doctor`, `Watch`, and `Setup` are dispatched specially in
//! `main.rs::run_cli` because they need the full `Config` (MCP fields), not just
//! `RustarrConfig`. `serve`/`mcp` never reach this enum — `main.rs` intercepts
//! them as run modes before `parse_args` is called.

use super::setup::SetupCommand;

// `Eq` is intentionally not derived: the `Curated` variant carries a
// `serde_json::Value` (which is `PartialEq` but not `Eq`). `PartialEq` is all the
// tests need (`assert_eq!`).
#[derive(Debug, PartialEq)]
pub enum Command {
    /// `rustarr integrations` — list supported and configured services.
    Integrations,
    /// `rustarr <service> status` — upstream status for one service.
    Status { service: String },
    /// `rustarr <service> get --path P` — passthrough GET.
    Get { service: String, path: String },
    /// `rustarr <service> post --path P [--body JSON] --confirm` — passthrough POST.
    Post {
        service: String,
        path: String,
        body: serde_json::Value,
        confirm: bool,
    },
    /// `rustarr <service> put --path P [--body JSON] --confirm` — passthrough PUT.
    Put {
        service: String,
        path: String,
        body: serde_json::Value,
        confirm: bool,
    },
    /// `rustarr <service> delete --path P [--body JSON] --confirm` — passthrough DELETE.
    Delete {
        service: String,
        path: String,
        body: Option<serde_json::Value>,
        confirm: bool,
    },
    /// `rustarr help` — structured JSON action reference.
    Help,
    /// `rustarr doctor [--json]` — pre-flight environment validation (§48).
    ///
    /// Dispatched in `main.rs::run_cli` (needs full `Config`).
    Doctor {
        /// Output JSON instead of human-readable text.
        json: bool,
    },
    /// `rustarr watch [--url URL] [--interval N]` — poll `/health`, emit on state change.
    ///
    /// Dispatched in `main.rs::run_cli` (needs the MCP port for the default URL).
    Watch {
        /// Base URL or /health URL of the MCP server (default: http://localhost:{RUSTARR_MCP_PORT}).
        url: Option<String>,
        /// Poll interval in seconds (default: 10).
        interval: u64,
    },
    /// `rustarr setup ...` — plugin setup wizard. Dispatched in `main.rs::run_cli`.
    Setup(SetupCommand),
    /// `rustarr <service> <curated-verb> [flags]` — a curated, capability-scoped
    /// command resolved from the registry. `action` is the MCP (snake_case) name;
    /// `params` is the JSON args object the router assembled from the positional
    /// service + flags. Dispatched through the SAME `execute_service_action` path
    /// as MCP, so CLI↔MCP parity is automatic.
    Curated {
        action: &'static str,
        params: serde_json::Value,
    },
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
