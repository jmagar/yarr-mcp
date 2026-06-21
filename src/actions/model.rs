//! Action data model: validation errors, scopes, spec/transport types, and the
//! `RustarrAction` enum for the generic passthrough actions.
//!
//! Curated commands are *not* enum variants — they live in the data-driven
//! [`crate::actions::registry`] descriptor table. This enum covers only the six
//! generic actions plus `help`, which is small enough to stay exhaustive.

use serde_json::Value;

pub const READ_SCOPE: &str = "rustarr:read";
pub const WRITE_SCOPE: &str = "rustarr:write";
pub const DENY_SCOPE: &str = "rustarr:__deny__";

pub fn scopes_satisfy(token_scopes: &[String], required: &str) -> bool {
    token_scopes
        .iter()
        .any(|s| s == required || (required == READ_SCOPE && s == WRITE_SCOPE))
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ValidationError {
    #[error("action is required")]
    MissingAction,
    #[error("`{field}` is required and must not be empty")]
    MissingField { field: String },
    #[error("`{field}` has the wrong type")]
    WrongType { field: String },
    #[error("action={action} is not available over REST; use MCP or action=help for documentation")]
    NotAvailableOverRest { action: String },
    #[error("unknown rustarr action: {action}; use action=help for documentation")]
    UnknownAction { action: String },
    #[error(
        "action={action} is not valid for kind={kind}; valid actions for {kind}: [{}]",
        valid_actions.join(", ")
    )]
    ActionNotValidForKind {
        action: String,
        kind: String,
        valid_actions: Vec<String>,
    },
}

pub fn is_validation_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<ValidationError>().is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTransport {
    Any,
    McpOnly,
}

/// Static spec for a generic (infrastructure) action. Drives schema, scope
/// lookup, REST/MCP filtering, and name enumeration so the action list has a
/// single materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionSpec {
    pub name: &'static str,
    pub required_scope: Option<&'static str>,
    pub transport: ActionTransport,
}

/// The six generic passthrough actions plus `help`.
#[derive(Debug, Clone, PartialEq)]
pub enum RustarrAction {
    Integrations,
    ServiceStatus {
        service: String,
    },
    ApiGet {
        service: String,
        path: String,
    },
    ApiPost {
        service: String,
        path: String,
        body: Value,
    },
    ApiPut {
        service: String,
        path: String,
        body: Value,
    },
    ApiDelete {
        service: String,
        path: String,
        body: Option<Value>,
        confirm: bool,
    },
    Help,
    /// A curated, capability-scoped command resolved from the registry's
    /// descriptor table (e.g. `quality_profiles`, `list`). Carries the registry
    /// command `name` and the raw `params` object so dispatch can hand both to the
    /// descriptor's handler. Curated commands are NOT enum variants — this single
    /// carrier keeps the enum small while every command stays data-driven.
    Curated {
        name: &'static str,
        params: Value,
    },
}

impl RustarrAction {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Integrations => "integrations",
            Self::ServiceStatus { .. } => "service_status",
            Self::ApiGet { .. } => "api_get",
            Self::ApiPost { .. } => "api_post",
            Self::ApiPut { .. } => "api_put",
            Self::ApiDelete { .. } => "api_delete",
            Self::Help => "help",
            Self::Curated { name, .. } => name,
        }
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
