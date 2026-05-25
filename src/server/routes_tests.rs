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
