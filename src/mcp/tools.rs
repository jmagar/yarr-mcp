//! MCP tool dispatch — thin shims only.

use rmcp::{RoleServer, service::Peer};
use serde_json::Value;

use crate::actions::{RustarrAction, execute_service_action};
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
    // Thin shim: parse args and route EVERY action (including `help`) through the
    // shared service-layer dispatch. No special cases or business logic here.
    let action = RustarrAction::from_mcp_args(&args)?;
    execute_service_action(&state.service, &action).await
}
