//! Generated-operation executor (business layer).
//!
//! Turns one `(service, op, args)` Code Mode call into an upstream request using
//! the generated `OperationSpec` table. The op's path/method come from the
//! vendored OpenAPI spec (trusted); the arg *values* are user input and are
//! percent-encoded by `build_operation_url`. This is the single dispatch point
//! for the entire generated surface — there is no per-operation Rust code.

use anyhow::{Result, anyhow, bail};
use serde_json::Value;

use crate::app::RustarrService;
use crate::openapi;
use crate::rustarr::helpers::build_operation_url;

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
        let mut query: Vec<(&str, String)> = Vec::new();
        for name in spec.query_params {
            if let Some(value) = args.get(*name) {
                let rendered = openapi::scalar_to_string(value).ok_or_else(|| {
                    anyhow!("query param `{name}` for `{op}` must be a string/number/bool")
                })?;
                query.push((name, rendered));
            }
        }

        let url = build_operation_url(config, spec.path, &path_args, &query)?;
        let method = parse_method(spec.method)?;
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

/// Parse the spec's uppercase method string into a `reqwest::Method`.
fn parse_method(method: &str) -> Result<reqwest::Method> {
    match method {
        "GET" => Ok(reqwest::Method::GET),
        "POST" => Ok(reqwest::Method::POST),
        "PUT" => Ok(reqwest::Method::PUT),
        "DELETE" => Ok(reqwest::Method::DELETE),
        "PATCH" => Ok(reqwest::Method::PATCH),
        "HEAD" => Ok(reqwest::Method::HEAD),
        other => bail!("unsupported HTTP method `{other}`"),
    }
}
