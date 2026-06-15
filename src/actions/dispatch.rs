//! Dispatch: map a parsed `RustarrAction` to the corresponding service method.
//!
//! This is the SHARED dispatch path for BOTH the CLI and MCP shims, so the
//! action×kind validation guard lives here (LD4 / architecture F4-a) rather than
//! in `mcp/rmcp_server.rs` — the CLI never touches that file. Every action that
//! targets a service is checked against the resolved [`ServiceKind`] before it
//! runs, so a curated command can never reach an incompatible kind regardless of
//! which transport invoked it.

use anyhow::Result;
use serde_json::Value;

use super::help::rest_help;
use super::model::{RustarrAction, ValidationError};
use super::registry::{action_allowed_for_kind, valid_actions_for_kind};
use crate::app::RustarrService;

/// Validate that `action` (by name) may run against the service named `service_name`.
///
/// Resolves the configured service's [`ServiceKind`] and calls
/// [`action_allowed_for_kind`]. On mismatch it returns
/// [`ValidationError::ActionNotValidForKind`], which carries the valid-action
/// list so its `Display` teaches the agent what it *can* run (AN-2). Generic
/// infra actions are allowed for every kind, so this is a no-op for them.
///
/// When the service name does not resolve to a configured kind, validation is
/// skipped here and the downstream service lookup surfaces the canonical
/// "unknown service" error instead.
pub fn validate_action_for_service(
    service: &RustarrService,
    action_name: &str,
    service_name: &str,
) -> Result<()> {
    let Some(kind) = service.kind_of(service_name) else {
        return Ok(());
    };
    if action_allowed_for_kind(action_name, kind) {
        return Ok(());
    }
    Err(ValidationError::ActionNotValidForKind {
        action: action_name.to_owned(),
        kind: kind.as_str().to_owned(),
        valid_actions: valid_actions_for_kind(kind)
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
    }
    .into())
}

/// The service name an action targets, if any. Infra actions that don't address
/// a service (`integrations`, `help`) return `None`.
fn target_service(action: &RustarrAction) -> Option<&str> {
    match action {
        RustarrAction::Integrations | RustarrAction::Help => None,
        RustarrAction::ServiceStatus { service }
        | RustarrAction::ApiGet { service, .. }
        | RustarrAction::ApiPost { service, .. }
        | RustarrAction::ApiPut { service, .. }
        | RustarrAction::ApiDelete { service, .. } => Some(service),
    }
}

pub async fn execute_service_action(
    service: &RustarrService,
    action: &RustarrAction,
) -> Result<Value> {
    // Shared action×kind guard: runs for every action that targets a service,
    // on both the CLI and MCP paths. No-op for generic/infra actions.
    if let Some(service_name) = target_service(action) {
        validate_action_for_service(service, action.name(), service_name)?;
    }
    match action {
        RustarrAction::Integrations => Ok(service.integrations()),
        RustarrAction::ServiceStatus { service: name } => service.service_status(name).await,
        RustarrAction::ApiGet {
            service: name,
            path,
        } => service.api_get(name, path).await,
        RustarrAction::ApiPost {
            service: name,
            path,
            body,
            confirm,
        } => service.api_post(name, path, body.clone(), *confirm).await,
        RustarrAction::ApiPut {
            service: name,
            path,
            body,
            confirm,
        } => service.api_put(name, path, body.clone(), *confirm).await,
        RustarrAction::ApiDelete {
            service: name,
            path,
            body,
            confirm,
        } => service.api_delete(name, path, body.clone(), *confirm).await,
        RustarrAction::Help => Ok(rest_help()),
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
