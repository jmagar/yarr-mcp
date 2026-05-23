//! Route-level tests for REST dispatch, status, and mounted auth behavior.

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
};
use rustarr::{
    app::RustarrService,
    config::{McpConfig, RustarrConfig, ServiceConfig, ServiceKind},
    rustarr::RustarrClient,
    server::AppState,
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
async fn rest_integrations_accepts_nested_params() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/rustarr",
        None,
        Some(json!({"action": "integrations", "params": {}})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["supported"]
        .as_array()
        .unwrap()
        .contains(&json!("sonarr")));
}

#[tokio::test]
async fn rest_validation_errors_are_bad_requests() {
    let app = server::router(loopback_state());
    for (body, expected_error) in [
        json!({"action": "api_get", "params": {}}),
        json!({"action": "api_get", "params": {"service": ""}}),
        json!({"action": "api_get", "params": {"service": 42}}),
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
            request_json(app.clone(), Method::POST, "/v1/rustarr", None, Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
        assert!(response.get("error").is_some(), "{response}");
        if let Some(expected_error) = expected_error {
            assert_eq!(response["error"], expected_error);
        }
    }
}

#[tokio::test]
async fn rest_runtime_request_errors_are_bad_requests() {
    let app = server::router(loopback_state());
    for body in [
        json!({"action": "api_get", "params": {"service": "missing", "path": "/api/v3/system/status"}}),
        json!({"action": "api_get", "params": {"service": "sonarr", "path": "https://example.com/api"}}),
        json!({"action": "api_post", "params": {"service": "sonarr", "path": "https://example.com/api", "body": {}}}),
    ] {
        let (status, response) =
            request_json(app.clone(), Method::POST, "/v1/rustarr", None, Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
        assert!(response.get("error").is_some(), "{response}");
    }
}

#[tokio::test]
async fn rest_upstream_http_errors_are_bad_gateway() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("should bind test server");
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("should accept one request");
        let mut buffer = [0_u8; 1024];
        let _ = stream.read(&mut buffer);
        stream
            .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 4\r\n\r\nboom")
            .unwrap();
    });

    let config = RustarrConfig {
        services: vec![ServiceConfig {
            name: "sonarr".into(),
            kind: ServiceKind::Sonarr,
            base_url: format!("http://{addr}"),
            api_key: Some("key".into()),
            ..ServiceConfig::default()
        }],
    };
    let client = RustarrClient::new(&config).unwrap();
    let state = AppState {
        config: McpConfig::default(),
        auth_policy: AuthPolicy::LoopbackDev,
        service: RustarrService::new(client, config),
    };
    let app = server::router(state);

    let (status, response) = request_json(
        app,
        Method::POST,
        "/v1/rustarr",
        None,
        Some(json!({"action": "api_get", "params": {"service": "sonarr", "path": "/api/v3/system/status"}})),
    )
    .await;
    handle.join().unwrap();

    assert_eq!(status, StatusCode::BAD_GATEWAY, "{response}");
    assert!(response["error"]
        .as_str()
        .unwrap_or_default()
        .contains("HTTP 500"));
}

#[tokio::test]
async fn rest_rejects_mcp_only_actions_as_bad_requests() {
    let app = server::router(loopback_state());
    for action in ["elicit_name", "scaffold_intent"] {
        let (status, response) = request_json(
            app.clone(),
            Method::POST,
            "/v1/rustarr",
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
        "/v1/rustarr",
        None,
        Some(json!({"action": "help", "params": {}})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["actions"],
        json!([
            "integrations",
            "service_status",
            "api_get",
            "api_post",
            "help"
        ])
    );
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
        json!([
            "integrations",
            "service_status",
            "api_get",
            "api_post",
            "help"
        ])
    );
    assert_eq!(
        body["paths"]["/v1/rustarr"]["post"]["security"],
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
    assert_eq!(body["server"], "rustarr-mcp");
    assert_eq!(body["transport"], "http");
    assert!(body.get("version").is_some());
    assert!(body.get("api_url").is_none(), "{body}");
    assert!(body.get("api_key").is_none(), "{body}");
    assert!(body.get("upstream").is_none(), "{body}");
}

#[tokio::test]
async fn ready_reports_configured_service_count() {
    let app = server::router(loopback_state());
    let (status, body) = request_json(app, Method::GET, "/ready", None, None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
    assert_eq!(body["configured_services"], 1);
}

#[tokio::test]
async fn mounted_bearer_auth_protects_rest_endpoint() {
    let app = server::router(bearer_state("secret"));
    let body = json!({"action": "integrations", "params": {}});

    let (missing_status, _) = request_json(
        app.clone(),
        Method::POST,
        "/v1/rustarr",
        None,
        Some(body.clone()),
    )
    .await;
    assert_eq!(missing_status, StatusCode::UNAUTHORIZED);

    let (valid_status, valid_body) =
        request_json(app, Method::POST, "/v1/rustarr", Some("secret"), Some(body)).await;
    assert_eq!(valid_status, StatusCode::OK);
    assert!(valid_body["supported"].as_array().is_some());
}

#[tokio::test]
async fn trusted_gateway_unscoped_bypasses_local_auth() {
    let mut state = loopback_state();
    state.auth_policy = AuthPolicy::TrustedGatewayUnscoped;
    let app = server::router(state);
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/rustarr",
        None,
        Some(json!({"action": "integrations", "params": {}})),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["supported"].as_array().is_some());
}

#[tokio::test]
async fn oversized_body_returns_413() {
    // The router mounts RequestBodyLimitLayer at 65_536 bytes (64 KiB).
    // A body one byte over the limit must be rejected with HTTP 413.
    let app = server::router(loopback_state());
    let oversized_body = vec![b'x'; 65_537];
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/rustarr")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(oversized_body))
        .expect("request should build");

    let response = app.oneshot(request).await.expect("route should respond");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
