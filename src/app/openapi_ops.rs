//! Generated-operation executor (business layer).
//!
//! Turns one `(service, op, args)` Code Mode call into an upstream request using
//! the generated `OperationSpec` table. The op's path/method come from the
//! vendored OpenAPI spec (trusted); the arg *values* are user input and are
//! percent-encoded by `build_operation_url`. This is the single dispatch point
//! for the entire generated surface — there is no per-operation Rust code.

use anyhow::{Context, Result, anyhow, ensure};
use base64::Engine;
use serde_json::Value;
use std::path::Path;

use crate::app::YarrService;
use crate::openapi;
use crate::yarr::helpers::build_operation_url;

impl YarrService {
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
        if let Some(encoded) = args.get("multipartFileBase64").and_then(Value::as_str) {
            ensure!(
                method == reqwest::Method::POST || method == reqwest::Method::PUT,
                "multipartFileBase64 is only supported for generated POST/PUT operations"
            );
            ensure!(
                args.get("body").is_none(),
                "multipartFileBase64 cannot be combined with body"
            );
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .context("decode multipartFileBase64")?;
            ensure!(
                bytes.len() as u64 <= MAX_MULTIPART_UPLOAD_BYTES,
                "multipartFileBase64 exceeds {} bytes",
                MAX_MULTIPART_UPLOAD_BYTES
            );
            let file_name = args
                .get("fileName")
                .and_then(Value::as_str)
                .unwrap_or("upload.bin");
            let field_name = args
                .get("multipartField")
                .and_then(Value::as_str)
                .unwrap_or("file");
            return self
                .client_ref()
                .request_url_multipart_file(method, config, url, field_name, file_name, bytes)
                .await;
        }
        if let Some(fixture) = args.get("multipartFixture").and_then(Value::as_str) {
            ensure!(
                method == reqwest::Method::POST || method == reqwest::Method::PUT,
                "multipartFixture is only supported for generated POST/PUT operations"
            );
            ensure!(
                args.get("body").is_none(),
                "multipartFixture cannot be combined with body"
            );
            let bytes = read_live_multipart_fixture(fixture)?;
            ensure!(
                bytes.len() as u64 <= MAX_MULTIPART_UPLOAD_BYTES,
                "multipartFixture exceeds {} bytes",
                MAX_MULTIPART_UPLOAD_BYTES
            );
            let file_name = args
                .get("fileName")
                .and_then(Value::as_str)
                .unwrap_or(fixture);
            let field_name = args
                .get("multipartField")
                .and_then(Value::as_str)
                .unwrap_or("file");
            return self
                .client_ref()
                .request_url_multipart_file(method, config, url, field_name, file_name, bytes)
                .await;
        }
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

const MAX_MULTIPART_UPLOAD_BYTES: u64 = 32 * 1024 * 1024;

fn read_live_multipart_fixture(name: &str) -> Result<Vec<u8>> {
    ensure!(
        std::env::var("YARR_ALLOW_DESTRUCTIVE").as_deref() == Ok("true"),
        "multipartFixture is only enabled for disposable destructive live-test stacks"
    );
    ensure!(
        !name.is_empty()
            && !name.contains('/')
            && !name.contains('\\')
            && name != "."
            && name != "..",
        "multipartFixture must be a filename under target/live-full/tmp"
    );
    let root = Path::new("target/live-full/tmp")
        .canonicalize()
        .context("resolve live multipart fixture root")?;
    let path = root
        .join(name)
        .canonicalize()
        .with_context(|| format!("resolve live multipart fixture {name}"))?;
    ensure!(
        path.starts_with(&root),
        "multipartFixture must live under {}",
        root.display()
    );
    let meta = std::fs::metadata(&path)
        .with_context(|| format!("stat live multipart fixture {}", path.display()))?;
    ensure!(meta.is_file(), "multipartFixture must be a regular file");
    std::fs::read(&path).with_context(|| format!("read live multipart fixture {}", path.display()))
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

#[cfg(test)]
#[path = "openapi_ops_tests.rs"]
mod tests;
