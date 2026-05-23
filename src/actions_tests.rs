use super::*;
use serde_json::json;

#[test]
fn action_metadata_matches_rustarr_surface() {
    assert_eq!(
        action_names(),
        vec![
            "integrations",
            "service_status",
            "api_get",
            "api_post",
            "help"
        ]
    );
    assert_eq!(required_scope_for_action("api_get"), Some(READ_SCOPE));
    assert_eq!(required_scope_for_action("api_post"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("help"), None);
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
fn rejects_missing_required_fields() {
    let error = RustarrAction::from_mcp_args(&json!({"action": "api_get"})).unwrap_err();
    assert!(error.to_string().contains("service"));
}
