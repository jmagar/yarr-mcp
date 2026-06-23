//! `cargo xtask gen-openapi` — generate the Code Mode operation + type tables for
//! the 6 spec-backed services from the vendored OpenAPI specs under `specs/`.
//!
//! Output: `src/openapi/generated/<svc>.rs`, each with `OPERATIONS` (one
//! [`OperationSpec`] per spec operation) and `TYPES` (one `TypeDef` per component
//! schema, rendered as a TypeScript interface for `codemode.describe`).
//!
//! The generated `.rs` files contain only data (string/array literals), so they
//! always compile; correctness that matters is the operation metadata (name,
//! method, path, params) which drives live execution. The TS strings are
//! best-effort for display.

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// (service module name, spec path) for each generated service.
const SPECS: &[(&str, &str)] = &[
    ("sonarr", "specs/sonarr.openapi.json"),
    ("radarr", "specs/radarr.openapi.json"),
    ("prowlarr", "specs/prowlarr.openapi.json"),
    ("overseerr", "specs/overseerr.openapi.yml"),
    ("jellyfin", "specs/jellyfin.openapi.json"),
    ("plex", "specs/plex.openapi.yml"),
];

pub fn run(_args: &[String]) -> Result<()> {
    for (svc, spec_path) in SPECS {
        let root = load_spec(spec_path).with_context(|| format!("loading {spec_path}"))?;
        let ops = extract_operations(&root)
            .with_context(|| format!("extracting operations from {spec_path}"))?;
        let types = extract_types(&root);
        let code = emit_rust(svc, &ops, &types);
        let out = format!("src/openapi/generated/{svc}.rs");
        std::fs::write(&out, code).with_context(|| format!("writing {out}"))?;
        println!(
            "  {svc:9} -> {out}  ({} ops, {} types)",
            ops.len(),
            types.len()
        );
    }
    println!("gen-openapi: done. Run `cargo fmt` + `cargo build` to verify.");
    Ok(())
}

fn load_spec(path: &str) -> Result<Value> {
    let text = std::fs::read_to_string(path)?;
    if path.ends_with(".json") {
        Ok(serde_json::from_str(&text)?)
    } else {
        // YAML specs (overseerr, plex) → serde_json::Value via serde_yaml.
        Ok(serde_yaml::from_str(&text)?)
    }
}

// ── operations ───────────────────────────────────────────────────────────────────

struct Op {
    name: String,
    method: String,
    path: String,
    path_params: Vec<String>,
    query_params: Vec<String>,
    has_body: bool,
    request_type: Option<String>,
    response_type: Option<String>,
    tag: String,
    summary: String,
}

const METHODS: &[&str] = &["get", "post", "put", "delete", "patch"];

