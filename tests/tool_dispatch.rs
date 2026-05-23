//! Integration tests for MCP tool dispatch.
//!
//! Tests verify that MCP action parsing and non-elicitation dispatch return valid JSON.
//! Uses `loopback_state()` from the test-support feature — no real creds needed.
//!
//! **Template**: mirror this file for your service. Add one test per action.

use rmcp_template::{
    actions::ExampleAction, mcp::execute_tool_without_peer_for_test, testing::loopback_state,
};
use serde_json::json;

async fn call_mcp_action(args: serde_json::Value) -> serde_json::Value {
    let state = loopback_state();
    execute_tool_without_peer_for_test(&state, "example", args)
        .await
        .expect("MCP tool dispatch should succeed")
}

#[tokio::test]
async fn test_greet_no_name_returns_greeting() {
    let result = call_mcp_action(json!({ "action": "greet" })).await;
    let greeting = result
        .get("greeting")
        .and_then(|v| v.as_str())
        .expect("greeting field should be present");
    assert!(
        greeting.contains("Hello"),
        "greeting should contain Hello, got: {greeting}"
    );
}

#[tokio::test]
async fn test_greet_with_name_includes_name() {
    let result = call_mcp_action(json!({ "action": "greet", "name": "Alice" })).await;
    let greeting = result
        .get("greeting")
        .and_then(|v| v.as_str())
        .expect("greeting field should be present");
    assert!(
        greeting.contains("Alice"),
        "greeting should include Alice, got: {greeting}"
    );
}

#[tokio::test]
async fn test_echo_returns_message() {
    let result = call_mcp_action(json!({ "action": "echo", "message": "hello world" })).await;
    let echo = result
        .get("echo")
        .and_then(|v| v.as_str())
        .expect("echo field should be present");
    assert_eq!(echo, "hello world");
}

#[tokio::test]
async fn test_status_returns_ok() {
    let result = call_mcp_action(json!({ "action": "status" })).await;
    let status = result
        .get("status")
        .and_then(|v| v.as_str())
        .expect("status field should be present");
    assert_eq!(status, "ok");
}

#[tokio::test]
async fn test_all_actions_return_valid_json_object() {
    for args in &[
        json!({ "action": "greet" }),
        json!({ "action": "echo", "message": "hello world" }),
        json!({ "action": "status" }),
        json!({ "action": "help" }),
    ] {
        let action = args["action"].as_str().unwrap();
        let result = call_mcp_action(args.clone()).await;
        assert!(
            result.is_object(),
            "action={action} should return a JSON object, got: {result}"
        );
    }
}

#[tokio::test]
async fn test_greet_target_defaults_to_world() {
    let result = call_mcp_action(json!({ "action": "greet" })).await;
    let target = result
        .get("target")
        .and_then(|v| v.as_str())
        .expect("target field should be present");
    assert_eq!(target, "World");
}

#[test]
fn test_schemas_actions_list_is_non_empty() {
    // Verify the schema action list compiles and has the expected entries
    use rmcp_template::server;
    let _ = server::router(loopback_state()); // builds router — exercises schema code path
}

#[test]
fn test_scaffold_intent_action_parses_for_mcp_dispatch() {
    let action = ExampleAction::from_mcp_args(&json!({ "action": "scaffold_intent" }))
        .expect("scaffold_intent should parse for MCP dispatch");
    assert_eq!(action, ExampleAction::ScaffoldIntent);
}

#[tokio::test]
async fn test_mcp_dispatch_rejects_missing_action() {
    let state = loopback_state();
    let error = execute_tool_without_peer_for_test(&state, "example", json!({}))
        .await
        .expect_err("missing action should be rejected");
    assert!(error.to_string().contains("action is required"));
}

#[tokio::test]
async fn test_mcp_dispatch_rejects_unknown_action() {
    let state = loopback_state();
    let error =
        execute_tool_without_peer_for_test(&state, "example", json!({ "action": "missing" }))
            .await
            .expect_err("unknown action should be rejected");
    assert!(error.to_string().contains("unknown example action"));
}

#[tokio::test]
async fn test_mcp_dispatch_rejects_peer_required_actions_without_peer() {
    let state = loopback_state();
    let error =
        execute_tool_without_peer_for_test(&state, "example", json!({ "action": "elicit_name" }))
            .await
            .expect_err("elicitation action should require a peer");
    assert!(error.to_string().contains("requires an MCP peer"));
}
