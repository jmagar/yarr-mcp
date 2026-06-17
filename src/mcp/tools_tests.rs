use crate::testing::loopback_state;
use serde_json::json;

#[tokio::test]
async fn integrations_dispatch_returns_object() {
    let state = loopback_state();
    let value = super::execute_tool_without_peer_for_test(
        &state,
        "sonarr",
        json!({"action": "integrations"}),
    )
    .await
    .unwrap();
    assert!(value.is_object());
}

#[tokio::test]
async fn service_tool_injects_service_argument() {
    let state = loopback_state();
    let result = super::execute_tool_without_peer_for_test(
        &state,
        "sonarr",
        json!({"action": "service_status"}),
    )
    .await;
    if let Err(err) = result {
        assert!(
            !err.to_string().contains("service"),
            "service-named tool should inject service arg: {err}"
        );
    }
}
