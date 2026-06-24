use crate::testing::loopback_state;
use serde_json::json;

#[tokio::test]
async fn yarr_tool_dispatches_codemode() {
    // The single `yarr` tool takes only `code` and runs it as the codemode action.
    let state = loopback_state();
    let value = super::execute_tool_without_peer_for_test(
        &state,
        "yarr",
        json!({ "code": "async () => 6 * 7" }),
    )
    .await
    .unwrap();
    assert_eq!(value["result"], 42);
}

#[tokio::test]
async fn help_dispatch_returns_object() {
    let state = loopback_state();
    let value =
        super::execute_tool_without_peer_for_test(&state, "sonarr", json!({"action": "help"}))
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
