use super::*;
use crate::actions::YarrAction;
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
    assert_eq!(
        optional_string(&args, "name").unwrap(),
        Some("x".to_string())
    );
    assert_eq!(optional_string(&args, "empty").unwrap(), None);
    assert_eq!(optional_string(&args, "absent").unwrap(), None);
    assert!(bool_arg(&args, "flag").unwrap());
    assert!(!bool_arg(&args, "absent").unwrap());
}

#[test]
fn optional_string_and_bool_reject_present_wrong_types() {
    let args = json!({"name": 42, "flag": "true"});
    assert!(optional_string(&args, "name").is_err());
    assert!(bool_arg(&args, "flag").is_err());
}

#[test]
fn parses_mcp_actions() {
    assert_eq!(
        YarrAction::from_mcp_args(&json!({"action": "help"})).unwrap(),
        YarrAction::Help
    );
    assert_eq!(
        YarrAction::from_mcp_args(&json!({
            "action": "api_get",
            "service": "sonarr",
            "path": "/api/v3/system/status"
        }))
        .unwrap(),
        YarrAction::ApiGet {
            service: "sonarr".into(),
            path: "/api/v3/system/status".into(),
        }
    );
}

#[test]
fn parses_put_and_delete_actions() {
    assert_eq!(
        YarrAction::from_mcp_args(&json!({
            "action": "api_put",
            "service": "sonarr",
            "path": "/api/v3/series/editor",
            "body": {"seriesIds": [1], "qualityProfileId": 4}
        }))
        .unwrap(),
        YarrAction::ApiPut {
            service: "sonarr".into(),
            path: "/api/v3/series/editor".into(),
            body: json!({"seriesIds": [1], "qualityProfileId": 4}),
        }
    );
    assert_eq!(
        YarrAction::from_mcp_args(&json!({
            "action": "api_delete",
            "service": "sonarr",
            "path": "/api/v3/series/9?deleteFiles=false",
        }))
        .unwrap(),
        YarrAction::ApiDelete {
            service: "sonarr".into(),
            path: "/api/v3/series/9?deleteFiles=false".into(),
            body: None,
        }
    );
}

#[test]
fn rejects_missing_required_fields() {
    let error = YarrAction::from_mcp_args(&json!({"action": "api_get"})).unwrap_err();
    assert!(error.to_string().contains("service"));
}

#[test]
fn rejects_unknown_action() {
    let error = YarrAction::from_mcp_args(&json!({"action": "no_such"})).unwrap_err();
    assert!(error.to_string().contains("unknown yarr action"));
}

// ── typed extractors ─────────────────────────────────────────────────────────────

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
