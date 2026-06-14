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
            "api_put",
            "api_delete",
            "help"
        ]
    );
    assert_eq!(required_scope_for_action("api_get"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_post"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_put"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_delete"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("help"), None);
    assert_eq!(
        rest_action_names(),
        vec![
            "integrations",
            "service_status",
            "api_get",
            "api_post",
            "api_put",
            "api_delete",
            "help"
        ]
    );
    assert_eq!(mcp_only_action_names(), Vec::<&str>::new());
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
