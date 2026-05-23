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
    assert!(result["supported"]
        .as_array()
        .unwrap()
        .contains(&json!("sonarr")));
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

#[tokio::test]
async fn mcp_dispatch_rejects_missing_action() {
    let state = loopback_state();
    let error = execute_tool_without_peer_for_test(&state, "rustarr", json!({}))
        .await
        .expect_err("missing action should be rejected");
    assert!(error.to_string().contains("action is required"));
}

#[tokio::test]
async fn mcp_only_actions_require_peer_in_test_dispatch() {
    let state = loopback_state();
    for action in ["elicit_name", "scaffold_intent"] {
        let error =
            execute_tool_without_peer_for_test(&state, "rustarr", json!({ "action": action }))
                .await
                .expect_err("elicitation actions need a live MCP peer");
        assert!(error.to_string().contains("requires an MCP peer"));
    }
}
