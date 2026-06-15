use axum::{body::Body, http::Request};
use tower::ServiceExt;

use super::router;

#[tokio::test]
async fn health_is_served_without_auth() {
    let response = router(crate::testing::loopback_state())
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn metrics_is_served_without_auth() {
    let response = router(crate::testing::loopback_state())
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn metrics_exposition_carries_rustarr_prefix() {
    // Record a request first so the prometheus recorder has an observed sample
    // (the metric handle is a process-global `OnceLock`, so this populates the
    // same registry the `/metrics` handler renders from).
    router(crate::testing::loopback_state())
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("warmup request should respond");

    let response = router(crate::testing::loopback_state())
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    // The body must be Prometheus exposition carrying the `rustarr` prefix —
    // guards against the handler rendering empty or the OnceLock prefix wiring
    // silently dropping out (both would still return 200).
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("metrics body should read");
    let text = String::from_utf8(body.to_vec()).expect("metrics body should be UTF-8");
    assert!(
        text.contains("rustarr"),
        "metrics exposition should carry the rustarr prefix, got: {text:?}"
    );
}
