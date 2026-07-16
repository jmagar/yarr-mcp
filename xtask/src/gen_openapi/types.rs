//! Component-schema to TypeScript catalog generation.

use std::fmt::Write as _;

use serde_json::Value;

use super::TypeOut;

pub(super) fn extract_types(root: &Value) -> Vec<TypeOut> {
    let Some(schemas) = root
        .get("components")
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
    else {
        if root.get("components").is_some() {
            eprintln!(
                "  WARNING: spec has `components` but no object `components.schemas` -- emitting 0 types"
            );
        }
        return Vec::new();
    };
    let mut output = schemas
        .iter()
        .map(|(name, schema)| TypeOut {
            name: name.clone(),
            ts: component_decl(name, schema),
        })
        .collect::<Vec<_>>();
    output.sort_by(|left, right| left.name.cmp(&right.name));
    output
}

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
            let rendered = parts.iter().map(ts_type).collect::<Vec<_>>();
            if !rendered.is_empty() {
                return rendered.join(if key == "allOf" { " & " } else { " | " });
            }
        }
    }
    let (kind, type_nullable) = match schema.get("type") {
        Some(Value::String(kind)) => (Some(kind.as_str()), false),
        Some(Value::Array(items)) => (
            items
                .iter()
                .find_map(|value| value.as_str().filter(|kind| *kind != "null")),
            items.iter().any(|value| value.as_str() == Some("null")),
        ),
        _ => (None, false),
    };
    let mut base = match kind {
        Some("string") => "string".to_string(),
        Some("integer" | "number") => "number".to_string(),
        Some("boolean") => "boolean".to_string(),
        Some("array") => {
            let inner = schema
                .get("items")
                .map(ts_type)
                .unwrap_or_else(|| "unknown".to_string());
            if inner.contains('|') || inner.contains('&') {
                format!("({inner})[]")
            } else {
                format!("{inner}[]")
            }
        }
        Some("object") | None => {
            if schema.get("properties").is_some() {
                object_body(schema, false)
            } else if let Some(additional) = schema.get("additionalProperties") {
                match additional {
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

fn object_body(schema: &Value, multiline: bool) -> String {
    let properties = schema.get("properties").and_then(Value::as_object);
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut fields = Vec::new();
    if let Some(properties) = properties {
        for (key, value) in properties {
            let optional = if required.contains(&key.as_str()) {
                ""
            } else {
                "?"
            };
            fields.push(format!(
                "{}{optional}: {};",
                ts_field_key(key),
                ts_type(value)
            ));
        }
    }
    if let Some(additional) = schema.get("additionalProperties") {
        match additional {
            Value::Object(_) => fields.push(format!("[key: string]: {};", ts_type(additional))),
            Value::Bool(true) => fields.push("[key: string]: unknown;".to_string()),
            _ => {}
        }
    }
    if fields.is_empty() {
        return "{}".to_string();
    }
    if multiline {
        let mut output = String::from("{\n");
        for field in fields {
            let _ = writeln!(output, "  {field}");
        }
        output.push('}');
        output
    } else {
        format!("{{ {} }}", fields.join(" "))
    }
}

fn ts_field_key(key: &str) -> String {
    let valid = !key.is_empty()
        && key.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character == '$'
                || character.is_ascii_alphabetic()
                || (index > 0 && character.is_ascii_digit())
        });
    if valid {
        key.to_string()
    } else {
        format!("\"{}\"", key.replace('"', "\\\""))
    }
}

fn enum_union(values: &[Value]) -> String {
    let parts = values
        .iter()
        .map(|value| match value {
            Value::String(value) => format!("\"{}\"", value.replace('"', "\\\"")),
            Value::Null => "null".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "never".to_string()
    } else {
        parts.join(" | ")
    }
}

fn ref_name(schema: &Value) -> Option<String> {
    schema
        .get("$ref")?
        .as_str()?
        .rsplit('/')
        .next()
        .map(str::to_string)
}
