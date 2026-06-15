//! Argument parsing: shared param extractors and `RustarrAction` construction
//! from MCP args / REST params.

use anyhow::Result;
use serde_json::{json, Value};

use super::model::{ActionTransport, RustarrAction, ValidationError};
use super::registry::{action_spec, curated_command};

// ── shared param extractors (reused by curated command handlers too) ────────────

/// Require a non-empty trimmed string field. Errors with `MissingField` /
/// `WrongType` validation errors so callers surface friendly messages.
pub fn string_arg(args: &Value, field: &str) -> Result<String> {
    let value = args
        .get(field)
        .ok_or_else(|| ValidationError::MissingField {
            field: field.into(),
        })?;
    let value = value
        .as_str()
        .ok_or_else(|| ValidationError::WrongType {
            field: field.into(),
        })?
        .trim()
        .to_owned();
    if value.is_empty() {
        return Err(ValidationError::MissingField {
            field: field.into(),
        }
        .into());
    }
    Ok(value)
}

/// Optional trimmed string field; `None` when absent, empty, or not a string.
pub fn optional_string(args: &Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

/// Boolean field, defaulting to `false` when absent or not a bool.
pub fn bool_arg(args: &Value, field: &str) -> bool {
    args.get(field).and_then(Value::as_bool).unwrap_or(false)
}

// ── action parsing ──────────────────────────────────────────────────────────────

impl RustarrAction {
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
                service: string_arg(params, "service")?,
            }),
            "api_get" => Ok(Self::ApiGet {
                service: string_arg(params, "service")?,
                path: string_arg(params, "path")?,
            }),
            "api_post" => Ok(Self::ApiPost {
                service: string_arg(params, "service")?,
                path: string_arg(params, "path")?,
                body: params.get("body").cloned().unwrap_or_else(|| json!({})),
                confirm: bool_arg(params, "confirm"),
            }),
            "api_put" => Ok(Self::ApiPut {
                service: string_arg(params, "service")?,
                path: string_arg(params, "path")?,
                body: params.get("body").cloned().unwrap_or_else(|| json!({})),
                confirm: bool_arg(params, "confirm"),
            }),
            "api_delete" => Ok(Self::ApiDelete {
                service: string_arg(params, "service")?,
                path: string_arg(params, "path")?,
                body: params.get("body").cloned(),
                confirm: bool_arg(params, "confirm"),
            }),
            "help" => Ok(Self::Help),
            // Curated commands are not enum variants: resolve the action name in
            // the registry's descriptor table. The handler extracts its own
            // params from `params`, so we only validate the always-required
            // `service` here (curated commands all target one service) and carry
            // the raw params object through to dispatch.
            other => match curated_command(other) {
                Some(cmd) => {
                    string_arg(params, "service")?;
                    Ok(Self::Curated {
                        name: cmd.name,
                        params: params.clone(),
                    })
                }
                None => Err(ValidationError::UnknownAction {
                    action: other.to_owned(),
                }
                .into()),
            },
        }
    }
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