fn extract_operations(root: &Value) -> Result<Vec<Op>> {
    let paths = root
        .get("paths")
        .and_then(Value::as_object)
        .context("spec has no `paths`")?;
    let mut ops: Vec<Op> = Vec::new();
    let mut used: BTreeMap<String, u32> = BTreeMap::new();

    for (path, item) in paths {
        let common = item.get("parameters").and_then(Value::as_array);
        for method in METHODS {
            let Some(operation) = item.get(method) else {
                continue;
            };
            // Merge path-item-level and operation-level parameters (resolving $refs).
            let mut path_params: Vec<String> = Vec::new();
            let mut query_params: Vec<String> = Vec::new();
            let mut params: Vec<Value> = Vec::new();
            if let Some(c) = common {
                params.extend(c.iter().cloned());
            }
            if let Some(p) = operation.get("parameters").and_then(Value::as_array) {
                params.extend(p.iter().cloned());
            }
            for p in &params {
                let resolved = resolve_param(root, p);
                let (Some(name), Some(loc)) = (
                    resolved.get("name").and_then(Value::as_str),
                    resolved.get("in").and_then(Value::as_str),
                ) else {
                    continue;
                };
                match loc {
                    "path" => path_params.push(name.to_string()),
                    "query" => query_params.push(name.to_string()),
                    _ => {}
                }
            }

            let has_body = operation.get("requestBody").is_some();
            let request_type = operation
                .get("requestBody")
                .and_then(|b| json_schema(b.get("content")))
                .and_then(type_name);
            let response_type = success_response_schema(root, operation)
                .as_ref()
                .and_then(type_name);
            let tag = operation
                .get("tags")
                .and_then(Value::as_array)
                .and_then(|t| t.first())
                .and_then(Value::as_str)
                .unwrap_or("default")
                .to_string();
            let summary = operation
                .get("summary")
                .or_else(|| operation.get("description"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(200)
                .collect::<String>();

            let base = operation
                .get("operationId")
                .and_then(Value::as_str)
                .map(to_snake)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| derive_name(method, path, &path_params));
            let name = unique(&mut used, base);

            ops.push(Op {
                name,
                method: method.to_uppercase(),
                path: path.clone(),
                path_params,
                query_params,
                has_body,
                request_type,
                response_type,
                tag,
                summary,
            });
        }
    }
    ops.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(ops)
}

/// Resolve a parameter that may be a `$ref` into `components/parameters`.
fn resolve_param(root: &Value, param: &Value) -> Value {
    if let Some(r) = param.get("$ref").and_then(Value::as_str) {
        if let Some(resolved) = resolve_pointer(root, r) {
            return resolved.clone();
        }
    }
    param.clone()
}

/// The first 2xx (or `default`) response's JSON schema (owned), if any.
fn success_response_schema(root: &Value, operation: &Value) -> Option<Value> {
    let responses = operation.get("responses").and_then(Value::as_object)?;
    let (_, picked) = responses
        .iter()
        .find(|(k, _)| k.starts_with('2'))
        .or_else(|| responses.iter().find(|(k, _)| *k == "default"))?;
    let response = if let Some(r) = picked.get("$ref").and_then(Value::as_str) {
        resolve_pointer(root, r)?.clone()
    } else {
        picked.clone()
    };
    json_schema(response.get("content")).cloned()
}

/// The `application/json`-ish schema node under a `content` map.
fn json_schema(content: Option<&Value>) -> Option<&Value> {
    let content = content?.as_object()?;
    let (_, media) = content
        .iter()
        .find(|(k, _)| k.contains("json"))
        .or_else(|| content.iter().next())?;
    media.get("schema")
}

/// The component type name a schema points at: a direct `$ref`, or an array's
/// `items` `$ref`. Returns `None` for inline/primitive schemas.
fn type_name(schema: &Value) -> Option<String> {
    if let Some(name) = ref_name(schema) {
        return Some(name);
    }
    if schema.get("type").and_then(Value::as_str) == Some("array") {
        if let Some(items) = schema.get("items") {
            return ref_name(items);
        }
    }
    None
}

// ── types (components → TypeScript) ───────────────────────────────────────────────

struct TypeOut {
    name: String,
    ts: String,
}

fn extract_types(root: &Value) -> Vec<TypeOut> {
    let Some(schemas) = root
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };
    let mut out: Vec<TypeOut> = schemas
        .iter()
        .map(|(name, schema)| TypeOut {
            name: name.clone(),
            ts: component_decl(name, schema),
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Render a top-level component as a TS declaration.
fn component_decl(name: &str, schema: &Value) -> String {
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return format!("export type {name} = {};", enum_union(values));
    }
    let is_object = schema.get("type").and_then(Value::as_str) == Some("object")
        || schema.get("properties").is_some();
    if is_object {
        format!("export interface {name} {}", object_body(schema, true))
    } else {
        format!("export type {name} = {};", ts_type(schema))
    }
}

/// A TS type expression for any schema node.
fn ts_type(schema: &Value) -> String {
    if let Some(name) = ref_name(schema) {
        return name;
    }
    let nullable = schema.get("nullable").and_then(Value::as_bool) == Some(true);
    let mut base = ts_type_inner(schema);
    if nullable && base != "null" {
        base = format!("{base} | null");
    }
    base
}

fn ts_type_inner(schema: &Value) -> String {
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return enum_union(values);
    }
    for key in ["allOf", "oneOf", "anyOf"] {
        if let Some(parts) = schema.get(key).and_then(Value::as_array) {
            let joiner = if key == "allOf" { " & " } else { " | " };
            let rendered: Vec<String> = parts.iter().map(ts_type).collect();
            if !rendered.is_empty() {
                return rendered.join(joiner);
            }
        }
    }
    // `type` may be a string (3.0) or an array incl. "null" (3.1).
    let (type_str, type_nullable) = match schema.get("type") {
        Some(Value::String(s)) => (Some(s.as_str()), false),
        Some(Value::Array(items)) => {
            let nullable = items.iter().any(|v| v.as_str() == Some("null"));
            let first = items
                .iter()
                .find_map(|v| v.as_str().filter(|s| *s != "null"));
            (first, nullable)
        }
        _ => (None, false),
    };
    let mut base = match type_str {
        Some("string") => "string".to_string(),
        Some("integer") | Some("number") => "number".to_string(),
        Some("boolean") => "boolean".to_string(),
        Some("array") => {
            let inner = schema
                .get("items")
                .map(ts_type)
                .unwrap_or_else(|| "unknown".to_string());
            // Parenthesize unions so `(A | B)[]` reads correctly.
            if inner.contains('|') || inner.contains('&') {
                format!("({inner})[]")
            } else {
                format!("{inner}[]")
            }
        }
        Some("object") | None => {
            if schema.get("properties").is_some() {
                object_body(schema, false)
            } else if let Some(ap) = schema.get("additionalProperties") {
                match ap {
                    Value::Bool(_) => "Record<string, unknown>".to_string(),
                    other => format!("Record<string, {}>", ts_type(other)),
                }
            } else {
                "unknown".to_string()
            }
        }
        Some(_) => "unknown".to_string(),
    };
    if type_nullable && base != "null" {
        base = format!("{base} | null");
    }
    base
}

/// Render an object schema as `{ field?: T; ... }`. `multiline` is used for
/// top-level interfaces (one field per line); nested objects stay inline.
fn object_body(schema: &Value, multiline: bool) -> String {
    let props = schema.get("properties").and_then(Value::as_object);
    let required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut fields: Vec<String> = Vec::new();
    if let Some(props) = props {
        for (key, value) in props {
            let opt = if required.contains(&key.as_str()) {
                ""
            } else {
                "?"
            };
            fields.push(format!("{}{opt}: {};", ts_field_key(key), ts_type(value)));
        }
    }
    if let Some(ap) = schema.get("additionalProperties") {
        match ap {
            Value::Object(_) => fields.push(format!("[key: string]: {};", ts_type(ap))),
            Value::Bool(true) => fields.push("[key: string]: unknown;".to_string()),
            _ => {}
        }
    }
    if fields.is_empty() {
        return "{}".to_string();
    }
    if multiline {
        let mut s = String::from("{\n");
        for f in &fields {
            let _ = writeln!(s, "  {f}");
        }
        s.push('}');
        s
    } else {
        format!("{{ {} }}", fields.join(" "))
    }
}

/// Quote a property key if it isn't a plain TS identifier.
fn ts_field_key(key: &str) -> String {
    let ok = !key.is_empty()
        && key.chars().enumerate().all(|(i, c)| {
            c == '_' || c == '$' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
        });
    if ok {
        key.to_string()
    } else {
        format!("\"{}\"", key.replace('"', "\\\""))
    }
}

fn enum_union(values: &[Value]) -> String {
    let parts: Vec<String> = values
        .iter()
        .map(|v| match v {
            Value::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Value::Null => "null".to_string(),
            other => other.to_string(),
        })
        .collect();
    if parts.is_empty() {
        "never".to_string()
    } else {
        parts.join(" | ")
    }
}

// ── shared helpers ────────────────────────────────────────────────────────────────

/// The component name a `$ref: "#/components/schemas/Name"` points at.
fn ref_name(schema: &Value) -> Option<String> {
    let r = schema.get("$ref")?.as_str()?;
    r.rsplit('/').next().map(str::to_string)
}

/// Resolve a `#/a/b/c` JSON pointer within the doc.
fn resolve_pointer<'a>(root: &'a Value, pointer: &str) -> Option<&'a Value> {
    let rest = pointer.strip_prefix("#/")?;
    let mut cur = root;
    for seg in rest.split('/') {
        let seg = seg.replace("~1", "/").replace("~0", "~");
        cur = cur.get(&seg)?;
    }
    Some(cur)
}

