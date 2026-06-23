//! Runtime tests for the generated-operation registry + scalar rendering.

use super::*;
use serde_json::json;

#[test]
fn scalar_rendering_covers_string_number_bool_only() {
    assert_eq!(scalar_to_string(&json!("x")).as_deref(), Some("x"));
    assert_eq!(scalar_to_string(&json!(42)).as_deref(), Some("42"));
    assert_eq!(scalar_to_string(&json!(true)).as_deref(), Some("true"));
    // Non-scalars are rejected so we never send `[object Object]`.
    assert!(scalar_to_string(&json!({"a": 1})).is_none());
    assert!(scalar_to_string(&json!([1, 2])).is_none());
    assert!(scalar_to_string(&json!(null)).is_none());
}

#[test]
fn find_operation_resolves_known_and_rejects_unknown() {
    // Sonarr is generated; a known op resolves and an unknown one does not.
    assert!(find_operation(ServiceKind::Sonarr, "get_system_status").is_some());
    assert!(find_operation(ServiceKind::Sonarr, "nope_not_real").is_none());
    // A doc-based kind has no generated operations.
    assert!(operations_for_kind(ServiceKind::Tautulli).is_empty());
    assert!(!is_generated(ServiceKind::Tautulli));
}
