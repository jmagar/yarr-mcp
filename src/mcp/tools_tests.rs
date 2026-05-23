use crate::testing::loopback_state;
use serde_json::json;

#[tokio::test]
async fn integrations_dispatch_returns_object() {
    let state = loopback_state();
    let value = super::execute_tool_without_peer_for_test(
        &state,
        "rustarr",
        json!({"action": "integrations"}),
    )
    .await
    .unwrap();
    assert!(value.is_object());
}
