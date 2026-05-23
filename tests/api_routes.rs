//! Route-level tests for REST dispatch, status, and mounted auth behavior.

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use rmcp_template::{
    server::{self, AuthPolicy},
    testing::{bearer_state, loopback_state},
};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn request_json(
    app: axum::Router,
    method: Method,
    path: &str,
    auth: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = auth {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = if let Some(body) = body {
        builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .expect("request should build")
    } else {
        builder.body(Body::empty()).expect("request should build")
    };

    let response = app.oneshot(request).await.expect("route should respond");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let value = serde_json::from_slice(&bytes).expect("response should be JSON");
    (status, value)
}

#[tokio::test]
async fn rest_echo_accepts_nested_params() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/example",
        None,
        Some(json!({"action": "echo", "params": {"message": "hello"}})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["echo"], "hello");
}

#[tokio::test]
async fn rest_validation_errors_are_bad_requests() {
    let app = server::router(loopback_state());
    for (body, expected_error) in [
        json!({"action": "echo", "params": {}}),
        json!({"action": "echo", "params": {"message": ""}}),
        json!({"action": "echo", "params": {"message": 42}}),
        json!({"action": "missing", "params": {}}),
        json!({"params": {}}),
    ]
    .into_iter()
    .map(|body| {
        let expected_error = if body.get("action").is_none() {
            Some("action is required")
        } else {
            None
        };
        (body, expected_error)
    }) {
        let (status, response) =
            request_json(app.clone(), Method::POST, "/v1/example", None, Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
        assert!(response.get("error").is_some(), "{response}");
        if let Some(expected_error) = expected_error {
            assert_eq!(response["error"], expected_error);
        }
    }
}

#[tokio::test]
async fn rest_rejects_mcp_only_actions_as_bad_requests() {
    let app = server::router(loopback_state());
    for action in ["elicit_name", "scaffold_intent"] {
        let (status, response) = request_json(
            app.clone(),
            Method::POST,
            "/v1/example",
            None,
            Some(json!({"action": action, "params": {}})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
        assert!(response["error"]
            .as_str()
            .unwrap_or_default()
            .contains("not available over REST"));
    }
}

#[tokio::test]
async fn rest_help_excludes_mcp_only_actions_from_rest_actions() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/example",
        None,
        Some(json!({"action": "help", "params": {}})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["actions"], json!(["greet", "echo", "status", "help"]));
    assert_eq!(
        body["mcp_only_actions"],
        json!(["elicit_name", "scaffold_intent"])
    );
}

#[tokio::test]
async fn openapi_json_is_public_and_excludes_mcp_only_actions() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(app, Method::GET, "/openapi.json", None, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["openapi"], "3.1.0");
    assert_eq!(
        body["components"]["schemas"]["ActionName"]["enum"],
        json!(["greet", "echo", "status", "help"])
    );
    assert_eq!(
        body["paths"]["/v1/example"]["post"]["security"],
        json!([{"BearerAuth": []}, {}])
    );
    assert!(
        body["components"]["schemas"]["StatusResponse"]["properties"]
            .get("api_url")
            .is_none(),
        "{body}"
    );
}

#[tokio::test]
async fn status_returns_only_local_redacted_metadata() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(app, Method::GET, "/status", None, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["server"], "example-mcp");
    assert_eq!(body["transport"], "http");
    assert!(body.get("version").is_some());
    assert!(body.get("api_url").is_none(), "{body}");
    assert!(body.get("api_key").is_none(), "{body}");
    assert!(body.get("upstream").is_none(), "{body}");
}

#[tokio::test]
async fn mounted_bearer_auth_protects_rest_endpoint() {
    let app = server::router(bearer_state("secret"));
    let body = json!({"action": "status", "params": {}});

    let (missing_status, _) = request_json(
        app.clone(),
        Method::POST,
        "/v1/example",
        None,
        Some(body.clone()),
    )
    .await;
    assert_eq!(missing_status, StatusCode::UNAUTHORIZED);

    let (valid_status, valid_body) =
        request_json(app, Method::POST, "/v1/example", Some("secret"), Some(body)).await;
    assert_eq!(valid_status, StatusCode::OK);
    assert_eq!(valid_body["status"], "ok");
}

#[tokio::test]
async fn trusted_gateway_unscoped_bypasses_local_auth() {
    let mut state = loopback_state();
    state.auth_policy = AuthPolicy::TrustedGatewayUnscoped;
    let app = server::router(state);
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/example",
        None,
        Some(json!({"action": "status", "params": {}})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn oversized_body_returns_413() {
    // The router mounts RequestBodyLimitLayer at 65_536 bytes (64 KiB).
    // A body one byte over the limit must be rejected with HTTP 413.
    let app = server::router(loopback_state());
    let oversized_body = vec![b'x'; 65_537];
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/example")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(oversized_body))
        .expect("request should build");

    let response = app.oneshot(request).await.expect("route should respond");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
