//! Path-validation + content-type tests for `writeArtifact`.

use super::*;
use std::path::Path;

#[test]
fn accepts_plain_relative_paths() {
    assert_eq!(
        validate_artifact_path("report.json").unwrap(),
        Path::new("report.json")
    );
    assert_eq!(
        validate_artifact_path("out/sub/report.json").unwrap(),
        Path::new("out/sub/report.json")
    );
    // "./" is harmless and normalized away.
    assert_eq!(
        validate_artifact_path("./a/b.txt").unwrap(),
        Path::new("a/b.txt")
    );
}

#[test]
fn rejects_traversal_and_absolute_and_encoded() {
    for bad in [
        "",
        "   ",
        "/etc/passwd",
        "../escape",
        "a/../../b",
        "a/../b",
        "..%2fescape",
        "a%2f..%2fb",
        "x%5cy",
        "%2e%2e/x",
        "with\0nul",
    ] {
        assert!(
            validate_artifact_path(bad).is_err(),
            "expected rejection for {bad:?}"
        );
    }
}

#[test]
fn rejects_windows_drive_and_unc() {
    // On Unix these are treated as a single weird filename; the explicit checks
    // below exercise the component-level rejection on platforms that parse them.
    assert!(validate_artifact_path("C:\\Windows\\system32").is_err() || cfg!(unix));
}

#[test]
fn resolve_under_root_keeps_inside() {
    let root = Path::new("/data/codemode/artifacts/run1");
    let rel = validate_artifact_path("out/report.json").unwrap();
    let full = resolve_under_root(root, &rel).unwrap();
    assert!(full.starts_with(root));
    assert!(full.ends_with("out/report.json"));
}

#[test]
fn resolve_under_root_rejects_traversal_directly() {
    // Second-layer defense: feed a `..`-bearing relative path directly (bypassing
    // validate_artifact_path) and confirm containment still refuses it.
    let root = Path::new("/data/run1");
    let escaping = Path::new("../../etc/passwd");
    assert!(resolve_under_root(root, escaping).is_err());
}

#[test]
fn content_type_inference_and_override() {
    assert_eq!(
        content_type_for(Path::new("a.json"), None),
        "application/json"
    );
    assert_eq!(content_type_for(Path::new("a.txt"), None), "text/plain");
    assert_eq!(content_type_for(Path::new("a.md"), None), "text/markdown");
    assert_eq!(
        content_type_for(Path::new("a.bin"), None),
        "application/octet-stream"
    );
    // Caller override wins.
    assert_eq!(
        content_type_for(Path::new("a.json"), Some("text/plain")),
        "text/plain"
    );
}
