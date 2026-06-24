use anyhow::{Context, Result};
use std::{fs, path::Path};

pub(super) fn read_file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

pub(super) fn display_path(path: &Path) -> String {
    path.strip_prefix(".")
        .unwrap_or(path)
        .to_string_lossy()
        .trim_start_matches('/')
        .to_string()
}

pub(super) fn contains_top_level_json_key(text: &str, key: &str) -> bool {
    // Avoids serde_json in xtask. Handles both formatted JSON (key on its own
    // line) and compact/single-line JSON (key after `{`).
    let pattern = format!("\"{key}\"");
    text.lines().any(|line| {
        let content = line.trim_start().trim_start_matches('{').trim_start();
        content.starts_with(&pattern) && content[pattern.len()..].trim_start().starts_with(':')
    })
}

pub(super) fn size_limit(path: &Path) -> Option<usize> {
    // Generated OpenAPI tables (`cargo xtask gen-openapi`) are pure data — one
    // `OperationSpec`/`TypeDef` literal per upstream operation/component — not
    // hand-maintained modules, so the per-file size targets do not apply.
    if path.to_string_lossy().contains("src/openapi/generated/") {
        return None;
    }
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => Some(350),
        Some("ts" | "tsx") => Some(300),
        _ => None,
    }
}

pub(super) fn is_test_file(path: &Path) -> bool {
    let path = path.to_string_lossy();
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with("/tests.rs")
        || path.ends_with(".test.ts")
        || path.ends_with(".test.tsx")
        || path.ends_with(".spec.ts")
        || path.ends_with(".spec.tsx")
        || path.contains("/__tests__/")
}

pub(super) fn effective_loc(path: &Path) -> Result<usize> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(effective_loc_from_text(
        &text,
        path.extension().and_then(|ext| ext.to_str()) == Some("rs"),
    ))
}

fn effective_loc_from_text(text: &str, strip_tests: bool) -> usize {
    let text = if strip_tests {
        strip_inline_test_module(text)
    } else {
        text
    };
    let mut count = 0usize;
    let mut in_block = false;

    for raw in text.lines() {
        let mut line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if in_block {
            if let Some((_, after)) = line.split_once("*/") {
                line = after.trim();
                in_block = false;
                if line.is_empty() {
                    continue;
                }
            } else {
                continue;
            }
        }
        if line.starts_with("//") {
            continue;
        }
        if line.starts_with("/*") {
            if let Some((_, after)) = line.split_once("*/") {
                line = after.trim();
                if line.is_empty() {
                    continue;
                }
            } else {
                in_block = true;
                continue;
            }
        }
        count += 1;
    }
    count
}

fn strip_inline_test_module(text: &str) -> &str {
    let lines = text.lines().collect::<Vec<_>>();
    for index in 0..lines.len().saturating_sub(1) {
        if lines[index].trim() != "#[cfg(test)]" {
            continue;
        }
        let next = lines[index + 1].trim_start();
        if next.starts_with("mod ") || next.starts_with("pub mod ") {
            let byte_index = lines[..index].iter().map(|line| line.len() + 1).sum();
            return &text[..byte_index];
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_loc_ignores_comments_blanks_and_trailing_tests() {
        let text = r#"
// comment
pub fn production() {}

/* block
   comment */
pub fn more() {}

#[cfg(test)]
mod tests {
    fn test_only() {}
}
"#;
        assert_eq!(effective_loc_from_text(text, true), 2);
    }

    #[test]
    fn effective_loc_counts_code_after_inline_block_comment() {
        let text = "/* license */ pub fn one() {}\nlet two = 2;";
        assert_eq!(effective_loc_from_text(text, false), 2);
    }

    #[test]
    fn top_level_json_key_detects_manifest_version_field() {
        assert!(contains_top_level_json_key(
            "{\n  \"version\": \"1\"\n}",
            "version"
        ));
        assert!(!contains_top_level_json_key(
            "{\n  \"not_version\": true\n}",
            "version"
        ));
    }

    #[test]
    fn top_level_json_key_handles_compact_json() {
        // Single-line / compact JSON: key appears after `{`
        assert!(contains_top_level_json_key(
            "{ \"version\": \"1\" }",
            "version"
        ));
        assert!(contains_top_level_json_key(
            "{\"version\":\"1\"}",
            "version"
        ));
        // Must not match a different key
        assert!(!contains_top_level_json_key(
            "{ \"name\": \"foo\" }",
            "version"
        ));
    }
}
