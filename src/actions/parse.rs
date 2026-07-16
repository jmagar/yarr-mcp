//! Argument parsing: shared param extractors and `YarrAction` construction
//! from MCP args / REST params.

use anyhow::Result;
use serde_json::{Value, json};

use super::model::{ActionTransport, ValidationError, YarrAction};
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

/// Optional trimmed string field. Absent/null/blank values are `Ok(None)`;
/// present values of another JSON type are rejected rather than silently lost.
pub fn optional_string(args: &Value, field: &str) -> Result<Option<String>> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            Ok((!value.trim().is_empty()).then(|| value.trim().to_owned()))
        }
        Some(_) => Err(ValidationError::WrongType {
            field: field.into(),
        }
        .into()),
    }
}

/// Boolean field, defaulting to `false` only when absent/null. Present values of
/// another JSON type are rejected rather than silently coerced.
pub fn bool_arg(args: &Value, field: &str) -> Result<bool> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(ValidationError::WrongType {
            field: field.into(),
        }
        .into()),
    }
}

/// Optional integer field. Returns `Ok(None)` when the field is absent, but
/// errors with [`ValidationError::WrongType`] when it is present yet not a number
/// or numeric string. Use this instead of permissive `.and_then(..).ok()` parsing
/// so a malformed value surfaces a clear error rather than being silently dropped.
pub fn optional_i64(args: &Value, field: &str) -> Result<Option<i64>> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value_to_i64(value).map(Some).ok_or_else(|| {
            ValidationError::WrongType {
                field: field.into(),
            }
            .into()
        }),
    }
}

/// Assert a required param is present (and not null / not an empty-or-blank
/// string). Used to enforce a curated command's `required_params` at the dispatch
/// boundary. Mirrors the presence semantics the `string_arg`/`i64_arg` extractors
/// already apply, but works for any value type (strings, numbers, arrays) so a
/// numeric required param like `id` is also enforced. Errors with the canonical
/// [`ValidationError::MissingField`].
pub fn require_present(args: &Value, field: &str) -> Result<()> {
    match args.get(field) {
        None | Some(Value::Null) => Err(ValidationError::MissingField {
            field: field.into(),
        }
        .into()),
        Some(Value::String(s)) if s.trim().is_empty() => Err(ValidationError::MissingField {
            field: field.into(),
        }
        .into()),
        Some(_) => Ok(()),
    }
}

/// Coerce a JSON value to an `i64` from a number or a numeric string.
fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|s| s.trim().parse::<i64>().ok()))
}

// ── action parsing ──────────────────────────────────────────────────────────────

impl YarrAction {
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
            }),
            "api_put" => Ok(Self::ApiPut {
                service: string_arg(params, "service")?,
                path: string_arg(params, "path")?,
                body: params.get("body").cloned().unwrap_or_else(|| json!({})),
            }),
            "api_delete" => Ok(Self::ApiDelete {
                service: string_arg(params, "service")?,
                path: string_arg(params, "path")?,
                body: params.get("body").cloned(),
            }),
            "help" => Ok(Self::Help),
            "codemode" => Ok(Self::CodeMode {
                code: string_arg(params, "code")?,
            }),
            "snippet_list" => Ok(Self::SnippetList),
            "snippet_save" => Ok(Self::SnippetSave {
                name: string_arg(params, "name")?,
                code: string_arg(params, "code")?,
                description: optional_string(params, "description")?,
            }),
            "snippet_run" => Ok(Self::SnippetRun {
                name: string_arg(params, "name")?,
                input: params.get("input").cloned().unwrap_or(Value::Null),
            }),
            "snippet_delete" => Ok(Self::SnippetDelete {
                name: string_arg(params, "name")?,
            }),
            "op" => Ok(Self::Op {
                service: string_arg(params, "service")?,
                op: string_arg(params, "op")?,
                args: params.get("args").cloned().unwrap_or_else(|| json!({})),
            }),
            // Curated commands are not enum variants: resolve the action name in
            // the registry's descriptor table. The handler extracts its own
            // params from `params`, so we only validate the always-required
            // `service` here (curated commands all target one service) and carry
            // the raw params object through to dispatch.
            other => match curated_command(other) {
                Some(cmd) => {
                    // Enforce every declared required param at the dispatch
                    // boundary so `required_params` is load-bearing (it agrees with
                    // the schema/help AND guards the handler): a missing one yields
                    // the canonical `MissingField` error before the handler runs.
                    for field in cmd.required_params {
                        require_present(params, field)?;
                    }
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
