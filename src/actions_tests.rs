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
            "elicit_name",
            "scaffold_intent",
            "help"
        ]
    );
    assert_eq!(required_scope_for_action("api_get"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_post"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("elicit_name"), Some(READ_SCOPE));
    assert_eq!(
        required_scope_for_action("scaffold_intent"),
        Some(READ_SCOPE)
    );
    assert_eq!(required_scope_for_action("help"), None);
    assert_eq!(
        rest_action_names(),
        vec![
            "integrations",
            "service_status",
            "api_get",
            "api_post",
            "help"
        ]
    );
    assert_eq!(
        mcp_only_action_names(),
        vec!["elicit_name", "scaffold_intent"]
    );
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
    assert_eq!(
        RustarrAction::from_mcp_args(&json!({"action": "elicit_name"})).unwrap(),
        RustarrAction::ElicitName
    );
    assert_eq!(
        RustarrAction::from_mcp_args(&json!({"action": "scaffold_intent"})).unwrap(),
        RustarrAction::ScaffoldIntent
    );
}

#[test]
fn rejects_mcp_only_actions_over_rest() {
    for action in ["elicit_name", "scaffold_intent"] {
        let error = RustarrAction::from_rest(action, &json!({})).unwrap_err();
        assert!(error.to_string().contains("not available over REST"));
    }
}

#[test]
fn rejects_missing_required_fields() {
    let error = RustarrAction::from_mcp_args(&json!({"action": "api_get"})).unwrap_err();
    assert!(error.to_string().contains("service"));
}
