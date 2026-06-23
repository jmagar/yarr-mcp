use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
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

    pub fn fail(&mut self, name: impl Into<String>, detail: impl Into<String>) {
        let check = Check {
            name: name.into(),
            passed: false,
            detail: detail.into(),
        };
        println!("FAIL {}: {}", check.name, check.detail);
        self.checks.push(check);
    }

    pub fn is_success(&self) -> bool {
        self.checks.iter().all(|check| check.passed)
    }

    pub fn len(&self) -> usize {
        self.checks.len()
    }

    pub fn passed_count(&self) -> usize {
        self.checks.iter().filter(|check| check.passed).count()
    }

    pub fn contains_check(&self, name: &str) -> bool {
        self.checks.iter().any(|check| check.name == name)
    }

    pub fn check_passed(&self, name: &str) -> Option<bool> {
        self.checks
            .iter()
            .find(|check| check.name == name)
            .map(|check| check.passed)
    }

    pub fn write_json(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(&self.checks)?;
        std::fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn read_json(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let checks = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(Self { checks })
    }
}
