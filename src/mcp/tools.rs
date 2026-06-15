//! MCP tool dispatch — thin shims only.

use rmcp::{service::Peer, RoleServer};
use serde_json::{json, Value};

use crate::actions::{execute_service_action, RustarrAction};
use crate::app::RustarrService;
use crate::server::AppState;

pub(super) async fn execute_tool(
    state: &AppState,
    name: &str,
    args: Value,
    _peer: &Peer<RoleServer>,
) -> anyhow::Result<Value> {
    match name {
        "rustarr" => dispatch_rustarr(state, args).await,
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub async fn execute_tool_without_peer_for_test(
    state: &AppState,
    name: &str,
    args: Value,
) -> anyhow::Result<Value> {
    match name {
        "rustarr" => dispatch_rustarr(state, args).await,
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

async fn dispatch_rustarr(state: &AppState, args: Value) -> anyhow::Result<Value> {
    let action = RustarrAction::from_mcp_args(&args)?;
    dispatch_action(&state.service, &action).await
}

async fn dispatch_action(
    service: &RustarrService,
    action: &RustarrAction,
) -> anyhow::Result<Value> {
    match action {
        // `help` is the only special case: it returns generated registry help
        // rather than calling a service method. Generation lives in `mcp::help`,
        // keeping this shim free of business logic.
        RustarrAction::Help => Ok(json!({ "help": super::help::help_text() })),
        other => execute_service_action(service, other).await,
    }
}
