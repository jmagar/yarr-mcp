//! Tests for the destructive-delete elicitation gate.
//!
//! The peer round-trip is not exercised here (it needs a live client); instead we
//! cover the pure decision surface: `preauthorized`, the prompt text, the
//! `normalize` accept/refuse mapping over the constructible `Ok` arms, and the
//! full `classify` decision (including `Unsupported → Abstain`, the fail-open
//! fallback path). The `Err` arms of `normalize` use rmcp's `#[non_exhaustive]`
//! `ElicitationError`, which a downstream crate cannot construct, so they are
//! covered by review + the trivial match rather than a unit test.

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

// ── normalize: rmcp Ok result → ElicitOutcome (Err arms not constructible) ───────

#[test]
fn normalize_accept_with_confirm_true_is_confirmed() {
    assert_eq!(
        normalize(Ok(Some(DeleteConfirmation { confirm: true }))),
        ElicitOutcome::Confirmed
    );
}

#[test]
fn normalize_accept_with_confirm_false_refuses() {
    assert_eq!(
        normalize(Ok(Some(DeleteConfirmation { confirm: false }))),
        ElicitOutcome::Refused
    );
}

#[test]
fn normalize_empty_content_refuses() {
    assert_eq!(normalize(Ok(None)), ElicitOutcome::Refused);
}

// ── classify: ElicitOutcome → DeleteGate (the gate decision, fully covered) ──────

#[test]
fn classify_confirmed_proceeds() {
    assert_eq!(classify(ElicitOutcome::Confirmed), DeleteGate::Proceed);
}

#[test]
fn classify_refused_declines() {
    assert_eq!(classify(ElicitOutcome::Refused), DeleteGate::Declined);
}

#[test]
fn classify_unsupported_abstains() {
    // The fail-open-to-app-layer path: a client that cannot elicit yields Abstain,
    // and dispatch then relies on the app-layer confirm gate.
    assert_eq!(classify(ElicitOutcome::Unsupported), DeleteGate::Abstain);
}
