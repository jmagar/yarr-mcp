//! `RustarrRmcpServer` — the `ServerHandler` implementation.
//!
//! This is the adapter between the rmcp crate and your application. It:
//!   - Advertises tools, resources, and prompts to MCP clients
//!   - Enforces auth scopes on every call
//!   - Delegates business logic to `tools.rs` → `app.rs` → `rustarr.rs`
//!
//! **Template**: rename `RustarrRmcpServer`. Update action metadata in
//! `src/actions.rs` to keep schemas, scope rules, and dispatch in sync.

use std::{borrow::Cow, sync::Arc, time::Instant};

use lab_auth::AuthContext;
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Content, GetPromptRequestParams, GetPromptResult,
        Implementation, ListPromptsResult, ListResourcesResult, ListToolsResult,
        PaginatedRequestParams, RawResource, ReadResourceRequestParams, ReadResourceResult,
        Resource, ResourceContents, ServerCapabilities, ServerInfo, Tool,
    },
    service::{Peer, RequestContext},
};
use serde_json::{Map, Value};

use crate::{
    actions::{ValidationError, is_known_action, required_scope_for_action},
    token_limit,
};

use crate::server::{AppState, AuthPolicy};

use super::{
    elicit, prompts,
    schemas::{tool_definitions, tool_definitions_for_kinds},
    tools::execute_tool,
};

// ── server ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RustarrRmcpServer {
    state: AppState,
}

pub fn rmcp_server(state: AppState) -> RustarrRmcpServer {
    warn_if_unscoped_with_mutations(&state);
    RustarrRmcpServer { state }
}

/// S5: `TrustedGatewayUnscoped` disables auth middleware *and* bypasses scope
/// checks entirely (see `require_auth_context`). When mutating actions are
/// registered, emit a one-time startup warning so operators know writes are not
/// scope-gated in this mode. Note that plain writes now run with no per-call
/// gate at all; only destructive deletes retain a gate (elicitation / confirm),
/// so the gateway is the sole authz boundary for writes.
fn warn_if_unscoped_with_mutations(state: &AppState) {
    if !matches!(state.auth_policy, AuthPolicy::TrustedGatewayUnscoped) {
        return;
    }
    let mutating: Vec<&str> = crate::actions::all_action_names()
        .into_iter()
        .filter(|name| {
            crate::actions::required_scope_for_action(name) == Some(crate::actions::WRITE_SCOPE)
        })
        .collect();
    if mutating.is_empty() {
        return;
    }
    tracing::warn!(
        mutating_actions = %mutating.join(", "),
        "AuthPolicy::TrustedGatewayUnscoped bypasses scope checks; mutating actions are NOT \
         scope-gated, and only destructive deletes retain a confirm/elicitation gate. Ensure \
         the upstream gateway enforces authz."
    );
}

