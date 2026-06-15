use rustarr::{
    actions::RustarrAction, mcp::execute_tool_without_peer_for_test, testing::loopback_state,
};
use serde_json::json;

async fn call_mcp_action(args: serde_json::Value) -> serde_json::Value {
    let state = loopback_state();
    execute_tool_without_peer_for_test(&state, "rustarr", args)
        .await
        .expect("MCP tool dispatch should succeed")
}

#[tokio::test]
async fn integrations_returns_supported_services() {
    let result = call_mcp_action(json!({ "action": "integrations" })).await;
    // `supported` is now a list of {kind, capability} objects (registry-derived).
    assert!(result["supported"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["kind"] == "sonarr"));
}

#[test]
fn service_status_action_parses_for_mcp_dispatch() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "service_status",
        "service": "sonarr"
    }))
    .expect("service_status should parse");
    assert!(matches!(action, RustarrAction::ServiceStatus { .. }));
}

#[tokio::test]
async fn help_returns_text() {
    let result = call_mcp_action(json!({ "action": "help" })).await;
    assert!(result["help"].as_str().unwrap().contains("api_get"));
}

#[test]
fn api_post_action_parses_for_mcp_dispatch() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "api_post",
        "service": "sonarr",
        "path": "/api/v3/command",
        "body": {"name": "RefreshSeries"}
    }))
    .expect("api_post should parse");
    assert!(matches!(action, RustarrAction::ApiPost { .. }));
}

#[test]
fn api_put_action_parses_for_mcp_dispatch() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "api_put",
        "service": "sonarr",
        "path": "/api/v3/series/editor",
        "body": {"seriesIds": [1], "qualityProfileId": 4},
        "confirm": true
    }))
    .expect("api_put should parse");
    assert!(matches!(
        action,
        RustarrAction::ApiPut { confirm: true, .. }
    ));
}

#[test]
fn api_delete_action_parses_for_mcp_dispatch() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "api_delete",
        "service": "sonarr",
        "path": "/api/v3/series/9?deleteFiles=false",
        "confirm": true
    }))
    .expect("api_delete should parse");
    assert!(matches!(
        action,
        RustarrAction::ApiDelete {
            body: None,
            confirm: true,
            ..
        }
    ));
}

#[tokio::test]
async fn api_put_requires_confirm() {
    let state = loopback_state();
    let error = state
        .service
        .api_put("sonarr", "/api/v3/series/editor", json!({}), false)
        .await
        .expect_err("api_put without confirm should be rejected");
    assert!(error.to_string().contains("confirm=true"));
}

#[tokio::test]
async fn api_delete_requires_confirm() {
    let state = loopback_state();
    let error = state
        .service
        .api_delete("sonarr", "/api/v3/series/9", None, false)
        .await
        .expect_err("api_delete without confirm should be rejected");
    assert!(error.to_string().contains("confirm=true"));
}

#[tokio::test]
async fn mcp_dispatch_rejects_missing_action() {
    let state = loopback_state();
    let error = execute_tool_without_peer_for_test(&state, "rustarr", json!({}))
        .await
        .expect_err("missing action should be rejected");
    assert!(error.to_string().contains("action is required"));
}
