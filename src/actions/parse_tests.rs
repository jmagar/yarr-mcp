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
            "body": {"seriesIds": [1], "qualityProfileId": 4},
            "confirm": true
        }))
        .unwrap(),
        RustarrAction::ApiPut {
            service: "sonarr".into(),
            path: "/api/v3/series/editor".into(),
            body: json!({"seriesIds": [1], "qualityProfileId": 4}),
            confirm: true,
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
