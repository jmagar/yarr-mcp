use anyhow::Result;
use serde_json::{json, Value};

use crate::app::RustarrService;

#[derive(Debug)]
pub enum ValidationError {
    MissingAction,
    MissingField { field: String },
    WrongType { field: String },
    NotAvailableOverRest { action: String },
    UnknownAction { action: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAction => write!(f, "action is required"),
            Self::MissingField { field } => {
                write!(f, "`{field}` is required and must not be empty")
            }
            Self::WrongType { field } => write!(f, "`{field}` has the wrong type"),
            Self::NotAvailableOverRest { action } => write!(
                f,
                "action={action} is not available over REST; use MCP or action=help for documentation"
            ),
            Self::UnknownAction { action } => {
                write!(
                    f,
                    "unknown rustarr action: {action}; use action=help for documentation"
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

pub const READ_SCOPE: &str = "rustarr:read";
pub const WRITE_SCOPE: &str = "rustarr:write";
pub const DENY_SCOPE: &str = "rustarr:__deny__";

pub fn scopes_satisfy(token_scopes: &[String], required: &str) -> bool {
    token_scopes
        .iter()
        .any(|s| s == required || (required == READ_SCOPE && s == WRITE_SCOPE))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTransport {
    Any,
    McpOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionSpec {
    pub name: &'static str,
    pub required_scope: Option<&'static str>,
    pub transport: ActionTransport,
}

pub const ACTION_SPECS: &[ActionSpec] = &[
    ActionSpec {
        name: "integrations",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "service_status",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "api_get",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "api_post",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "api_put",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "api_delete",
        required_scope: Some(WRITE_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "help",
        required_scope: None,
        transport: ActionTransport::Any,
    },
];

pub fn action_names() -> Vec<&'static str> {
    ACTION_SPECS.iter().map(|spec| spec.name).collect()
}

pub fn is_known_action(action: &str) -> bool {
    ACTION_SPECS.iter().any(|spec| spec.name == action)
}

pub fn rest_action_names() -> Vec<&'static str> {
    ACTION_SPECS
        .iter()
        .filter(|spec| spec.transport == ActionTransport::Any)
        .map(|spec| spec.name)
        .collect()
}

pub fn is_rest_action(action: &str) -> bool {
    action_spec(action)
        .map(|spec| spec.transport == ActionTransport::Any)
        .unwrap_or(false)
}

pub fn mcp_only_action_names() -> Vec<&'static str> {
    ACTION_SPECS
        .iter()
        .filter(|spec| spec.transport == ActionTransport::McpOnly)
        .map(|spec| spec.name)
        .collect()
}

pub fn required_scope_for_action(action: &str) -> Option<&'static str> {
    action_spec(action)
        .map(|spec| spec.required_scope)
        .unwrap_or(Some(DENY_SCOPE))
}

fn action_spec(action: &str) -> Option<&'static ActionSpec> {
    ACTION_SPECS.iter().find(|spec| spec.name == action)
}

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
        confirm: bool,
    },
    ApiPut {
        service: String,
        path: String,
        body: Value,
        confirm: bool,
    },
    ApiDelete {
        service: String,
        path: String,
        body: Option<Value>,
        confirm: bool,
    },
    Help,
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
        }
    }

    pub fn from_mcp_args(args: &Value) -> Result<Self> {
        let action = args
            .get("action")
            .and_then(Value::as_str)
            .ok_or(ValidationError::MissingAction)?;
        Self::from_params(action, args)
    }

    pub fn from_rest(action: &str, params: &Value) -> Result<Self> {
        if action.is_empty() {
            return Err(ValidationError::MissingAction.into());
        }
        if action_spec(action)
            .map(|spec| spec.transport == ActionTransport::McpOnly)
            .unwrap_or(false)
        {
            return Err(ValidationError::NotAvailableOverRest {
                action: action.to_owned(),
            }
            .into());
        }
        Self::from_params(action, params)
    }

    fn from_params(action: &str, params: &Value) -> Result<Self> {
        match action {
            "integrations" => Ok(Self::Integrations),
            "service_status" => Ok(Self::ServiceStatus {
                service: required_string_param(params, "service")?,
            }),
            "api_get" => Ok(Self::ApiGet {
                service: required_string_param(params, "service")?,
                path: required_string_param(params, "path")?,
            }),
            "api_post" => Ok(Self::ApiPost {
                service: required_string_param(params, "service")?,
                path: required_string_param(params, "path")?,
                body: params.get("body").cloned().unwrap_or_else(|| json!({})),
                confirm: params
                    .get("confirm")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            }),
            "api_put" => Ok(Self::ApiPut {
                service: required_string_param(params, "service")?,
                path: required_string_param(params, "path")?,
                body: params.get("body").cloned().unwrap_or_else(|| json!({})),
                confirm: params
                    .get("confirm")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            }),
            "api_delete" => Ok(Self::ApiDelete {
                service: required_string_param(params, "service")?,
                path: required_string_param(params, "path")?,
                body: params.get("body").cloned(),
                confirm: params
                    .get("confirm")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            }),
            "help" => Ok(Self::Help),
            other => Err(ValidationError::UnknownAction {
                action: other.to_owned(),
            }
            .into()),
        }
    }
}

pub async fn execute_service_action(
    service: &RustarrService,
    action: &RustarrAction,
) -> Result<Value> {
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

pub fn rest_help() -> Value {
    json!({
        "actions": rest_action_names(),
        "mcp_only_actions": mcp_only_action_names(),
        "usage": "Use the rustarr MCP tool or CLI commands such as `rustarr get --service sonarr --path /api/v3/system/status`.",
        "examples": {
            "integrations": {"action": "integrations"},
            "service_status": {"action": "service_status", "service": "sonarr"},
            "api_get": {"action": "api_get", "service": "radarr", "path": "/api/v3/system/status"},
            "api_post": {"action": "api_post", "service": "overseerr", "path": "/api/v1/request", "body": {}, "confirm": true},
            "api_put": {"action": "api_put", "service": "sonarr", "path": "/api/v3/series/editor", "body": {}, "confirm": true},
            "api_delete": {"action": "api_delete", "service": "sonarr", "path": "/api/v3/series/123?deleteFiles=false", "confirm": true}
        }
    })
}

fn required_string_param(params: &Value, name: &str) -> Result<String> {
    let value = params
        .get(name)
        .ok_or_else(|| ValidationError::MissingField { field: name.into() })?;
    let value = value
        .as_str()
        .ok_or_else(|| ValidationError::WrongType { field: name.into() })?
        .trim()
        .to_owned();
    if value.is_empty() {
        return Err(ValidationError::MissingField { field: name.into() }.into());
    }
    Ok(value)
}

pub fn is_validation_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<ValidationError>().is_some()
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
