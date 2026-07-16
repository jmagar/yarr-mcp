use super::*;
pub(super) fn internal_tool_error_message(action: &str) -> String {
    format!("tool execution failed: kind=execution_error action='{action}'")
}

pub(super) fn tool_error_result(tool: &str, action: &str, error: &anyhow::Error) -> CallToolResult {
    let (class, reason, guidance) = match error.downcast_ref::<crate::yarr::UpstreamError>() {
        Some(crate::yarr::UpstreamError::Http {
            status,
            body_preview,
            ..
        }) => (
            "upstream_http",
            format!(
                "upstream returned HTTP {} ({})",
                status.as_u16(),
                crate::yarr::helpers::body_preview(body_preview)
            ),
            "verify the service URL and credentials, then retry; use service_status to test connectivity",
        ),
        Some(crate::yarr::UpstreamError::InvalidJson {
            content_type,
            body_preview,
            ..
        }) => (
            "invalid_upstream_response",
            format!(
                "upstream returned non-JSON content-type {} ({})",
                content_type.as_deref().unwrap_or("unknown"),
                crate::yarr::helpers::body_preview(body_preview)
            ),
            "verify the configured base URL points at the service API rather than a proxy login page",
        ),
        Some(crate::yarr::UpstreamError::QbittorrentLoginRejected { .. }) => (
            "authentication_rejected",
            "qBittorrent rejected the configured username/password".to_owned(),
            "verify the qBittorrent WebUI credentials and host-header allowlist, then retry",
        ),
        Some(crate::yarr::UpstreamError::ResponseTooLarge {
            observed, limit, ..
        }) => (
            "upstream_response_too_large",
            format!("upstream response was {observed} bytes; limit is {limit} bytes"),
            "narrow or paginate the request; use a supported artifact/export operation for large data",
        ),
        None if error.downcast_ref::<reqwest::Error>().is_some() => (
            "upstream_transport",
            "the upstream connection failed or timed out".to_owned(),
            "verify the configured service URL is reachable from yarr, then retry service_status",
        ),
        None => (
            "execution_error",
            format!(
                "{} ({})",
                internal_tool_error_message(action),
                crate::yarr::helpers::body_preview(&error.to_string())
            ),
            "check the action parameters and server logs; use action=help for the expected shape",
        ),
    };
    CallToolResult::error(vec![ContentBlock::text(format!(
        "ERROR: tool={tool} action={action} class={class}\nReason: {reason}\nHint: {guidance}"
    ))])
}

// ── auth helpers ──────────────────────────────────────────────────────────────

pub(super) fn require_auth_context<'a>(
    state: &AppState,
    ctx: &'a RequestContext<RoleServer>,
) -> Result<Option<&'a AuthContext>, ErrorData> {
    match &state.auth_policy {
        AuthPolicy::LoopbackDev | AuthPolicy::TrustedGatewayUnscoped => Ok(None),
        AuthPolicy::Mounted { .. } => {
            let parts = ctx
                .extensions
                .get::<axum::http::request::Parts>()
                .ok_or_else(|| {
                    axum_prometheus::metrics::counter!(
                        "yarr_auth_failures_total",
                        "reason" => "missing_http_context"
                    )
                    .increment(1);
                    tracing::error!(
                        "rmcp HTTP Parts extension absent — middleware ordering may be broken"
                    );
                    ErrorData::invalid_request("forbidden: missing http context", None)
                })?;
            let auth = parts.extensions.get::<AuthContext>().ok_or_else(|| {
                axum_prometheus::metrics::counter!(
                    "yarr_auth_failures_total",
                    "reason" => "missing_auth_context"
                )
                .increment(1);
                tracing::warn!("AuthContext absent — AuthLayer may not be mounted");
                ErrorData::invalid_request("forbidden: missing auth context", None)
            })?;
            Ok(Some(auth))
        }
    }
}

pub(super) fn check_scope(
    auth: &AuthContext,
    required_scope: &str,
    action: &str,
) -> Result<(), ErrorData> {
    if scope_satisfied(&auth.scopes, required_scope) {
        return Ok(());
    }
    tracing::warn!(
        subject = %auth.sub,
        action = %action,
        required_scope = %required_scope,
        "MCP tool denied: insufficient scope"
    );
    axum_prometheus::metrics::counter!(
        "yarr_auth_failures_total",
        "reason" => "insufficient_scope"
    )
    .increment(1);
    Err(ErrorData::invalid_request(
        format!("forbidden: requires scope: {required_scope}"),
        None,
    ))
}

pub(super) fn scope_satisfied(token_scopes: &[String], required: &str) -> bool {
    crate::actions::scopes_satisfy(token_scopes, required)
}

#[cfg(test)]
#[path = "rmcp_server_tests.rs"]
mod tests;
