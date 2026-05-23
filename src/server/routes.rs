//! Axum router — wires HTTP endpoints to the MCP service, REST API, and auth middleware.
//!
//! Endpoints:
//!   `POST /mcp`         — MCP Streamable HTTP transport (tools, resources, prompts)
//!   `GET  /health`      — Health check (unauthenticated)
//!   `GET  /ready`       — Local readiness check (unauthenticated)
//!   `GET  /status`      — Runtime status (unauthenticated, redacts secrets)
//!   `GET  /openapi.json` — Generated REST OpenAPI schema (unauthenticated)
//!   `POST /v1/rustarr`  — REST API action dispatch (see `crate::api`)
//!   `/*`                — SPA fallback for embedded web UI (when web feature enabled)

use std::sync::Arc;

use axum::{
    http::{HeaderValue, Method, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::json;
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer};

use crate::api::{api_dispatch, health, openapi_json, ready, status};
use crate::mcp::{allowed_origins, streamable_http_config, streamable_http_service};
use crate::server::{build_auth_layer, AppState, AuthPolicy};

const MCP_BODY_LIMIT_BYTES: usize = 65_536;

pub fn router(state: AppState) -> Router {
    let rmcp_config = streamable_http_config(&state.config);

    let resource_url = match &state.auth_policy {
        AuthPolicy::Mounted { .. } => state
            .config
            .auth
            .public_url
            .as_deref()
            .map(|u| Arc::<str>::from(format!("{}/mcp", u.trim_end_matches('/')))),
        AuthPolicy::LoopbackDev | AuthPolicy::TrustedGatewayUnscoped => None,
    };

    // Auth layer applied to both /mcp and /v1/rustarr.
    let auth_layer = build_auth_layer(
        &state.auth_policy,
        state.config.api_token.as_deref().map(Arc::<str>::from),
        resource_url,
    );

    let api_and_mcp: Router<AppState> = Router::new()
        .nest_service("/mcp", streamable_http_service(state.clone(), rmcp_config))
        .route("/v1/rustarr", post(api_dispatch));

    let api_and_mcp_resolved: Router<()> = api_and_mcp.with_state(state.clone());

    let authenticated = if let Some(layer) = auth_layer {
        api_and_mcp_resolved.layer(layer)
    } else {
        api_and_mcp_resolved
    };

    let oauth_router: Option<Router> = if let AuthPolicy::Mounted {
        auth_state: Some(ref state_arc),
    } = state.auth_policy
    {
        let auth_state = state_arc.as_ref().clone();
        let path_based_discovery = Router::new()
            .route(
                "/mcp/.well-known/oauth-authorization-server",
                get(lab_auth::metadata::authorization_server_metadata),
            )
            .route(
                "/mcp/.well-known/openid-configuration",
                get(lab_auth::metadata::authorization_server_metadata),
            )
            .route(
                "/mcp/.well-known/oauth-protected-resource",
                get(lab_auth::metadata::protected_resource_metadata),
            )
            .with_state(auth_state.clone());
        Some(lab_auth::routes::router(auth_state).merge(path_based_discovery))
    } else {
        None
    };

    let public: Router<()> = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/status", get(status))
        .route("/openapi.json", get(openapi_json))
        .with_state(state.clone());

    let mut base: Router<()> = Router::new().merge(authenticated).merge(public);

    if let Some(oauth) = oauth_router {
        base = base.merge(oauth);
    }

    let base = if crate::web::web_assets_available() {
        base.fallback(crate::web::serve_web_assets)
    } else {
        base.fallback(|| async { (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))) })
    };

    base.layer(RequestBodyLimitLayer::new(MCP_BODY_LIMIT_BYTES))
        .layer(cors_layer(&state.config))
}

fn cors_layer(config: &crate::config::McpConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins(config)
        .into_iter()
        .filter_map(|o| match o.parse::<HeaderValue>() {
            Ok(hv) => Some(hv),
            Err(e) => {
                tracing::warn!(origin = %o, error = %e, "invalid CORS origin — skipping");
                None
            }
        })
        .collect();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ])
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;
