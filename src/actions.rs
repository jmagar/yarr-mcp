use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::app::ExampleService;

// ── Validation error type ─────────────────────────────────────────────────────

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
            Self::WrongType { field } => write!(f, "`{field}` must be a string"),
            Self::NotAvailableOverRest { action } => write!(
                f,
                "action={action} is not available over REST; use MCP or action=help for documentation"
            ),
            Self::UnknownAction { action } => write!(
                f,
                "unknown example action: {action}; use action=help for documentation"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

pub const READ_SCOPE: &str = "example:read";
pub const WRITE_SCOPE: &str = "example:write";
pub const DENY_SCOPE: &str = "example:__deny__";

/// Returns true if `token_scopes` satisfy `required`.
/// Write scope satisfies read (write ⊇ read).
/// Single source of truth — called from both REST and MCP enforcement paths.
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
        name: "greet",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "echo",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "status",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "elicit_name",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::McpOnly,
    },
    ActionSpec {
        name: "scaffold_intent",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::McpOnly,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExampleAction {
    Greet { name: Option<String> },
    Echo { message: String },
    Status,
    Help,
    ElicitName,
    ScaffoldIntent,
}

impl ExampleAction {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Greet { .. } => "greet",
            Self::Echo { .. } => "echo",
            Self::Status => "status",
            Self::Help => "help",
            Self::ElicitName => "elicit_name",
            Self::ScaffoldIntent => "scaffold_intent",
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
            "greet" => Ok(Self::Greet {
                name: optional_string_param(params, "name")?,
            }),
            "echo" => {
                let message = optional_string_param(params, "message")?
                    .filter(|m| !m.is_empty())
                    .ok_or_else(|| ValidationError::MissingField {
                        field: "message".into(),
                    })?;
                Ok(Self::Echo { message })
            }
            "status" => Ok(Self::Status),
            "help" => Ok(Self::Help),
            "elicit_name" => Ok(Self::ElicitName),
            "scaffold_intent" => Ok(Self::ScaffoldIntent),
            other => Err(ValidationError::UnknownAction {
                action: other.to_owned(),
            }
            .into()),
        }
    }
}

pub async fn execute_service_action(
    service: &ExampleService,
    action: &ExampleAction,
) -> Result<Value> {
    match action {
        ExampleAction::Greet { name } => service.greet(name.as_deref()).await,
        ExampleAction::Echo { message } => service.echo(message).await,
        ExampleAction::Status => service.status().await,
        ExampleAction::Help => Ok(rest_help()),
        ExampleAction::ElicitName => Err(anyhow!(
            "action=elicit_name is only available over MCP because it requires a peer"
        )),
        ExampleAction::ScaffoldIntent => Err(anyhow!(
            "action=scaffold_intent is only available over MCP because it requires elicitation"
        )),
    }
}

pub fn rest_help() -> Value {
    json!({
        "actions": rest_action_names(),
        "mcp_only_actions": mcp_only_action_names(),
        "usage": "POST /v1/example with {\"action\": \"<action>\", \"params\": {...}}",
        "examples": {
            "greet":  {"action": "greet",  "params": {"name": "Alice"}},
            "echo":   {"action": "echo",   "params": {"message": "Hello!"}},
            "status": {"action": "status", "params": {}},
        }
    })
}

fn optional_string_param(params: &Value, name: &str) -> Result<Option<String>> {
    match params.get(name) {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .map(|s| Some(s.to_owned()))
            .ok_or_else(|| ValidationError::WrongType { field: name.into() }.into()),
    }
}

pub fn is_validation_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<ValidationError>().is_some()
        || error
            .downcast_ref::<crate::app::ScaffoldIntentValidationError>()
            .is_some()
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
