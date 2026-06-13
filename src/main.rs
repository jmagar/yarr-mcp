//! Binary entry point — mode dispatch only.
//!
//! Modes:
//!   `rustarr [serve]`        Start MCP HTTP server (default if no args)
//!   `rustarr mcp`            Start MCP stdio transport
//!   `rustarr integrations`   CLI inventory command
//!   `rustarr get ...`        CLI upstream GET command
//!   `rustarr status`         CLI status command
//!   `rustarr --help`         Print usage
//!   `rustarr --version`      Print version
//!
//! **Template**: add your binary name in Cargo.toml `[[bin]] name = "..."`.
//! Extend `run_cli` if you add more CLI subcommands.

use anyhow::Result;
use std::sync::Arc;

use rmcp::{transport::stdio, ServiceExt};
use rustarr::{
    app::RustarrService,
    cli,
    config::Config,
    mcp,
    rustarr::RustarrClient,
    server::{self, resolve_auth_policy_kind, AppState, AuthPolicy, AuthPolicyKind},
};
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Handle meta-flags before initialising logging (they print and exit)
    match args.as_slice() {
        [f] if matches!(f.as_str(), "--help" | "-h") => {
            eprintln!("{}", cli::usage());
            return Ok(());
        }
        [f] if matches!(f.as_str(), "--version" | "-V" | "version") => {
            println!("rustarr {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    // Suppress logs in stdio/CLI mode — MCP clients communicate over stdio
    // and cannot tolerate log lines mixed into the JSON stream.
    let stdio_mode = matches!(args.as_slice(), [c] if c == "mcp");
    let serve_mode = args.is_empty()
        || matches!(args.as_slice(), [c] if c == "serve")
        || matches!(args.as_slice(), [a, b] if a == "serve" && b == "mcp");

    let log_level = if stdio_mode || !serve_mode {
        "warn"
    } else {
        "info"
    };
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .with_writer(std::io::stderr)
        .with_target(true)
        .init();

    if serve_mode {
        serve_mcp().await
    } else if stdio_mode {
        serve_stdio_mcp().await
    } else {
        run_cli().await
    }
}

// ── modes ─────────────────────────────────────────────────────────────────────

/// Start the MCP HTTP server (Streamable HTTP transport).
async fn serve_mcp() -> Result<()> {
    let config = Config::load()?;
    let state = build_state(config).await?;

    info!(
        bind = %state.config.bind_addr(),
        server_name = %state.config.server_name,
        auth = ?state.auth_policy,
        "rustarr-mcp starting"
    );

    let bind = state.config.bind_addr();
    let app = server::router(state).layer(tower_http::trace::TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    info!(bind = %bind, "MCP HTTP server listening");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Start the MCP stdio transport (for local/subprocess MCP clients).
///
/// Stdio is always LoopbackDev — it's a local trusted pipe between parent and
/// child process. HTTP auth middleware doesn't apply; forcing Mounted here
/// breaks all stdio clients with "forbidden: missing http context".
async fn serve_stdio_mcp() -> Result<()> {
    let config = Config::load()?;
    let service = RustarrService::new(RustarrClient::new(&config.rustarr)?, config.rustarr.clone());
    let state = AppState {
        config: config.mcp,
        auth_policy: AuthPolicy::LoopbackDev, // stdio = trusted local transport
        service,
    };
    let svc = mcp::rmcp_server(state).serve(stdio()).await?;
    svc.waiting().await?;
    Ok(())
}

/// Dispatch CLI subcommands.
async fn run_cli() -> Result<()> {
    // Copy any CLAUDE_PLUGIN_OPTION_* values into RUSTARR_* env vars BEFORE
    // Config::load reads them. No-op outside the plugin context.
    cli::apply_plugin_options();
    let config = Config::load()?;
    match cli::parse_args()? {
        Some(cli::Command::Doctor { json }) => {
            // Doctor needs the full Config (not just RustarrConfig) to check
            // MCP port, auth mode, etc. — intercept here before service construction.
            cli::doctor::run_doctor(&config, json).await
        }
        Some(cli::Command::Watch { url, interval }) => {
            // Watch needs the MCP port to build the default URL but no service layer.
            let base = url.unwrap_or_else(|| format!("http://localhost:{}", config.mcp.port));
            cli::watch::run_watch(&base, interval).await
        }
        Some(cli::Command::Setup(command)) => cli::run_setup(&config, command).await,
        Some(cmd) => cli::run(cmd, &config.rustarr).await,
        None => {
            eprintln!("Unknown command. Run `rustarr --help` for usage.");
            std::process::exit(1);
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

async fn build_state(config: Config) -> Result<AppState> {
    let auth_policy = build_auth_policy(&config).await?;
    let service = RustarrService::new(RustarrClient::new(&config.rustarr)?, config.rustarr.clone());
    Ok(AppState {
        config: config.mcp,
        auth_policy,
        service,
    })
}

async fn build_auth_policy(config: &Config) -> Result<AuthPolicy> {
    match resolve_auth_policy_kind(config, config.mcp.trusted_gateway)? {
        AuthPolicyKind::LoopbackDev => Ok(AuthPolicy::LoopbackDev),
        AuthPolicyKind::TrustedGatewayUnscoped => Ok(AuthPolicy::TrustedGatewayUnscoped),
        AuthPolicyKind::MountedBearer => Ok(AuthPolicy::Mounted { auth_state: None }),
        AuthPolicyKind::MountedOAuth => {
            let auth_cfg = lab_auth::config::AuthConfigBuilder::new()
                .env_prefix("RUSTARR_MCP")
                .session_cookie_name("rustarr_mcp_session")
                .scopes_supported(vec![
                    rustarr::actions::READ_SCOPE.into(),
                    rustarr::actions::WRITE_SCOPE.into(),
                ])
                .default_scope("rustarr:read")
                .resource_path("/mcp")
                .enable_dynamic_registration(true)
                .build_from_sources(auth_config_sources(config))
                .map_err(|e| anyhow::anyhow!("OAuth config error: {e}"))?;
            let auth_state = lab_auth::state::AuthState::new(auth_cfg)
                .await
                .map_err(|e| anyhow::anyhow!("OAuth state init error: {e}"))?;
            Ok(AuthPolicy::Mounted {
                auth_state: Some(Arc::new(auth_state)),
            })
        }
    }
}

fn auth_config_sources(config: &Config) -> Vec<(String, String)> {
    let auth = &config.mcp.auth;
    let mut vars = vec![
        ("RUSTARR_MCP_AUTH_MODE".into(), "oauth".into()),
        (
            "RUSTARR_MCP_AUTH_SQLITE_PATH".into(),
            auth.sqlite_path.clone(),
        ),
        ("RUSTARR_MCP_AUTH_KEY_PATH".into(), auth.key_path.clone()),
        (
            "RUSTARR_MCP_AUTH_ACCESS_TOKEN_TTL_SECS".into(),
            auth.access_token_ttl_secs.to_string(),
        ),
        (
            "RUSTARR_MCP_AUTH_REFRESH_TOKEN_TTL_SECS".into(),
            auth.refresh_token_ttl_secs.to_string(),
        ),
        (
            "RUSTARR_MCP_AUTH_CODE_TTL_SECS".into(),
            auth.auth_code_ttl_secs.to_string(),
        ),
        (
            "RUSTARR_MCP_AUTH_REGISTER_RPM".into(),
            auth.register_rpm.to_string(),
        ),
        (
            "RUSTARR_MCP_AUTH_AUTHORIZE_RPM".into(),
            auth.authorize_rpm.to_string(),
        ),
    ];
    push_optional(&mut vars, "RUSTARR_MCP_PUBLIC_URL", &auth.public_url);
    push_optional(
        &mut vars,
        "RUSTARR_MCP_GOOGLE_CLIENT_ID",
        &auth.google_client_id,
    );
    push_optional(
        &mut vars,
        "RUSTARR_MCP_GOOGLE_CLIENT_SECRET",
        &auth.google_client_secret,
    );
    if !auth.admin_email.is_empty() {
        vars.push((
            "RUSTARR_MCP_AUTH_ADMIN_EMAIL".into(),
            auth.admin_email.clone(),
        ));
    }
    if !auth.allowed_emails.is_empty() {
        vars.push((
            "RUSTARR_MCP_AUTH_ALLOWED_EMAILS".into(),
            auth.allowed_emails.join(","),
        ));
    }
    if !auth.allowed_client_redirect_uris.is_empty() {
        vars.push((
            "RUSTARR_MCP_AUTH_ALLOWED_CLIENT_REDIRECT_URIS".into(),
            auth.allowed_client_redirect_uris.join(","),
        ));
    }
    vars
}

fn push_optional(vars: &mut Vec<(String, String)>, key: &str, value: &Option<String>) {
    if let Some(value) = value.as_ref().filter(|value| !value.is_empty()) {
        vars.push((key.into(), value.clone()));
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "CTRL+C handler failed");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "SIGTERM handler failed");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Shutdown signal received");
}
