//! `rustarr` library crate.
//!
//! Exposes the service layer, config, and transport client so that integration
//! tests can import them without duplicating state construction.
//!
//! Public modules:
//!   [`app`]     — `RustarrService` (business logic)
//!   [`config`]  — `Config`, `RustarrConfig`, `McpConfig`
//!   [`rustarr`] — `RustarrClient` (transport stub)
//!   [`mcp`]     — MCP protocol layer (tools, schemas, prompts, server handler)
//!   [`server`]  — `AppState`, `AuthPolicy`, HTTP router

pub mod actions;
pub mod app;
pub mod cli;
pub mod config;
pub mod logging;
pub mod mcp;
pub mod rustarr;
pub mod scaffold;
pub mod server;
pub mod token_limit;

/// Test helpers — available when `features = ["test-support"]` or in `cfg(test)`.
///
/// Use these in integration tests to construct `AppState` without real creds.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod testing {
    use std::sync::{Arc, Mutex};

    /// Process-wide lock serialising any test that mutates environment variables.
    ///
    /// All test modules that call `std::env::set_var` / `std::env::remove_var` on
    /// shared keys (`RUSTARR_HOME`, `RUSTARR_SERVICES`, …) **must** hold this lock
    /// for the duration of their env mutation + `Config::load()` call.  Using a
    /// single shared instance prevents races between `config_tests` and
    /// `setup_tests` which previously each defined their own, independent `Mutex`.
    pub static ENV_LOCK: Mutex<()> = Mutex::new(());

    use crate::{
        app::RustarrService,
        config::{McpConfig, RustarrConfig, ServiceConfig, ServiceKind},
        rustarr::RustarrClient,
        server::{AppState, AuthPolicy},
    };

    fn stub_service() -> RustarrService {
        let config = RustarrConfig {
            services: vec![ServiceConfig {
                name: "sonarr".into(),
                kind: ServiceKind::Sonarr,
                base_url: "http://localhost:1".into(),
                api_key: Some("test".into()),
                ..ServiceConfig::default()
            }],
        };
        let client = RustarrClient::new(&config).expect("stub client should always build");
        RustarrService::new(client, config)
    }

    /// `AppState` with no auth (loopback trust boundary).
    /// Use this for unit tests that don't need auth.
    pub fn loopback_state() -> AppState {
        AppState {
            config: McpConfig::default(),
            auth_policy: AuthPolicy::LoopbackDev,
            service: stub_service(),
        }
    }

    /// `AppState` requiring a static bearer token.
    pub fn bearer_state(token: &str) -> AppState {
        AppState {
            config: McpConfig {
                api_token: Some(token.to_string()),
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted { auth_state: None },
            service: stub_service(),
        }
    }

    /// `AppState` with full OAuth (requires data directory for SQLite + key file).
    pub async fn oauth_state(data_dir: &std::path::Path) -> AppState {
        let auth_state = build_auth_state(data_dir).await;
        AppState {
            config: McpConfig {
                auth: crate::config::AuthConfig {
                    public_url: Some("https://rustarr.rustarr.com".to_string()),
                    ..Default::default()
                },
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted {
                auth_state: Some(Arc::new(auth_state)),
            },
            service: stub_service(),
        }
    }

    pub async fn build_auth_state(data_dir: &std::path::Path) -> lab_auth::state::AuthState {
        let vars: Vec<(String, String)> = vec![
            ("RUSTARR_MCP_AUTH_MODE".into(), "oauth".into()),
            (
                "RUSTARR_MCP_PUBLIC_URL".into(),
                "https://rustarr.rustarr.com".into(),
            ),
            (
                "RUSTARR_MCP_GOOGLE_CLIENT_ID".into(),
                "test-client-id".into(),
            ),
            (
                "RUSTARR_MCP_GOOGLE_CLIENT_SECRET".into(),
                "test-client-secret".into(),
            ),
            (
                "RUSTARR_MCP_AUTH_ADMIN_EMAIL".into(),
                "admin@rustarr.com".into(),
            ),
            (
                "RUSTARR_MCP_AUTH_SQLITE_PATH".into(),
                data_dir.join("auth.db").display().to_string(),
            ),
            (
                "RUSTARR_MCP_AUTH_KEY_PATH".into(),
                data_dir.join("auth-jwt.pem").display().to_string(),
            ),
        ];

        let auth_config = lab_auth::config::AuthConfigBuilder::new()
            .env_prefix("RUSTARR_MCP")
            .session_cookie_name("rustarr_mcp_session")
            .scopes_supported(vec![
                crate::actions::READ_SCOPE.into(),
                crate::actions::WRITE_SCOPE.into(),
            ])
            .default_scope("rustarr:read")
            .resource_path("/mcp")
            .build_from_sources(vars)
            .expect("test auth config should build");

        lab_auth::state::AuthState::new(auth_config)
            .await
            .expect("test auth state should init")
    }
}
