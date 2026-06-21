use super::*;
use crate::actions::RustarrAction;
use serde_json::json;

#[test]
fn string_arg_trims_and_rejects_empty() {
    let args = json!({"service": "  sonarr  ", "blank": "   "});
    assert_eq!(string_arg(&args, "service").unwrap(), "sonarr");
    assert!(string_arg(&args, "blank").is_err());
    assert!(string_arg(&args, "missing").is_err());
}

#[test]
fn optional_string_and_bool_arg() {
    let args = json!({"name": " x ", "empty": "", "flag": true});
    assert_eq!(optional_string(&args, "name"), Some("x".to_string()));
    assert_eq!(optional_string(&args, "empty"), None);
    assert_eq!(optional_string(&args, "absent"), None);
    assert!(bool_arg(&args, "flag"));
    assert!(!bool_arg(&args, "absent"));
}

#[test]
fn parses_mcp_actions() {
    assert_eq!(
        RustarrAction::from_mcp_args(&json!({"action": "integrations"})).unwrap(),
        RustarrAction::Integrations
    );
    assert_eq!(
        RustarrAction::from_mcp_args(&json!({
            "action": "api_get",
            "service": "sonarr",
            "path": "/api/v3/system/status"
        }))
        .unwrap(),
        RustarrAction::ApiGet {
            service: "sonarr".into(),
            path: "/api/v3/system/status".into(),
        }
    );
}

#[test]
fn parses_put_and_delete_actions() {
    assert_eq!(
        RustarrAction::from_mcp_args(&json!({
            "action": "api_put",
            "service": "sonarr",
            "path": "/api/v3/series/editor",
            "body": {"seriesIds": [1], "qualityProfileId": 4}
        }))
        .unwrap(),
        RustarrAction::ApiPut {
            service: "sonarr".into(),
            path: "/api/v3/series/editor".into(),
            body: json!({"seriesIds": [1], "qualityProfileId": 4}),
        }
    );
    assert_eq!(
        RustarrAction::from_mcp_args(&json!({
            "action": "api_delete",
            "service": "sonarr",
            "path": "/api/v3/series/9?deleteFiles=false",
            "confirm": true
        }))
        .unwrap(),
        RustarrAction::ApiDelete {
            service: "sonarr".into(),
            path: "/api/v3/series/9?deleteFiles=false".into(),
            body: None,
            confirm: true,
        }
    );
}

#[test]
fn rejects_missing_required_fields() {
    let error = RustarrAction::from_mcp_args(&json!({"action": "api_get"})).unwrap_err();
    assert!(error.to_string().contains("service"));
}

#[test]
fn rejects_unknown_action() {
    let error = RustarrAction::from_mcp_args(&json!({"action": "no_such"})).unwrap_err();
    assert!(error.to_string().contains("unknown rustarr action"));
}

// ── C2 typed extractors ──────────────────────────────────────────────────────────

#[test]
fn i64_arg_accepts_number_and_numeric_string() {
    assert_eq!(i64_arg(&json!({"id": 9}), "id").unwrap(), 9);
    assert_eq!(i64_arg(&json!({"id": "12"}), "id").unwrap(), 12);
    assert!(i64_arg(&json!({"id": "nope"}), "id").is_err());
    assert!(i64_arg(&json!({}), "id").is_err());
}

#[test]
fn optional_i64_errors_when_present_but_invalid() {
    // Absent -> Ok(None); valid (number or numeric string) -> Ok(Some); present
    // but unparseable -> Err so a malformed value is never silently dropped.
    assert_eq!(optional_i64(&json!({}), "take").unwrap(), None);
    assert_eq!(optional_i64(&json!({"take": null}), "take").unwrap(), None);
    assert_eq!(optional_i64(&json!({"take": 5}), "take").unwrap(), Some(5));
    assert_eq!(
        optional_i64(&json!({"take": "20"}), "take").unwrap(),
        Some(20)
    );
    assert!(optional_i64(&json!({"take": "nope"}), "take").is_err());
}

#[test]
fn i64_array_arg_accepts_array_single_and_numeric_strings() {
    assert_eq!(
        i64_array_arg(&json!({"ids": [1, 2, 3]}), "ids"),
        vec![1, 2, 3]
    );
    // CLI passes ids as numeric strings inside a JSON array.
    assert_eq!(
        i64_array_arg(&json!({"ids": ["4", "5"]}), "ids"),
        vec![4, 5]
    );
    assert_eq!(i64_array_arg(&json!({"ids": 7}), "ids"), vec![7]);
    assert!(i64_array_arg(&json!({}), "ids").is_empty());
}

#[test]
fn string_array_arg_accepts_array_and_single_string() {
    assert_eq!(
        string_array_arg(&json!({"title": ["A", "B"]}), "title"),
        vec!["A".to_string(), "B".to_string()]
    );
    assert_eq!(
        string_array_arg(&json!({"title": "Solo"}), "title"),
        vec!["Solo".to_string()]
    );
    assert!(string_array_arg(&json!({}), "title").is_empty());
}