/// Derive a deterministic snake_case name from method + path when there is no
/// operationId. e.g. GET /api/v3/series/{id} → `get_series_by_id`.
fn derive_name(method: &str, path: &str, path_params: &[String]) -> String {
    let mut parts: Vec<String> = vec![method.to_string()];
    for seg in path.split('/') {
        if seg.is_empty() || seg == "api" {
            continue;
        }
        if seg.starts_with('{') {
            continue; // path params handled via `by_`
        }
        // Skip version prefixes like v1/v2/v3.
        if seg.len() >= 2 && seg.starts_with('v') && seg[1..].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        parts.push(to_snake(seg));
    }
    if !path_params.is_empty() {
        parts.push("by".to_string());
        for p in path_params {
            parts.push(to_snake(p));
        }
    }
    let joined = parts.join("_");
    let cleaned = sanitize_ident(&joined);
    if cleaned.is_empty() {
        method.to_string()
    } else {
        cleaned
    }
}

/// snake_case an arbitrary identifier (CamelCase, kebab, dotted, etc.).
fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            let prev_lower = i > 0 && chars[i - 1].is_ascii_lowercase();
            let prev_digit = i > 0 && chars[i - 1].is_ascii_digit();
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_ascii_lowercase();
            if (prev_lower || prev_digit || (i > 0 && next_lower)) && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

