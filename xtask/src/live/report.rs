use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Default)]
pub struct Report {
    checks: Vec<Check>,
}

impl Report {
    pub fn pass(&mut self, name: impl Into<String>, detail: impl Into<String>) {
        let check = Check {
            name: name.into(),
            passed: true,
            detail: detail.into(),
        };
        println!("PASS {}: {}", check.name, check.detail);
        self.checks.push(check);
    }

    pub fn is_success(&self) -> bool {
        self.checks.iter().all(|check| check.passed)
    }

    pub fn len(&self) -> usize {
        self.checks.len()
    }

    pub fn contains_check(&self, name: &str) -> bool {
        self.checks.iter().any(|check| check.name == name)
    }

    pub fn write_json(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(&self.checks)?;
        std::fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }
}