impl ServerHandler for RustarrRmcpServer {
    // ── tools ─────────────────────────────────────────────────────────────────

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        let tools = rmcp_tool_definitions_for_service(&self.state.service)?;
        tracing::debug!(tool_count = tools.len(), "MCP tools listed");
        Ok(ListToolsResult {
            tools,
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool_name = request.name.to_string();

        // Extract action before scope check so a missing action returns the
        // more useful "action is required" validation error, not DENY_SCOPE.
        let action_opt: Option<String> = request
            .arguments
            .as_ref()
            .and_then(|m| m.get("action"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        let auth = require_auth_context(&self.state, &context)?;
        if let Some(action_str) = action_opt.as_deref() {
            reject_unknown_action_before_scope(action_str)?;
        }
        // Only scope-check when a known action is present; dispatch_rustarr will
        // return the validation error for a missing action below.
        if let (Some(auth), Some(action_str)) = (auth, action_opt.as_deref())
            && let Some(required_scope) = required_scope_for_action(action_str)
        {
            check_scope(auth, required_scope, action_str)?;
        }

        let action: String = action_opt.unwrap_or_default();

        let mut arguments = request
            .arguments
            .map(Value::Object)
            .unwrap_or_else(|| Value::Object(Map::new()));

        // Clone the peer for client interaction (elicitation) and the dispatcher
        // signature.
        let peer: Peer<RoleServer> = context.peer.clone();

        // Destructive-delete gate (MCP-only). Destructive deletes are the only
        // actions still gated after the write-confirm removal; obtain confirmation
        // via elicitation before dispatch. The app layer remains the final
        // enforcement point, so an `Abstain` outcome still reaches a confirm gate
        // downstream. The tool name IS the service name (the MCP tool is
        // service-named; `action` is a parameter).
        if crate::actions::action_is_destructive(&action) {
            match elicit::gate_destructive(&peer, &action, &tool_name, &arguments).await {
                elicit::DeleteGate::Proceed => inject_confirm(&mut arguments),
                elicit::DeleteGate::Declined => {
                    tracing::info!(
                        tool = %tool_name,
                        action = %action,
                        "destructive action declined via elicitation; nothing changed"
                    );
                    return declined_result(&action);
                }
                // No elicitation channel: abstain and leave args as-is so the app
                // layer returns its needs-confirm response (the caller can
                // re-issue with confirm=true).
                elicit::DeleteGate::Abstain => {}
            }
        }

        let started = Instant::now();
        tracing::info!(tool = %tool_name, action = %action, "MCP tool execution started");

        match execute_tool(&self.state, &tool_name, arguments, &peer).await {
            Ok(result) => {
                tracing::info!(
                    tool = %tool_name,
                    elapsed_ms = started.elapsed().as_millis(),
                    "MCP tool execution completed"
                );
                tool_result_from_json(result)
            }
            Err(error) if crate::actions::is_validation_error(&error) => {
                tracing::warn!(
                    tool = %tool_name,
                    elapsed_ms = started.elapsed().as_millis(),
                    "MCP tool rejected invalid params"
                );
                Err(ErrorData::invalid_params(error.to_string(), None))
            }
            Err(error) => {
                tracing::error!(
                    tool = %tool_name,
                    elapsed_ms = started.elapsed().as_millis(),
                    error = %error,
                    "MCP tool execution failed"
                );
                Err(ErrorData::internal_error(
                    internal_tool_error_message(&action),
                    None,
                ))
            }
        }
    }

    // ── resources ─────────────────────────────────────────────────────────────

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        Ok(ListResourcesResult {
            resources: vec![schema_resource()],
            ..Default::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        if request.uri != SCHEMA_RESOURCE_URI {
            return Err(ErrorData::invalid_params(
                format!("unknown resource: {}", request.uri),
                None,
            ));
        }
        let schema = tool_definitions();
        let text = serde_json::to_string_pretty(&schema)
            .map_err(|e| ErrorData::internal_error(format!("serialization error: {e}"), None))?;
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(text, SCHEMA_RESOURCE_URI).with_mime_type("application/json"),
        ]))
    }

    // ── prompts ───────────────────────────────────────────────────────────────

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        Ok(prompts::list_prompts())
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        require_auth_context(&self.state, &context)?;
        prompts::get_prompt(request).map_err(|e| ErrorData::invalid_params(e.to_string(), None))
    }

    // ── server info ───────────────────────────────────────────────────────────

    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new(
            self.state.config.server_name.clone(),
            env!("CARGO_PKG_VERSION"),
        ))
    }
}

// ── resource definitions ──────────────────────────────────────────────────────

/// URI for the schema resource. **Template**: change `rustarr` to your service name.
const SCHEMA_RESOURCE_URI: &str = "rustarr://schema/mcp-tool";

fn schema_resource() -> Resource {
    Resource::new(
        RawResource::new(SCHEMA_RESOURCE_URI, "rustarr service tool schema")
            .with_description(
                "JSON schema for the rustarr service-named MCP tools and their action-based parameters",
            )
            .with_mime_type("application/json"),
        None,
    )
}

// ── tool definition conversion ────────────────────────────────────────────────

fn rmcp_tool_definitions_for_service(
    service: &crate::app::RustarrService,
) -> Result<Vec<Tool>, ErrorData> {
    tool_definitions_for_kinds(&service.configured_kinds())
        .into_iter()
        .map(rmcp_tool_from_json)
        .collect()
}

