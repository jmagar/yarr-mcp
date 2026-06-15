use anyhow::{Result, bail};
use serde_json::Value;

use super::matrix::Expectation;

pub fn assert_value(value: &Value, expectation: &Expectation) -> Result<()> {
    if expectation.xml_root.is_some() {
        if let Some(text) = value.as_str() {
            return assert_text(text, expectation);
        }
        let text = value.to_string();
        return assert_text(&text, expectation);
    }

    let node = if let Some(path) = &expectation.json_path {
        json_path(value, path)?
    } else {
        value
    };

    if let Some(expected_type) = &expectation.value_type {
        let ok = matches!(
            (expected_type.as_str(), node),
            ("array", Value::Array(_))
                | ("object", Value::Object(_))
                | ("string", Value::String(_))
                | ("number", Value::Number(_))
                | ("boolean", Value::Bool(_))
        );
        if !ok {
            bail!("expected type {expected_type}, got {node}");
        }
    }
    if let Some(expected) = &expectation.equals
        && node != expected
    {
        bail!("expected {expected}, got {node}");
    }
    if let Some(expected_values) = &expectation.equals_any
        && !expected_values.iter().any(|expected| expected == node)
    {
        bail!("expected one of {expected_values:?}, got {node}");
    }
    if let Some(needle) = &expectation.contains {
        let haystack = node.as_str().unwrap_or("");
        if !haystack.contains(needle) {
            bail!("expected {haystack:?} to contain {needle:?}");
        }
    }
    Ok(())
}

pub fn assert_text(text: &str, expectation: &Expectation) -> Result<()> {
    if let Some(root_name) = &expectation.xml_root {
        let doc = roxmltree::Document::parse(text)?;
        let root = doc.root_element().tag_name().name().to_string();
        if &root != root_name {
            bail!("expected XML root {root_name}, got {root}");
        }
        return Ok(());
    }
    let value: Value = serde_json::from_str(text)?;
    assert_value(&value, expectation)
}

pub fn assert_expected_error(text: &str, tokens: &[String]) -> Result<()> {
    if tokens.iter().any(|token| text.contains(token)) {
        return Ok(());
    }
    bail!("expected error to contain one of {tokens:?}; got {text}");
}

fn json_path<'a>(value: &'a Value, path: &str) -> Result<&'a Value> {
    let mut node = value;
    for part in path.split('.') {
        node = node
            .get(part)
            .ok_or_else(|| anyhow::anyhow!("missing JSON path {path} at {part}"))?;
    }
    Ok(node)
}
