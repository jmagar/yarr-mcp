//! Per-service request authentication, driven by `KindDescriptor.auth_style`,
//! plus the qBittorrent cookie-session lifecycle.
//!
//! Header auth lives here; query-string auth (apikey / X-Plex-Token) lives in
//! [`super::helpers::build_url`]. The two never both append the api key.

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};

use super::helpers::build_url;
use crate::capability::AuthStyle;
use crate::config::ServiceConfig;

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;

/// Jellyfin's preferred auth header. The MediaBrowser token value is quoted per
/// the Emby/Jellyfin auth scheme; `X-Emby-Token` remains as a fallback.
fn jellyfin_authorization(key: &str) -> String {
    format!("MediaBrowser Token=\"{key}\"")
}

/// Apply header-based auth for the request's service kind.
///
/// S4: `bearer_auth` is **only** applied for kinds that actually use a bearer
/// token — never for Plex (token travels in the query string) or Jellyfin (uses
/// the MediaBrowser / X-Emby-Token headers). This prevents leaking a bearer
/// `Authorization` header alongside the real credential.
pub fn apply_auth(
    mut request: reqwest::RequestBuilder,
    service: &ServiceConfig,
) -> reqwest::RequestBuilder {
    // No AuthStyle is bearer-only, so every arm returns `request` without ever
    // falling through to a trailing `bearer_auth` (S4: a bearer Authorization
    // header would leak alongside the real per-kind credential).
    match service.kind.descriptor().auth_style {
        AuthStyle::ApiKeyHeader => {
            if let Some(key) = service.api_key.as_deref() {
                request = request.header("X-Api-Key", key);
            }
            request
        }
        AuthStyle::JellyfinToken => {
            if let Some(key) = service.api_key.as_deref().or(service.token.as_deref()) {
                request = request
                    .header("Authorization", jellyfin_authorization(key))
                    .header("X-Emby-Token", key);
            }
            request
        }
        // Plex token is injected as the X-Plex-Token query param in build_url.
        AuthStyle::PlexToken => request,
        // apikey is injected in the query string / cookie session is established
        // separately.
        AuthStyle::QueryApiKey | AuthStyle::CookieSession => request,
    }
}

/// Establish a qBittorrent SID cookie session if username/password are set.
///
/// S1: this uses the caller-provided `qbit_client`, a dedicated cookie-store
/// `Client` separate from the shared default client. The SID therefore cannot
/// bleed onto other services that happen to share an upstream host.
pub async fn ensure_qbittorrent_session(
    qbit_client: &Client,
    service: &ServiceConfig,
) -> Result<()> {
    let Some(username) = service.username.as_deref() else {
        return Ok(());
    };
    let Some(password) = service.password.as_deref() else {
        return Ok(());
    };
    let url = build_url(service, "/api/v2/auth/login")?;
    let response = qbit_client
        .post(url)
        .form(&[("username", username), ("password", password)])
        .send()
        .await
        .with_context(|| format!("{} login failed", service.name))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .with_context(|| format!("{} login response body read failed", service.name))?;
    if !status.is_success() {
        anyhow::bail!("{} login returned HTTP {}", service.name, status.as_u16());
    }
    if !qbittorrent_login_accepted(status, &text) {
        return Err(super::UpstreamError::QbittorrentLoginRejected {
            service: service.name.clone(),
        }
        .into());
    }
    Ok(())
}

pub fn qbittorrent_login_accepted(status: StatusCode, text: &str) -> bool {
    status.is_success() && (status == StatusCode::NO_CONTENT || text.trim() == "Ok.")
}
