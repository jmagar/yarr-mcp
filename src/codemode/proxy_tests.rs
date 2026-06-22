//! Preamble generation tests.

use super::*;

#[test]
fn preamble_defines_calltool_and_runner() {
    let pre = build_preamble(&[]);
    assert!(pre.contains("globalThis.callTool ="));
    assert!(pre.contains("__rustarrEmitToolCall"));
    assert!(pre.contains("globalThis.__rustarrRun ="));
    assert!(pre.contains("globalThis.console ="));
    assert!(pre.contains("globalThis.tools = {};"));
}

#[test]
fn preamble_includes_generic_and_curated_action_helpers() {
    let pre = build_preamble(&[]);
    // Generic infra actions.
    assert!(pre.contains(r#"tools["integrations"]"#));
    assert!(pre.contains(r#"tools["service_status"]"#));
    assert!(pre.contains(r#"tools["api_get"]"#));
    // A representative curated command (registry-derived).
    assert!(pre.contains(r#"tools["list"]"#));
}

#[test]
fn preamble_excludes_codemode_and_help_helpers() {
    let pre = build_preamble(&[]);
    assert!(!pre.contains(r#"tools["codemode"]"#));
    assert!(!pre.contains(r#"tools["help"]"#));
}

#[test]
fn api_namespace_generated_per_configured_service() {
    let pre = build_preamble(&["sonarr".to_string(), "radarr".to_string()]);
    assert!(pre.contains("globalThis.api = {};"));
    assert!(pre.contains(r#"globalThis.api["sonarr"]"#));
    assert!(pre.contains(r#"globalThis.api["radarr"]"#));
    // get/post/put/delete sugar over the api_* passthrough actions.
    assert!(pre.contains(r#"callTool("api_get", { service: "sonarr""#));
    assert!(pre.contains(r#"callTool("api_delete", { service: "radarr""#));
}

#[test]
fn api_namespace_empty_when_no_services() {
    let pre = build_preamble(&[]);
    assert!(pre.contains("globalThis.api = {};"));
    assert!(!pre.contains("globalThis.api[\""));
}

#[test]
fn preamble_injects_discovery_catalog_and_helpers() {
    let pre = build_preamble(&[]);
    assert!(pre.contains("globalThis.__codemodeCatalog = ["));
    assert!(pre.contains("globalThis.codemode.search ="));
    assert!(pre.contains("globalThis.codemode.describe ="));
    // The catalog embeds a destructive flag for at least one delete.
    assert!(pre.contains("\"destructive\":true"));
    // writeArtifact must NOT be a registry-derived tools.* helper.
    assert!(!pre.contains(r#"tools["writeArtifact"]"#));
}

#[test]
fn snippet_verbs_are_not_in_tools_namespace() {
    let pre = build_preamble(&[]);
    // Snippet store verbs are reachable only via codemode.run/snippets — never as
    // incidental in-script tools.* side effects (no writing/deleting files).
    for action in [
        "snippet_save",
        "snippet_delete",
        "snippet_run",
        "snippet_list",
    ] {
        assert!(
            !pre.contains(&format!(r#"tools["{action}"]"#)),
            "{action} must not be a tools.* helper"
        );
    }
    // But the explicit discovery/run helpers ARE present.
    assert!(pre.contains("globalThis.codemode.run ="));
    assert!(pre.contains("globalThis.codemode.snippets ="));
    // And `input` is wired (defaults to null for non-snippet runs).
    assert!(pre.contains("globalThis.input ="));
}

#[test]
fn reserved_word_actions_use_bracket_notation() {
    // `delete` is a reserved word; bracket-notation keys keep it valid JS.
    let pre = build_preamble(&[]);
    if proxy_action_names().contains(&"delete") {
        assert!(pre.contains(r#"tools["delete"]"#));
    }
}
