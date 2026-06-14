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
        RustarrAction::Help => Ok(json!({ "help": HELP_TEXT })),
        other => execute_service_action(service, other).await,
    }
}

const HELP_TEXT: &str = r#"# rustarr MCP Tool

Single tool: `rustarr`

Actions:
- `integrations`: list supported and configured integrations.
- `service_status`: call the default status endpoint for a configured service. Requires `service`.
- `api_get`: GET a safe relative path. Requires `service` and `path`.
- `api_post`: POST JSON to a safe relative path. Requires `service`, `path`, and `confirm=true`; optional `body` defaults to `{}`.
- `api_put`: PUT JSON to a safe relative path. Requires `service`, `path`, and `confirm=true`; optional `body` defaults to `{}`. Use for resource updates like Sonarr/Radarr `editor` bulk edits.
- `api_delete`: DELETE a safe relative path. Requires `service`, `path`, and `confirm=true`; optional `body`. Query params go in `path` (e.g. `?deleteFiles=false`).
- `help`: return this text.

Credentials are configured outside tool-call arguments through `RUSTARR_SERVICES`
and per-service environment variables or config.toml. Do not pass API keys in
paths or request bodies unless the upstream endpoint itself requires it.
"#;
