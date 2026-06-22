//! Preamble generation tests.

use super::*;

#[test]
fn preamble_defines_calltool_and_runner() {
    let pre = build_preamble();
    assert!(pre.contains("globalThis.callTool ="));
    assert!(pre.contains("__rustarrEmitToolCall"));
    assert!(pre.contains("globalThis.__rustarrRun ="));
    assert!(pre.contains("globalThis.console ="));
    assert!(pre.contains("globalThis.tools = {};"));
}

#[test]
fn preamble_includes_generic_and_curated_action_helpers() {
    let pre = build_preamble();
    // Generic infra actions.
    assert!(pre.contains(r#"tools["integrations"]"#));
    assert!(pre.contains(r#"tools["service_status"]"#));
    assert!(pre.contains(r#"tools["api_get"]"#));
    // A representative curated command (registry-derived).
    assert!(pre.contains(r#"tools["list"]"#));
}

#[test]
fn preamble_excludes_codemode_and_help_helpers() {
    let pre = build_preamble();
    assert!(!pre.contains(r#"tools["codemode"]"#));
    assert!(!pre.contains(r#"tools["help"]"#));
}

#[test]
fn reserved_word_actions_use_bracket_notation() {
    // `delete` is a reserved word; bracket-notation keys keep it valid JS.
    let pre = build_preamble();
    if proxy_action_names().contains(&"delete") {
        assert!(pre.contains(r#"tools["delete"]"#));
    }
}
