//! Per-service request authentication, driven by `KindDescriptor.auth_style`,
//! plus the qBittorrent cookie-session lifecycle.
//!
//! Header auth lives here; query-string auth (apikey / X-Plex-Token) lives in
//! [`super::helpers::build_url`]. The two never both append the api key.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use tokio::sync::Mutex;

use super::helpers::build_url;
use crate::capability::AuthStyle;
use crate::config::ServiceConfig;

/// How long a qBittorrent SID session is treated as fresh before we re-login.
/// qBittorrent's default WebUI session timeout is 3600s; 50 minutes keeps a
/// comfortable margin while skipping the per-request login (P1-2).
const QBIT_SESSION_TTL: Duration = Duration::from_secs(50 * 60);

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
///
/// P1-2: the SID cookie is retained by `qbit_client`, so a successful login is
/// cached in `sessions` (keyed by `base_url`, matching the host the cookie jar is
/// scoped to) and reused for [`QBIT_SESSION_TTL`]. We only re-POST
/// `/api/v2/auth/login` when the cached session is stale or absent. The lock is
/// held **only** to read/update the timestamp — never across the login `.await`.
///
/// Concurrency note: the lock is released before the login `.await` (required —
/// never hold a mutex across network I/O), so N requests arriving on a cold/expired
/// cache may each log in once before the first timestamp lands. This is benign for
/// single-instance home services (logins are idempotent) and only opens at startup
/// or once per TTL boundary; it is not single-flight by design.
pub async fn ensure_qbittorrent_session(
    qbit_client: &Client,
    sessions: &Arc<Mutex<HashMap<String, Instant>>>,
    service: &ServiceConfig,
) -> Result<()> {
    let Some(username) = service.username.as_deref() else {
        return Ok(());
    };
    let Some(password) = service.password.as_deref() else {
        return Ok(());
    };

    // Fast path: skip the login if we logged in recently. Lock scope ends here.
    // Keyed by `base_url` (the upstream origin) rather than the display name,
    // because the SID cookie lives in the shared `qbit_client` cookie jar scoped
    // to that host — so freshness must track the host, not the config alias.
    {
        let guard = sessions.lock().await;
        if let Some(last) = guard.get(&service.base_url) {
            if last.elapsed() < QBIT_SESSION_TTL {
                return Ok(());
            }
        }
    }

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
    // Record freshness so subsequent calls reuse the retained SID cookie. Lock
    // is taken only for this insert — not across the login above.
    sessions
        .lock()
        .await
        .insert(service.base_url.clone(), Instant::now());
    Ok(())
}

/// Evict a cached qBittorrent session so the next request forces a fresh login.
///
/// Called when an otherwise-fresh session is rejected upstream (401/403) — e.g.
/// the WebUI restarted or expired the SID before our TTL lapsed.
pub async fn invalidate_qbittorrent_session(
    sessions: &Arc<Mutex<HashMap<String, Instant>>>,
    service: &ServiceConfig,
) {
    sessions.lock().await.remove(&service.base_url);
}

pub fn qbittorrent_login_accepted(status: StatusCode, text: &str) -> bool {
    status.is_success() && (status == StatusCode::NO_CONTENT || text.trim() == "Ok.")
}
