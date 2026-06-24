//! MCP tool dispatch — thin shims only.

use rmcp::{RoleServer, service::Peer};
use serde_json::{Map, Value};

use crate::actions::{RustarrAction, execute_service_action};
use crate::server::AppState;

use super::schemas::{YARR_TOOL_NAME, service_tool_kind};

pub(super) async fn execute_tool(
    state: &AppState,
    name: &str,
    args: Value,
    _peer: &Peer<RoleServer>,
) -> anyhow::Result<Value> {
    dispatch_tool(state, name, args).await
}

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub async fn execute_tool_without_peer_for_test(
    state: &AppState,
    name: &str,
    args: Value,
) -> anyhow::Result<Value> {
    dispatch_tool(state, name, args).await
}

/// Route a tool call. The live MCP surface is the single `yarr` tool (→ the
/// `codemode` action); the per-service path remains for the dispatch-layer test
/// helper and is what a `yarr` script's own `callTool` mirrors internally.
async fn dispatch_tool(state: &AppState, name: &str, args: Value) -> anyhow::Result<Value> {
    if name == YARR_TOOL_NAME {
        return dispatch_yarr(state, args).await;
    }
    match service_tool_kind(name) {
        Some(_) => dispatch_service_tool(state, name, args).await,
        None => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

/// The `yarr` tool's only param is `code`; it dispatches the `codemode` action.
async fn dispatch_yarr(state: &AppState, args: Value) -> anyhow::Result<Value> {
    let mut object = match args {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    object.insert("action".to_owned(), Value::String("codemode".to_owned()));
    let action = RustarrAction::from_mcp_args(&Value::Object(object))?;
    execute_service_action(&state.service, &action).await
}

async fn dispatch_service_tool(
    state: &AppState,
    service: &str,
    args: Value,
) -> anyhow::Result<Value> {
    // Thin shim: parse args and route EVERY action (including `help`) through the
    // shared service-layer dispatch. No special cases or business logic here.
    let args = inject_service(args, service);
    let action = RustarrAction::from_mcp_args(&args)?;
    execute_service_action(&state.service, &action).await
}

fn inject_service(args: Value, service: &str) -> Value {
    let mut object = match args {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    object.insert("service".to_owned(), Value::String(service.to_owned()));
    Value::Object(object)
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
