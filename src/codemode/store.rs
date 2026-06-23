//! Snippet store: persisted, named, reusable Code Mode scripts.
//!
//! Each snippet is two files under `<data_dir>/codemode/snippets/`: `<name>.js`
//! (the source) and `<name>.json` (metadata). All filesystem + name-validation
//! logic lives here; the actions/dispatch (`snippet_list/save/run/delete`) are
//! thin wrappers in the app layer.
//!
//! Security (S7): the snippet name is the ONLY caller-controlled filename
//! component. [`validate_snippet_name`] is the sole, sufficient control — it
//! allowlists `[A-Za-z0-9._-]`, forbids a leading dot, `..`, and any path
//! separator, so `dir.join("<name>.js")` is always a direct child of the store
//! dir (no traversal possible). No symlink/canonicalize dance is claimed.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{CODEMODE_MAX_SNIPPET_NAME_LEN, CODEMODE_SNIPPETS_SUBDIR};

/// Metadata persisted alongside a snippet's source (`<name>.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnippetMeta {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Source size at save time. Advisory only — `list` re-derives it from the
    /// `.js` file for hand-dropped snippets, so it may be stale for out-of-band
    /// edits; never load-bearing for security or correctness.
    #[serde(default)]
    pub bytes: u64,
}

/// Validate a caller-supplied snippet name. Allowlist-only; fail-closed.
pub fn validate_snippet_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("snippet name must not be empty".to_string());
    }
    if name.len() > CODEMODE_MAX_SNIPPET_NAME_LEN {
        return Err(format!(
            "snippet name is too long (max {CODEMODE_MAX_SNIPPET_NAME_LEN})"
        ));
    }
    if name.starts_with('.') {
        return Err("snippet name must not start with a dot".to_string());
    }
    if name.contains("..") {
        return Err("snippet name must not contain `..`".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err("snippet name may only contain letters, digits, `.`, `_`, `-`".to_string());
    }
    Ok(())
}

/// The snippets directory under the data dir.
pub fn snippets_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(CODEMODE_SNIPPETS_SUBDIR)
}

fn source_path(data_dir: &Path, name: &str) -> PathBuf {
    snippets_dir(data_dir).join(format!("{name}.js"))
}

fn meta_path(data_dir: &Path, name: &str) -> PathBuf {
    snippets_dir(data_dir).join(format!("{name}.json"))
}

/// Save (create or overwrite) a snippet's source + metadata. Returns the metadata.
pub fn save(
    data_dir: &Path,
    name: &str,
    code: &str,
    description: Option<&str>,
) -> Result<SnippetMeta, String> {
    validate_snippet_name(name)?;
    let dir = snippets_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create snippets dir: {e}"))?;
    let meta = SnippetMeta {
        name: name.to_string(),
        description: description.map(str::to_string),
        bytes: code.len() as u64,
    };
    std::fs::write(source_path(data_dir, name), code.as_bytes())
        .map_err(|e| format!("could not write snippet source: {e}"))?;
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("could not encode metadata: {e}"))?;
    std::fs::write(meta_path(data_dir, name), meta_json.as_bytes())
        .map_err(|e| format!("could not write snippet metadata: {e}"))?;
    Ok(meta)
}

/// List saved snippets (metadata only), sorted by name. Missing/corrupt metadata
/// is synthesized from the `.js` file so a hand-dropped snippet still lists.
pub fn list(data_dir: &Path) -> Result<Vec<SnippetMeta>, String> {
    let dir = snippets_dir(data_dir);
    let mut out: Vec<SnippetMeta> = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(format!("could not read snippets dir: {e}")),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("js") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        // Allowlist-gate on read too: a manually-dropped file with an invalid name
        // is skipped (it could never be loaded by name anyway).
        if validate_snippet_name(name).is_err() {
            continue;
        }
        let meta = std::fs::read_to_string(meta_path(data_dir, name))
            .ok()
            .and_then(|s| serde_json::from_str::<SnippetMeta>(&s).ok())
            .unwrap_or_else(|| SnippetMeta {
                name: name.to_string(),
                description: None,
                bytes: std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
            });
        out.push(meta);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Load a snippet's source by name.
pub fn load_source(data_dir: &Path, name: &str) -> Result<String, String> {
    validate_snippet_name(name)?;
    std::fs::read_to_string(source_path(data_dir, name))
        .map_err(|_| format!("no snippet named `{name}`"))
}

/// Delete a snippet's source + metadata. Returns true if the source existed.
pub fn delete(data_dir: &Path, name: &str) -> Result<bool, String> {
    validate_snippet_name(name)?;
    let src = source_path(data_dir, name);
    let existed = src.exists();
    if existed {
        std::fs::remove_file(&src).map_err(|e| format!("could not delete snippet: {e}"))?;
    }
    // Best-effort metadata cleanup.
    let _ = std::fs::remove_file(meta_path(data_dir, name));
    Ok(existed)
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
