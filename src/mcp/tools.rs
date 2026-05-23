//! MCP tool dispatch — thin shims only.

use rmcp::{
    service::{ElicitationError, Peer},
    RoleServer,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::actions::{execute_service_action, RustarrAction};
use crate::app::{ElicitedNameOutcome, RustarrService};
use crate::scaffold::ScaffoldIntent;
use crate::server::AppState;

pub(super) async fn execute_tool(
    state: &AppState,
    name: &str,
    args: Value,
    peer: &Peer<RoleServer>,
) -> anyhow::Result<Value> {
    match name {
        "rustarr" => dispatch_rustarr(state, args, peer).await,
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
        "rustarr" => dispatch_rustarr_without_peer(state, args).await,
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    }
}

async fn dispatch_rustarr(
    state: &AppState,
    args: Value,
    peer: &Peer<RoleServer>,
) -> anyhow::Result<Value> {
    let action = RustarrAction::from_mcp_args(&args)?;
    match action {
        RustarrAction::ElicitName => elicit_name(&state.service, peer).await,
        RustarrAction::ScaffoldIntent => scaffold_intent(&state.service, peer).await,
        other => dispatch_non_elicitation_action(&state.service, &other).await,
    }
}

#[cfg(any(test, feature = "test-support"))]
async fn dispatch_rustarr_without_peer(state: &AppState, args: Value) -> anyhow::Result<Value> {
    let action = RustarrAction::from_mcp_args(&args)?;
    match action {
        RustarrAction::ElicitName | RustarrAction::ScaffoldIntent => Err(anyhow::anyhow!(
            "action={} requires an MCP peer",
            action.name()
        )),
        other => dispatch_non_elicitation_action(&state.service, &other).await,
    }
}

async fn dispatch_non_elicitation_action(
    service: &RustarrService,
    action: &RustarrAction,
) -> anyhow::Result<Value> {
    match action {
        RustarrAction::Help => Ok(json!({ "help": HELP_TEXT })),
        other => execute_service_action(service, other).await,
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct NameInput {
    /// Your first name or preferred display name.
    name: String,
}

rmcp::elicit_safe!(NameInput);

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ScaffoldIntentInput {
    /// Human-readable project name, e.g. "Unraid MCP" or "Lab Gateway".
    display_name: String,
    /// Cargo package name, e.g. "unraid-mcp".
    crate_name: String,
    /// Binary/tool name, e.g. "unraid".
    binary_name: String,
    /// Server category: "upstream-client" or "application-platform".
    server_category: String,
    /// Environment variable prefix, e.g. "UNRAID" or "LAB".
    env_prefix: String,
    /// Upstream auth kind: "none", "api-key", "bearer", "oauth", "both", or "other".
    auth_kind: String,
    /// Default bind host, e.g. "127.0.0.1" or "0.0.0.0".
    host: String,
    /// Default HTTP port, e.g. 3100.
    port: u16,
    /// MCP transport mode: "stdio", "http", or "dual".
    mcp_transport: String,
    /// MCP primitives to scaffold, comma-separated.
    mcp_primitives: String,
    /// Deployment scaffolding: "none", "systemd", or "docker".
    deployment: String,
    /// Plugin surfaces to scaffold, comma-separated: "claude, codex, gemini".
    plugins: String,
    /// Whether to scaffold MCP registry publishing through server.json.
    publish_mcp: bool,
    /// Documentation URLs to crawl, comma-separated.
    crawl_urls: String,
    /// Repository URLs to crawl, comma-separated.
    crawl_repos: String,
    /// Search topics to crawl, comma-separated.
    crawl_search_topics: String,
}

rmcp::elicit_safe!(ScaffoldIntentInput);

async fn elicit_name(service: &RustarrService, peer: &Peer<RoleServer>) -> anyhow::Result<Value> {
    match peer
        .elicit::<NameInput>("What is your name? I'll use it to return a personalized greeting.")
        .await
    {
        Ok(Some(input)) => {
            Ok(service.elicited_name_greeting(ElicitedNameOutcome::Accepted(&input.name)))
        }
        Ok(None) => Ok(service.elicited_name_greeting(ElicitedNameOutcome::NoInput)),
        Err(ElicitationError::UserDeclined) => {
            Ok(service.elicited_name_greeting(ElicitedNameOutcome::Declined))
        }
        Err(ElicitationError::UserCancelled) => {
            Ok(service.elicited_name_greeting(ElicitedNameOutcome::Cancelled))
        }
        Err(ElicitationError::CapabilityNotSupported) => {
            tracing::warn!("elicitation requested but client does not support it");
            Ok(service.elicited_name_greeting(ElicitedNameOutcome::Unsupported))
        }
        Err(e) => {
            tracing::error!(error = %e, "elicitation failed unexpectedly");
            Err(anyhow::anyhow!("elicitation failed unexpectedly: {e}"))
        }
    }
}

async fn scaffold_intent(
    service: &RustarrService,
    peer: &Peer<RoleServer>,
) -> anyhow::Result<Value> {
    match peer
        .elicit::<ScaffoldIntentInput>(
            "Tell me what kind of project you are scaffolding. I will return JSON only; the scaffold-project skill will turn it into an approval-first plan.",
        )
        .await
    {
        Ok(Some(input)) => service.scaffold_intent(input.into()),
        Ok(None) => Ok(json!({
            "kind": "rustarr_scaffold_intent",
            "schema_version": 1,
            "status": "no_input",
            "message": "No scaffold intent was provided.",
        })),
        Err(ElicitationError::UserDeclined) => Ok(json!({
            "kind": "rustarr_scaffold_intent",
            "schema_version": 1,
            "status": "declined",
            "message": "User declined to provide scaffold intent.",
        })),
        Err(ElicitationError::UserCancelled) => Ok(json!({
            "kind": "rustarr_scaffold_intent",
            "schema_version": 1,
            "status": "cancelled",
            "message": "Scaffold intent elicitation was cancelled.",
        })),
        Err(ElicitationError::CapabilityNotSupported) => Ok(json!({
            "kind": "rustarr_scaffold_intent",
            "schema_version": 1,
            "status": "elicitation_not_supported",
            "message": "This MCP client does not support elicitation.",
            "fallback": {
                "recommended_skill": "scaffold-project",
                "instructions": "Ask the user for the scaffold fields manually, then create the same JSON shape documented by the scaffold-project skill. Do not mutate files until the user approves the plan."
            }
        })),
        Err(e) => {
            tracing::error!(error = %e, "scaffold intent elicitation failed unexpectedly");
            Err(anyhow::anyhow!("scaffold intent elicitation failed unexpectedly: {e}"))
        }
    }
}

impl From<ScaffoldIntentInput> for ScaffoldIntent {
    fn from(input: ScaffoldIntentInput) -> Self {
        Self {
            display_name: input.display_name,
            crate_name: input.crate_name,
            binary_name: input.binary_name,
            server_category: input.server_category,
            env_prefix: input.env_prefix,
            auth_kind: input.auth_kind,
            host: input.host,
            port: input.port,
            mcp_transport: input.mcp_transport,
            mcp_primitives: input.mcp_primitives,
            deployment: input.deployment,
            plugins: input.plugins,
            publish_mcp: input.publish_mcp,
            crawl_urls: input.crawl_urls,
            crawl_repos: input.crawl_repos,
            crawl_search_topics: input.crawl_search_topics,
        }
    }
}

const HELP_TEXT: &str = r#"# rustarr MCP Tool

Single tool: `rustarr`

Actions:
- `integrations`: list supported and configured integrations.
- `service_status`: call the default status endpoint for a configured service. Requires `service`.
- `api_get`: GET a safe relative path. Requires `service` and `path`.
- `api_post`: POST JSON to a safe relative path. Requires `service`, `path`, and `confirm=true`; optional `body` defaults to `{}`.
- `elicit_name`: ask the MCP client for a name and return a greeting.
- `scaffold_intent`: collect scaffold requirements through MCP elicitation and return handoff JSON.
- `help`: return this text.

Credentials are configured outside tool-call arguments through `RUSTARR_SERVICES`
and per-service environment variables or config.toml. Do not pass API keys in
paths or request bodies unless the upstream endpoint itself requires it.
"#;
