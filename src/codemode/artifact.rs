//! Pure helpers for Code Mode `writeArtifact`: fail-closed path validation,
//! content-type inference, and containment resolution. No I/O, no tokio — the
//! actual write lives in the app layer ([`crate::app`]).
//!
//! Security (S7): a script-supplied artifact path must be a plain relative path
//! under the per-run dir. We reject absolute paths, `..` (raw and percent-encoded),
//! encoded separators, NUL, and Windows drive/UNC prefixes BEFORE any filesystem
//! access, then [`resolve_under_root`] re-asserts lexical containment as defense
//! in depth.

use std::path::{Component, Path, PathBuf};

/// Validate a script-supplied artifact path, returning the cleaned relative path.
/// Fail-closed: anything that could escape the run dir is rejected.
pub fn validate_artifact_path(raw: &str) -> Result<PathBuf, String> {
    if raw.trim().is_empty() {
        return Err("writeArtifact path must be non-empty".to_string());
    }
    if raw.len() > 1024 {
        return Err("writeArtifact path is too long".to_string());
    }
    if raw.contains('\0') {
        return Err("writeArtifact path contains a NUL byte".to_string());
    }
    // Reject percent-encoded separators / traversal before decoding tricks them in.
    let lowered = raw.to_ascii_lowercase();
    if lowered.contains("%2f") || lowered.contains("%5c") || lowered.contains("%2e%2e") {
        return Err("writeArtifact path contains an encoded separator or traversal".to_string());
    }
    let path = Path::new(raw);
    if path.is_absolute() {
        return Err("writeArtifact path must be relative".to_string());
    }
    // Windows drive (C:) / UNC (\\) prefixes, and any non-normal component.
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {} // "./" is harmless
            Component::ParentDir => {
                return Err("writeArtifact path must not contain `..`".to_string());
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("writeArtifact path must be relative (no root/drive)".to_string());
            }
        }
    }
    // Re-collect as normal components only (drops any "./").
    let cleaned: PathBuf = path
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .collect();
    if cleaned.as_os_str().is_empty() {
        return Err("writeArtifact path resolves to nothing".to_string());
    }
    Ok(cleaned)
}

/// Join a validated relative path under `root` and assert the result stays under
/// `root` (lexical containment — belt-and-suspenders behind [`validate_artifact_path`]).
pub fn resolve_under_root(root: &Path, rel: &Path) -> Result<PathBuf, String> {
    // `rel` is already validated to contain only Normal components, so this is a
    // pure-lexical containment check (no canonicalize needed and none of the inputs
    // can introduce `..`); a defensive re-scan keeps it true if a caller skips
    // validation.
    for component in rel.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err("artifact path escaped the run directory".to_string());
        }
    }
    let joined = root.join(rel);
    if !joined.starts_with(root) {
        return Err("artifact path escaped the run directory".to_string());
    }
    Ok(joined)
}

/// Infer a content type from the path extension, honouring a caller override.
pub fn content_type_for(path: &Path, override_type: Option<&str>) -> String {
    if let Some(ct) = override_type.filter(|s| !s.trim().is_empty()) {
        return ct.to_string();
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "json" => "application/json",
        "txt" | "log" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "xml" => "application/xml",
        "yaml" | "yml" => "application/yaml",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;
