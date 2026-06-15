//! Tests for the action-layer facade: verify the re-exports resolve and expose
//! the stable `crate::actions::*` surface.

use super::*;

#[test]
fn action_names_is_non_empty() {
    assert!(!action_names().is_empty());
}

#[test]
fn scope_constants_are_distinct() {
    assert_ne!(READ_SCOPE, WRITE_SCOPE);
    assert!(!READ_SCOPE.is_empty());
    assert!(!WRITE_SCOPE.is_empty());
    assert!(!DENY_SCOPE.is_empty());
}

#[test]
fn rest_help_returns_a_value() {
    let help = rest_help();
    assert!(help.is_object() || help.is_array());
}

#[test]
fn is_known_action_recognizes_api_get() {
    assert!(is_known_action("api_get"));
    assert!(!is_known_action("definitely_not_an_action"));
}