/// Keep only `[a-z0-9_]`, collapsing runs of `_`.
fn sanitize_ident(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

/// Ensure a name is unique within the service by appending `_2`, `_3`, … .
fn unique(used: &mut BTreeMap<String, u32>, base: String) -> String {
    let base = if base.is_empty() {
        "op".to_string()
    } else {
        base
    };
    let count = used.entry(base.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        base
    } else {
        format!("{base}_{}", *count)
    }
}

// ── Rust emission ─────────────────────────────────────────────────────────────────

fn emit_rust(svc: &str, ops: &[Op], types: &[TypeOut]) -> String {
    let mut s = String::with_capacity(64 * 1024);
    let _ = writeln!(
        s,
        "//! GENERATED by `cargo xtask gen-openapi` from specs/{svc}.openapi.* — DO NOT EDIT.\n\
         //!\n\
         //! {} operations, {} component types.\n\
         #![allow(clippy::all)]\n\
         use crate::openapi::{{OperationSpec, TypeDef}};\n",
        ops.len(),
        types.len()
    );

    let _ = writeln!(s, "pub static OPERATIONS: &[OperationSpec] = &[");
    for op in ops {
        let _ = writeln!(
            s,
            "    OperationSpec {{ name: \"{}\", method: \"{}\", path: \"{}\", path_params: &[{}], query_params: &[{}], has_body: {}, request_type: {}, response_type: {}, tag: \"{}\", summary: \"{}\" }},",
            esc(&op.name),
            esc(&op.method),
            esc(&op.path),
            str_array(&op.path_params),
            str_array(&op.query_params),
            op.has_body,
            opt_str(&op.request_type),
            opt_str(&op.response_type),
            esc(&op.tag),
            esc(&op.summary),
        );
    }
    let _ = writeln!(s, "];\n");

    let _ = writeln!(s, "pub static TYPES: &[TypeDef] = &[");
    for t in types {
        let _ = writeln!(
            s,
            "    TypeDef {{ name: \"{}\", ts: \"{}\" }},",
            esc(&t.name),
            esc(&t.ts),
        );
    }
    let _ = writeln!(s, "];");
    s
}

/// Escape a string for a Rust double-quoted literal.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn str_array(items: &[String]) -> String {
    items
        .iter()
        .map(|i| format!("\"{}\"", esc(i)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn opt_str(value: &Option<String>) -> String {
    match value {
        Some(v) => format!("Some(\"{}\")", esc(v)),
        None => "None".to_string(),
    }
}