fn rmcp_tool_from_json(value: Value) -> Result<Tool, ErrorData> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| ErrorData::internal_error("tool definition missing name", None))?;
    let description = value
        .get("description")
        .and_then(Value::as_str)
        .map(|d| Cow::Owned(d.to_string()));
    let input_schema = value
        .get("inputSchema")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| ErrorData::internal_error("tool definition missing inputSchema", None))?;
    Ok(Tool::new_with_raw(
        Cow::Owned(name.to_string()),
        description,
        Arc::new(input_schema),
    ))
}

fn tool_result_from_json(value: Value) -> Result<CallToolResult, ErrorData> {
    // Compact JSON (not pretty) recovers ~30-40% of the 40 KB token budget.
    // When the payload is over the cap, `serialize_with_limit` returns a
    // parseable `{"truncated":true,...}` envelope (AN-6) instead of a notice
    // appended after the closing brace, so the client can always JSON.parse it.
    let (text, truncated) = token_limit::serialize_with_limit(&value);
    if truncated {
        tracing::warn!(
            bytes = text.len(),
            "MCP tool response truncated to fit token budget"
        );
    }
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

/// Inject `confirm=true` into the tool arguments once a destructive action has
/// been confirmed via elicitation, so the shared dispatch + app-layer gate see
/// the same confirmation the CLI's `--confirm` produces.
fn inject_confirm(args: &mut Value) {
    if let Value::Object(map) = args {
        map.insert("confirm".to_owned(), Value::Bool(true));
    }
}

/// Result returned when a destructive action is declined at the elicitation
/// prompt: a structured success payload (nothing was changed), not an error.
fn declined_result(action: &str) -> Result<CallToolResult, ErrorData> {
    tool_result_from_json(serde_json::json!({
        "declined": true,
        "action": action,
        "note": "destructive action not confirmed; nothing was changed",
    }))
}

fn reject_unknown_action_before_scope(action: &str) -> Result<(), ErrorData> {
    if is_known_action(action) {
        return Ok(());
    }
    Err(ErrorData::invalid_params(
        ValidationError::UnknownAction {
            action: action.to_owned(),
        }
        .to_string(),
        None,
    ))
}

fn internal_tool_error_message(action: &str) -> String {
    format!("tool execution failed: kind=execution_error action='{action}'")
}

// ── auth helpers ──────────────────────────────────────────────────────────────

fn require_auth_context<'a>(
    state: &AppState,
    ctx: &'a RequestContext<RoleServer>,
) -> Result<Option<&'a AuthContext>, ErrorData> {
    match &state.auth_policy {
        AuthPolicy::LoopbackDev | AuthPolicy::TrustedGatewayUnscoped => Ok(None),
        AuthPolicy::Mounted { .. } => {
            let parts = ctx
                .extensions
                .get::<axum::http::request::Parts>()
                .ok_or_else(|| {
                    tracing::error!(
                        "rmcp HTTP Parts extension absent — middleware ordering may be broken"
                    );
                    ErrorData::invalid_request("forbidden: missing http context", None)
                })?;
            let auth = parts.extensions.get::<AuthContext>().ok_or_else(|| {
                tracing::warn!("AuthContext absent — AuthLayer may not be mounted");
                ErrorData::invalid_request("forbidden: missing auth context", None)
            })?;
            Ok(Some(auth))
        }
    }
}

fn check_scope(auth: &AuthContext, required_scope: &str, action: &str) -> Result<(), ErrorData> {
    if scope_satisfied(&auth.scopes, required_scope) {
        return Ok(());
    }
    tracing::warn!(
        subject = %auth.sub,
        action = %action,
        required_scope = %required_scope,
        "MCP tool denied: insufficient scope"
    );
    Err(ErrorData::invalid_request(
        format!("forbidden: requires scope: {required_scope}"),
        None,
    ))
}

fn scope_satisfied(token_scopes: &[String], required: &str) -> bool {
    crate::actions::scopes_satisfy(token_scopes, required)
}

#[cfg(test)]
#[path = "rmcp_server_tests.rs"]
mod tests;
