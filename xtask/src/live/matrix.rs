use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Matrix {
    pub services: Vec<ServiceCase>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceCase {
    pub name: String,
    pub kind: String,
    pub status: Expectation,
    pub get: Vec<GetCase>,
    pub post_blocked: PostCase,
    pub post_expected_error: PostExpectedError,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetCase {
    pub path: String,
    #[serde(flatten)]
    pub expectation: Expectation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostCase {
    pub path: String,
    pub body: Value,
    pub error_contains: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostExpectedError {
    pub path: String,
    pub body: Value,
    pub error_contains_any: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Expectation {
    pub json_path: Option<String>,
    pub equals: Option<Value>,
    pub equals_any: Option<Vec<Value>>,
    #[serde(rename = "type")]
    pub value_type: Option<String>,
    pub contains: Option<String>,
    pub xml_root: Option<String>,
}

pub fn load(path: &Path) -> Result<Matrix> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read live matrix {}", path.display()))?;
    serde_json::from_str(&raw).context("failed to parse live service matrix")
}
