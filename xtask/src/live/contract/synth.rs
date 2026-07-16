//! Spec helpers for the contract harness: load a vendored OpenAPI spec, index its
//! operations, synthesize request inputs from the schema, and validate a response
//! against the operation's declared success-response schema (JSON-Schema with
//! `$ref` resolution into `components/schemas`).

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// The path component of the spec's first `servers.url` — the same prefix the
/// generator (`xtask::gen_openapi`) applies, so harness lookups match `op.path`.
/// Host-only URLs yield an empty base; `{server}/api/v1` yields `/api/v1`.
fn server_base_path(doc: &Value) -> String {
    let url = doc
        .get("servers")
        .and_then(Value::as_array)
        .and_then(|s| s.first())
        .and_then(|s| s.get("url"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let after_host = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    match after_host.find('/') {
        Some(i) => after_host[i..].trim_end_matches('/').to_string(),
        None => String::new(),
    }
}

/// A parsed spec: the raw doc plus an index of `(METHOD, path)` -> operation object.
pub struct Spec {
    pub doc: Value,
    pub ops: BTreeMap<(String, String), Value>,
}

impl Spec {
    pub fn load(path: &str) -> Result<Spec> {
        let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
        let doc: Value = if path.ends_with(".json") {
            serde_json::from_str(&text)?
        } else {
            noyalib::from_str_strict(&text)?
        };
        // Operation paths are stored under the generator's full path (server base
        // path + the spec path), so harness lookups by `op.path` match. Most specs
        // have an empty base; Overseerr's is `/api/v1`.
        let base = server_base_path(&doc);
        let mut ops = BTreeMap::new();
        if let Some(paths) = doc.get("paths").and_then(Value::as_object) {
            for (p, item) in paths {
                for m in ["get", "post", "put", "delete", "patch"] {
                    if let Some(op) = item.get(m) {
                        ops.insert((m.to_uppercase(), format!("{base}{p}")), op.clone());
                    }
                }
            }
        }
        Ok(Spec { doc, ops })
    }

    /// The `components/schemas` map (for `$ref` resolution + validation),
    /// relaxed from OpenAPI 3.0 dialect to the JSON-Schema the `jsonschema` crate
    /// speaks: `nullable: true` is rewritten to also accept `null`, and the
    /// closed-world `additionalProperties: false` is dropped. See `relax_for_client`.
    fn components(&self) -> Value {
        let mut schemas = self
            .doc
            .get("components")
            .and_then(|c| c.get("schemas"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        relax_for_client(&mut schemas);
        relax_known_server_drifts(&mut schemas);
        schemas
    }

    /// Resolve a local `#/...` pointer within the spec doc.
    fn resolve<'a>(&'a self, pointer: &str) -> Option<&'a Value> {
        let rest = pointer.strip_prefix("#/")?;
        let mut cur = &self.doc;
        for seg in rest.split('/') {
            let seg = seg.replace("~1", "/").replace("~0", "~");
            cur = cur.get(&seg)?;
        }
        Some(cur)
    }

    /// Validate `instance` against the named component schema. `type_name` is the
    /// generated response type — the ELEMENT type for array responses — so an array
    /// instance is validated element-wise. Returns `Ok(())` on conformance (or an
    /// empty array), `Err` with the first validation message otherwise.
    pub fn validate_response(&self, type_name: &str, instance: &Value) -> Result<()> {
        let schema = json!({
            "$ref": format!("#/components/schemas/{type_name}"),
            "components": { "schemas": self.components() },
        });
        let validator = jsonschema::validator_for(&schema)
            .with_context(|| format!("compile schema for {type_name}"))?;
        // Array responses carry the element type; validate each item.
        let items: Vec<&Value> = match instance {
            Value::Array(a) => a.iter().collect(),
            other => vec![other],
        };
        for item in items {
            if !validator.is_valid(item) {
                let msg = validator
                    .iter_errors(item)
                    .next()
                    .map(|e| format!("{e}"))
                    .unwrap_or_else(|| "schema mismatch".into());
                anyhow::bail!("response item does not match {type_name}: {msg}");
            }
        }
        Ok(())
    }

    /// True iff the operation's success (2xx) response declares a body whose only
    /// content types are NON-JSON (e.g. `text/calendar`, a file). Such ops aren't a
    /// JSON contract. Ops with no declared content (204) or a JSON content type
    /// return false. Kept for diagnostics even though the live harness now invokes
    /// non-JSON endpoints instead of skipping them.
    #[allow(dead_code)]
    pub fn success_is_nonjson(&self, method: &str, path: &str) -> bool {
        let Some(op) = self.ops.get(&(method.to_string(), path.to_string())) else {
            return false;
        };
        let Some(responses) = op.get("responses").and_then(Value::as_object) else {
            return false;
        };
        let Some((_, resp)) = responses
            .iter()
            .find(|(k, _)| k.starts_with('2'))
            .or_else(|| responses.iter().find(|(k, _)| *k == "default"))
        else {
            return false;
        };
        match resp.get("content").and_then(Value::as_object) {
            Some(c) if !c.is_empty() => !c.keys().any(|k| k.contains("json")),
            _ => false,
        }
    }

    /// Build the `--args` object for an operation: required query params from the
    /// spec (example/default/enum/type-default), and a synthesized request body
    /// (`args.body`) when the operation declares one. Path params are supplied by
    /// the caller (from discovered IDs). Returns `None` to SKIP (a required body we
    /// cannot synthesize without guessing).
    pub fn build_args(
        &self,
        method: &str,
        path: &str,
        path_args: &Map<String, Value>,
    ) -> Option<Map<String, Value>> {
        let fixture = ensure_default_multipart_fixture().ok()?;
        self.build_args_with_multipart_fixture(method, path, path_args, &fixture)
    }

    pub(super) fn build_args_with_multipart_fixture(
        &self,
        method: &str,
        path: &str,
        path_args: &Map<String, Value>,
        multipart_fixture: &std::path::Path,
    ) -> Option<Map<String, Value>> {
        let op = self.ops.get(&(method.to_string(), path.to_string()))?;
        let mut args = path_args.clone();

        // Required query params.
        if let Some(params) = op.get("parameters").and_then(Value::as_array) {
            for p in params {
                let p = self.deref(p);
                if p.get("in").and_then(Value::as_str) == Some("query")
                    && p.get("required").and_then(Value::as_bool) == Some(true)
                    && let Some(name) = p.get("name").and_then(Value::as_str)
                    && let Some(v) = self.sample(p.get("schema").unwrap_or(&Value::Null))
                {
                    args.insert(name.to_string(), v);
                }
            }
        }

        // Request body. The live harness owns fixture filesystem access; the
        // production dispatcher receives only base64 bytes and a filename.
        let multipart = op
            .get("requestBody")
            .and_then(|body| body.get("content"))
            .and_then(|content| content.get("multipart/form-data"));
        if multipart.is_some() || path.ends_with("/system/backup/restore/upload") {
            let bytes = std::fs::read(multipart_fixture).ok()?;
            let field = multipart
                .and_then(|media| media.get("schema"))
                .and_then(|schema| schema.get("properties"))
                .and_then(Value::as_object)
                .and_then(|properties| {
                    properties.iter().find_map(|(name, schema)| {
                        (schema.get("format").and_then(Value::as_str) == Some("binary"))
                            .then_some(name.as_str())
                    })
                })
                .unwrap_or("file");
            args.insert("multipartField".into(), json!(field));
            args.insert(
                "fileName".into(),
                json!(multipart_fixture.file_name()?.to_str()?),
            );
            args.insert("multipartFileBase64".into(), json!(base64_encode(&bytes)));
        }

        // JSON request body.
        if let Some(schema) = op
            .get("requestBody")
            .and_then(|b| b.get("content"))
            .and_then(Value::as_object)
            .and_then(|c| c.iter().find(|(k, _)| k.contains("json")).map(|(_, m)| m))
            .and_then(|m| m.get("schema"))
        {
            let body = self.sample(schema).unwrap_or_else(|| json!({}));
            args.insert("body".into(), body);
        }
        Some(args)
    }

    /// Resolve a `$ref` node one level (parameters / schemas).
    fn deref(&self, node: &Value) -> Value {
        if let Some(r) = node.get("$ref").and_then(Value::as_str)
            && let Some(v) = self.resolve(r)
        {
            return v.clone();
        }
        node.clone()
    }

    /// Synthesize a sample value for a schema: prefer `example`/`default`/`enum`,
    /// else build from `type` (objects use only `required` properties). Bounded
    /// recursion via `depth`.
    pub fn sample(&self, schema: &Value) -> Option<Value> {
        self.sample_depth(schema, 0)
    }

    fn sample_depth(&self, schema: &Value, depth: usize) -> Option<Value> {
        if depth > 6 {
            return None;
        }
        let schema = self.deref(schema);
        if let Some(ex) = schema.get("example").or_else(|| schema.get("default")) {
            return Some(ex.clone());
        }
        if let Some(first) = schema
            .get("enum")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
        {
            return Some(first.clone());
        }
        for key in ["allOf", "oneOf", "anyOf"] {
            if let Some(parts) = schema.get(key).and_then(Value::as_array)
                && let Some(first) = parts.first()
            {
                return self.sample_depth(first, depth + 1);
            }
        }
        let ty = schema
            .get("type")
            .and_then(Value::as_str)
            .or_else(|| schema.get("properties").map(|_| "object"));
        match ty {
            Some("string") => Some(json!(schema.get("format").and_then(Value::as_str).map_or(
                "x",
                |f| match f {
                    "date-time" => "2024-01-01T00:00:00Z",
                    "date" => "2024-01-01",
                    _ => "x",
                }
            ))),
            Some("integer") => Some(json!(1)),
            Some("number") => Some(json!(1.0)),
            Some("boolean") => Some(json!(false)),
            Some("array") => Some(json!([])),
            Some("object") => {
                // Populate EVERY property (Servarr marks nothing `required`, so a
                // required-only body would be `{}` and a create would 500). Skip
                // `readOnly` fields — server-assigned `id`s etc. don't belong in a
                // request body and would be rejected.
                let mut obj = Map::new();
                if let Some(props) = schema.get("properties").and_then(Value::as_object) {
                    for (name, ps) in props {
                        // Skip `readOnly` and a top-level server-assigned `id`: the
                        // Servarr specs don't flag `id` readOnly, but POSTing one
                        // 500s ("Can't insert model with existing ID"). Nested `id`s
                        // (depth > 0) are kept.
                        if (name == "id" && depth == 0)
                            || self.deref(ps).get("readOnly").and_then(Value::as_bool) == Some(true)
                        {
                            continue;
                        }
                        if let Some(v) = self.sample_depth(ps, depth + 1) {
                            obj.insert(name.clone(), v);
                        }
                    }
                }
                Some(Value::Object(obj))
            }
            _ => Some(json!({})),
        }
    }
}

fn ensure_default_multipart_fixture() -> std::io::Result<std::path::PathBuf> {
    let path = std::path::PathBuf::from("target/live-full/tmp/openapi-empty-backup.zip");
    if !path.exists() {
        std::fs::create_dir_all(path.parent().expect("fixture path has parent"))?;
        // Valid empty ZIP end-of-central-directory record. Upstreams may reject it
        // as a backup, but the contract run still exercises multipart transport.
        let mut empty_zip = b"PK\x05\x06".to_vec();
        empty_zip.resize(22, 0);
        std::fs::write(&path, empty_zip)?;
    }
    Ok(path)
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        output.push(TABLE[((value >> 18) & 63) as usize] as char);
        output.push(TABLE[((value >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[((value >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(value & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

/// Rewrite an OpenAPI 3.0 schema tree into the JSON-Schema dialect the
/// `jsonschema` crate validates against, so a faithful contract check doesn't
/// fire on dialect differences the generated client would never trip on:
///
/// * `nullable: true` (OpenAPI's null marker; unknown to JSON Schema) is folded
///   into the type so an actual `null` validates — `type: string` becomes
///   `type: [string, null]`, and a typeless `$ref`/`allOf` is wrapped in an
///   `anyOf` with `{ type: null }`. Generated fields for these are `Option<T>`.
/// * `additionalProperties: false` is dropped — a client (serde) ignores unknown
///   fields, so an extra server field is not a contract break; enforcing the
///   spec's closed world would reject forward-compatible responses.
#[path = "synth/relax.rs"]
mod relax;
use relax::*;

#[cfg(test)]
#[path = "synth_tests.rs"]
mod tests;
