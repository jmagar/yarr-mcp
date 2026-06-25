//! Tests for the generated-operation executor helpers.

use crate::testing::loopback_state;
use serde_json::json;

#[test]
fn http_method_maps_to_reqwest_method() {
    use crate::openapi::HttpMethod;

    assert_eq!(HttpMethod::Get.as_reqwest(), reqwest::Method::GET);
    assert_eq!(HttpMethod::Post.as_reqwest(), reqwest::Method::POST);
    assert_eq!(HttpMethod::Put.as_reqwest(), reqwest::Method::PUT);
    assert_eq!(HttpMethod::Delete.as_reqwest(), reqwest::Method::DELETE);
    assert_eq!(HttpMethod::Patch.as_reqwest(), reqwest::Method::PATCH);
}

#[tokio::test]
async fn execute_operation_requires_each_path_param_before_dispatch() {
    // loopback configures `sonarr`. get_series_by_id needs `id`; omitting it must
    // fail at param resolution with a clear message BEFORE any network call.
    let service = loopback_state().service;
    let err = service
        .execute_operation("sonarr", "get_series_by_id", &json!({}))
        .await
        .expect_err("missing path param must error before HTTP");
    let msg = err.to_string();
    assert!(
        msg.contains("path param") && msg.contains("id"),
        "got: {msg}"
    );
}

#[tokio::test]
async fn execute_operation_rejects_unknown_op() {
    let service = loopback_state().service;
    let err = service
        .execute_operation("sonarr", "no_such_op", &json!({}))
        .await
        .expect_err("unknown op must error");
    assert!(err.to_string().contains("unknown"), "got: {err}");
}

#[test]
fn op_is_destructive_delete_flags_only_delete_ops() {
    let service = loopback_state().service;
    // sonarr has a generated DELETE op and many non-DELETE ops.
    assert!(service.op_is_destructive_delete("sonarr", "delete_series_by_id"));
    assert!(!service.op_is_destructive_delete("sonarr", "get_series"));
    // Unknown service/op → not destructive (no false positive).
    assert!(!service.op_is_destructive_delete("sonarr", "no_such_op"));
    assert!(!service.op_is_destructive_delete("nope", "delete_series_by_id"));
}
