//! Tests for the destructive-delete elicitation gate.
//!
//! The peer round-trip is not exercised here (it needs a live client); instead we
//! cover the pure decision surface: `preauthorized`, the prompt text, and the
//! `interpret` accept/decline mapping over the `Ok` arms.

use super::*;
use serde_json::json;

#[test]
fn preauthorized_true_only_for_explicit_confirm_true() {
    assert!(preauthorized(&json!({ "confirm": true })));
    assert!(!preauthorized(&json!({ "confirm": false })));
    assert!(!preauthorized(&json!({})));
    // A non-bool confirm is not authorization.
    assert!(!preauthorized(&json!({ "confirm": "true" })));
}

#[test]
fn confirm_message_names_action_and_service() {
    let msg = confirm_message("delete", "sonarr");
    assert!(msg.contains("delete"));
    assert!(msg.contains("sonarr"));
    assert!(msg.contains("cannot be undone"));
}

#[test]
fn interpret_accept_with_confirm_true_proceeds() {
    assert_eq!(
        interpret(Ok(Some(DeleteConfirmation { confirm: true }))),
        DeleteGate::Proceed
    );
}

#[test]
fn interpret_accept_with_confirm_false_declines() {
    assert_eq!(
        interpret(Ok(Some(DeleteConfirmation { confirm: false }))),
        DeleteGate::Declined
    );
}

#[test]
fn interpret_empty_content_declines() {
    assert_eq!(interpret(Ok(None)), DeleteGate::Declined);
}
