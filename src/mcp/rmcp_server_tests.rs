use serde_json::json;

use crate::{
    actions::{READ_SCOPE, WRITE_SCOPE, required_scope_for_action},
    token_limit::MAX_RESPONSE_BYTES,
};

use super::{
    internal_tool_error_message, reject_unknown_action_before_scope, scope_satisfied,
    tool_result_from_json,
};

fn scopes(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| x.to_string()).collect()
}

#[test]
fn read_scope_satisfies_read_requirement() {
    assert!(scope_satisfied(&scopes(&[READ_SCOPE]), READ_SCOPE));
}

#[test]
fn write_scope_satisfies_read_requirement() {
    assert!(
        scope_satisfied(&scopes(&[WRITE_SCOPE]), READ_SCOPE),
        "write scope should satisfy read requirement (write includes read)"
    );
}

#[test]
fn empty_scopes_denied() {
    assert!(!scope_satisfied(&[], READ_SCOPE));
}

#[test]
fn unrelated_scope_denied() {
    assert!(!scope_satisfied(&scopes(&["other:scope"]), READ_SCOPE));
}

#[test]
fn read_scope_does_not_satisfy_write() {
    assert!(
        !scope_satisfied(&scopes(&[READ_SCOPE]), WRITE_SCOPE),
        "read scope must not satisfy write requirement"
    );
}

#[test]
fn api_get_requires_write_scope() {
    assert_eq!(required_scope_for_action("api_get"), Some(WRITE_SCOPE));
}

#[test]
fn api_post_requires_write_scope() {
    assert_eq!(required_scope_for_action("api_post"), Some(WRITE_SCOPE));
}

#[test]
fn help_requires_no_scope() {
    assert_eq!(required_scope_for_action("help"), None);
}

#[test]
fn unknown_action_gets_deny_scope() {
    use crate::actions::DENY_SCOPE;
    assert_eq!(
        required_scope_for_action("nonexistent_action"),
        Some(DENY_SCOPE)
    );
}

#[test]
fn unknown_action_is_rejected_as_validation_before_scope() {
    let error = reject_unknown_action_before_scope("nonexistent_action")
        .expect_err("unknown action should be invalid params");
    assert!(error.message.contains("unknown rustarr action"));
}

#[test]
fn internal_tool_errors_include_stable_kind() {
    let message = internal_tool_error_message("status");
    assert!(message.contains("kind=execution_error"));
    assert!(message.contains("action='status'"));
}

#[test]
fn tool_result_from_json_applies_response_cap() {
    let result = tool_result_from_json(json!({
        "payload": "x".repeat(MAX_RESPONSE_BYTES + 1)
    }))
    .expect("tool result should serialize");
    let text = result.content[0]
        .raw
        .as_text()
        .expect("tool result should contain text")
        .text
        .as_str();
    // The over-cap result is now a parseable JSON envelope with a truncation
    // marker (AN-6), not a notice appended after the JSON.
    let parsed: serde_json::Value =
        serde_json::from_str(text).expect("truncated tool result is valid JSON");
    assert_eq!(parsed["truncated"], true);
    assert!(text.len() <= MAX_RESPONSE_BYTES);
}
