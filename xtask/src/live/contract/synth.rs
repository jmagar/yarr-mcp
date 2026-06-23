//! Spec helpers for the contract harness: load a vendored OpenAPI spec, index its
//! operations, synthesize request inputs from the schema, and validate a response
//! against the operation's declared success-response schema (JSON-Schema with
//! `$ref` resolution into `components/schemas`).

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

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
            serde_yaml::from_str(&text)?
        };
        let mut ops = BTreeMap::new();
        if let Some(paths) = doc.get("paths").and_then(Value::as_object) {
            for (p, item) in paths {
                for m in ["get", "post", "put", "delete", "patch"] {
                    if let Some(op) = item.get(m) {
                        ops.insert((m.to_uppercase(), p.clone()), op.clone());
                    }
                }
            }
        }
        Ok(Spec { doc, ops })
    }

    /// The `components/schemas` map (for `$ref` resolution + validation).
    fn components(&self) -> Value {
        self.doc
            .get("components")
            .and_then(|c| c.get("schemas"))
            .cloned()
            .unwrap_or_else(|| json!({}))
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
    /// JSON contract and are skipped. Ops with no declared content (204) or a JSON
    /// content type return false (the harness proceeds).
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

        // Request body (JSON).
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
                let mut obj = Map::new();
                let required: Vec<String> = schema
                    .get("required")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                if let Some(props) = schema.get("properties").and_then(Value::as_object) {
                    for name in &required {
                        if let Some(ps) = props.get(name)
                            && let Some(v) = self.sample_depth(ps, depth + 1)
                        {
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
