//! Generated-operation executor (business layer).
//!
//! Turns one `(service, op, args)` Code Mode call into an upstream request using
//! the generated `OperationSpec` table. The op's path/method come from the
//! vendored OpenAPI spec (trusted); the arg *values* are user input and are
//! percent-encoded by `build_operation_url`. This is the single dispatch point
//! for the entire generated surface — there is no per-operation Rust code.

use anyhow::{Result, anyhow};
use serde_json::Value;

use crate::app::RustarrService;
use crate::openapi;
use crate::yarr::helpers::build_operation_url;

impl RustarrService {
    /// Execute a generated operation: `service` resolves the configured upstream,
    /// `op` names the `OperationSpec`, and `args` carries path params, query
    /// params, and (for body operations) `args.body`.
    pub async fn execute_operation(&self, service: &str, op: &str, args: &Value) -> Result<Value> {
        let config = self.service(service)?;
        let kind = config.kind;
        let spec = openapi::find_operation(kind, op)
            .ok_or_else(|| anyhow!("unknown {} operation `{op}`", kind.as_str()))?;

        // Resolve required path params (encoded as single segments downstream).
        let mut path_args: Vec<(&str, String)> = Vec::with_capacity(spec.path_params.len());
        for name in spec.path_params {
            let value = args
                .get(*name)
                .and_then(openapi::scalar_to_string)
                .ok_or_else(|| {
                    anyhow!("operation `{op}` requires path param `{name}` (a string/number)")
                })?;
            path_args.push((name, value));
        }

        // Optional query params: include only those present in `args`.
        let mut query_owned: Vec<(String, String)> = Vec::new();
        for name in spec.query_params {
            if let Some(value) = args.get(*name) {
                query_owned.extend(query_arg_values(name, value)?);
            }
        }
        let query: Vec<(&str, String)> = query_owned
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone()))
            .collect();

        let url = build_operation_url(config, spec.path, &path_args, &query)?;
        let method = spec.method.as_reqwest();
        let body = if spec.has_body {
            args.get("body").cloned()
        } else {
            None
        };
        // Always negotiate JSON: Plex defaults to XML otherwise, and it is harmless
        // for the header-auth kinds.
        self.client_ref()
            .request_url(method, config, url, body, Some("application/json"))
            .await
    }
}

fn query_arg_values(name: &str, value: &Value) -> Result<Vec<(String, String)>> {
    match value {
        Value::Array(values) => values
            .iter()
            .map(|value| {
                openapi::scalar_to_string(value)
                    .map(|rendered| (name.to_string(), rendered))
                    .ok_or_else(|| {
                        anyhow!("query param `{name}` array items must be strings/numbers/bools")
                    })
            })
            .collect(),
        other => openapi::scalar_to_string(other)
            .map(|rendered| vec![(name.to_string(), rendered)])
            .ok_or_else(|| anyhow!("query param `{name}` must be a string/number/bool or array")),
    }
}

impl RustarrService {
    /// True iff `op` is a generated DELETE operation on `service` — i.e. a
    /// destructive generated op. The single source of truth for the destructive
    /// gate, shared by the CLI `op` verb (which requires `--confirm`) and the Code
    /// Mode dispatch (which refuses it mid-script), so the policy isn't duplicated.
    pub(crate) fn op_is_destructive_delete(&self, service: &str, op: &str) -> bool {
        self.kind_of(service)
            .and_then(|kind| openapi::find_operation(kind, op))
            .is_some_and(|spec| spec.method.is_delete())
    }
}

#[cfg(test)]
#[path = "openapi_ops_tests.rs"]
mod tests;
