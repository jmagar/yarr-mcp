//! Transport-only HTTP client for upstream media services.
//!
//! This module is split into:
//!   * `rustarr.rs` (this file) — `RustarrClient` + the `request_json` core
//!   * [`auth`] — per-kind header auth + qBittorrent cookie session
//!   * [`helpers`] — URL building, query-string assembly, path validation,
//!     response slimming, log redaction
//!
//! Public path/url helpers are re-exported here so callers keep importing them
//! from `crate::rustarr`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use reqwest::{Client, Method, StatusCode};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};

pub mod auth;
pub mod helpers;

pub use helpers::{build_url, query_get, slim, validate_safe_path};

#[cfg(test)]
#[path = "rustarr_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct RustarrClient {
    /// Shared, cookie-less client for every service except qBittorrent.
    client: Client,
    /// Dedicated cookie-store client for qBittorrent so its SID cookie cannot
    /// bleed onto other services sharing an upstream host (S1).
    qbit_client: Client,
    /// Timestamp of the last successful qBittorrent login, per upstream host. The
    /// SID cookie is retained by `qbit_client`, so we only need to re-login when the
    /// cached session is stale (P1-2). Keyed by `ServiceConfig.base_url` — the host
    /// the shared cookie jar is scoped to, not the display name.
    qbit_sessions: Arc<Mutex<HashMap<String, Instant>>>,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum UpstreamError {
    #[error("{service} returned HTTP {} ({body_preview})", status.as_u16())]
    Http {
        service: String,
        status: StatusCode,
        body_preview: String,
    },
    #[error(
        "{service} returned non-JSON response (content-type: {}; body: {body_preview})",
        content_type.as_deref().unwrap_or("unknown")
    )]
    InvalidJson {
        service: String,
        content_type: Option<String>,
        body_preview: String,
    },
    #[error("{service} login rejected username/password")]
    QbittorrentLoginRejected { service: String },
}

/// Per-request upstream timeout. Defaults to 30s; override with
/// `RUSTARR_HTTP_TIMEOUT_SECS` for stacks with slow upstreams (e.g. a Prowlarr
/// that fans an `/indexer` read out to many trackers). A value of 0 or an
/// unparseable value falls back to the 30s default.
fn http_timeout() -> Duration {
    std::env::var("RUSTARR_HTTP_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(30))
}

impl RustarrClient {
    pub fn new(_cfg: &RustarrConfig) -> Result<Self> {
        let timeout = http_timeout();
        let client = Client::builder()
            .timeout(timeout)
            // L1-lang: bound the TCP connect phase and disable redirect-following
            // so a malicious/compromised upstream cannot redirect a credentialed
            // request to an attacker-controlled host.
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .pool_max_idle_per_host(0)
            // S1: the shared client carries no cookie jar, so no service can
            // inherit another's session cookie.
            .cookie_store(false)
            .build()
            .context("failed to build HTTP client")?;
        let qbit_client = Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .pool_max_idle_per_host(0)
            .cookie_store(true)
            .build()
            .context("failed to build qBittorrent HTTP client")?;
        Ok(Self {
            client,
            qbit_client,
            qbit_sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn get_json(&self, service: &ServiceConfig, path: &str) -> Result<Value> {
        self.request_json(Method::GET, service, path, None, None)
            .await
    }

    pub async fn post_json(
        &self,
        service: &ServiceConfig,
        path: &str,
        body: Value,
    ) -> Result<Value> {
        self.request_json(Method::POST, service, path, Some(body), None)
            .await
    }

    pub async fn put_json(
        &self,
        service: &ServiceConfig,
        path: &str,
        body: Value,
    ) -> Result<Value> {
        self.request_json(Method::PUT, service, path, Some(body), None)
            .await
    }

    pub async fn delete_json(
        &self,
        service: &ServiceConfig,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        self.request_json(Method::DELETE, service, path, body, None)
            .await
    }

    /// Core request path. `accept_mime` lets callers (e.g. Plex) negotiate a
    /// JSON response via the `Accept` header without the transport learning
    /// anything Plex-specific.
    pub async fn request_json(
        &self,
        method: Method,
        service: &ServiceConfig,
        path: &str,
        body: Option<Value>,
        accept_mime: Option<&str>,
    ) -> Result<Value> {
        let http = if service.kind == ServiceKind::Qbittorrent {
            auth::ensure_qbittorrent_session(&self.qbit_client, &self.qbit_sessions, service)
                .await?;
            &self.qbit_client
        } else {
            &self.client
        };
        let url = build_url(service, path)?;
        let mut request = http.request(method, url);
        request = auth::apply_auth(request, service);
        if let Some(accept) = accept_mime {
            request = request.header(reqwest::header::ACCEPT, accept);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        self.finish_with_retry(service, request).await
    }

    /// Send a request to a **pre-built URL** with any method + optional body.
    ///
    /// The generated-operation executor builds the URL itself (substituting and
    /// encoding path/query params via `helpers::build_operation_url`), so unlike
    /// [`request_json`](Self::request_json) this takes a finished `Url` and does
    /// not re-derive the path or enforce the per-kind allowlist. Auth headers,
    /// qBittorrent session handling, and the JSON/error parsing are shared.
    pub async fn request_url(
        &self,
        method: Method,
        service: &ServiceConfig,
        url: reqwest::Url,
        body: Option<Value>,
        accept_mime: Option<&str>,
    ) -> Result<Value> {
        let http = if service.kind == ServiceKind::Qbittorrent {
            auth::ensure_qbittorrent_session(&self.qbit_client, &self.qbit_sessions, service)
                .await?;
            &self.qbit_client
        } else {
            &self.client
        };
        let mut request = http.request(method, url);
        request = auth::apply_auth(request, service);
        if let Some(accept) = accept_mime {
            request = request.header(reqwest::header::ACCEPT, accept);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        self.finish_with_retry(service, request).await
    }

    pub async fn request_url_multipart_file(
        &self,
        method: Method,
        service: &ServiceConfig,
        url: reqwest::Url,
        field_name: &str,
        file_name: &str,
        bytes: Vec<u8>,
    ) -> Result<Value> {
        let http = if service.kind == ServiceKind::Qbittorrent {
            auth::ensure_qbittorrent_session(&self.qbit_client, &self.qbit_sessions, service)
                .await?;
            &self.qbit_client
        } else {
            &self.client
        };
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(file_name.to_string())
            .mime_str("application/zip")?;
        let form = reqwest::multipart::Form::new().part(field_name.to_string(), part);

        let mut request = http.request(method, url);
        request = auth::apply_auth(request, service).multipart(form);
        self.finish_with_retry(service, request).await
    }

    /// Send a pre-built request (used by query-style helpers) and parse it.
    pub async fn send_get(
        &self,
        service: &ServiceConfig,
        url: reqwest::Url,
        accept_mime: Option<&str>,
    ) -> Result<Value> {
        let http = if service.kind == ServiceKind::Qbittorrent {
            auth::ensure_qbittorrent_session(&self.qbit_client, &self.qbit_sessions, service)
                .await?;
            &self.qbit_client
        } else {
            &self.client
        };
        let mut request = http.get(url);
        request = auth::apply_auth(request, service);
        if let Some(accept) = accept_mime {
            request = request.header(reqwest::header::ACCEPT, accept);
        }
        self.finish_with_retry(service, request).await
    }

    /// Send a `application/x-www-form-urlencoded` POST to a pre-built URL.
    ///
    /// qBittorrent's WebUI API (`/api/v2/torrents/{add,start,stop,delete}`)
    /// consumes form fields, never JSON. This routes through the dedicated
    /// cookie-store client and establishes/refreshes the SID session first, so it
    /// is the qBittorrent counterpart to [`post_json`](Self::post_json). The
    /// `form` pairs are percent-encoded by reqwest, so callers never `format!`
    /// untrusted values into the body.
    pub async fn send_form_post(
        &self,
        service: &ServiceConfig,
        url: reqwest::Url,
        form: &[(&str, &str)],
    ) -> Result<Value> {
        let http = if service.kind == ServiceKind::Qbittorrent {
            auth::ensure_qbittorrent_session(&self.qbit_client, &self.qbit_sessions, service)
                .await?;
            &self.qbit_client
        } else {
            &self.client
        };
        let mut request = http.post(url).form(form);
        request = auth::apply_auth(request, service);
        self.finish_with_retry(service, request).await
    }

    /// Send a request, retrying once for qBittorrent if the cached SID was
    /// rejected upstream.
    ///
    /// A session cached within `auth::QBIT_SESSION_TTL` can still be invalid if
    /// qBittorrent expired it server-side (WebUI restart, session timeout, ban).
    /// Without this, every subsequent call would fast-path into the same 401/403
    /// until the TTL lapsed. On an auth failure we evict the cached session,
    /// force a fresh login, and retry the request exactly once. `try_clone`
    /// succeeds for the GET / form / JSON bodies used here (no streaming body);
    /// if it ever returns `None` we fall through to a single non-retried send.
    async fn finish_with_retry(
        &self,
        service: &ServiceConfig,
        request: reqwest::RequestBuilder,
    ) -> Result<Value> {
        if service.kind == ServiceKind::Qbittorrent {
            match request.try_clone() {
                Some(retry) => match self.finish(service, request).await {
                    Err(err) if is_auth_failure(&err) => {
                        auth::invalidate_qbittorrent_session(&self.qbit_sessions, service).await;
                        auth::ensure_qbittorrent_session(
                            &self.qbit_client,
                            &self.qbit_sessions,
                            service,
                        )
                        .await?;
                        return self.finish(service, retry).await;
                    }
                    result => return result,
                },
                // try_clone only returns None for a non-cloneable (streaming) body,
                // which the qBittorrent paths never use. Warn rather than fail
                // silently so a future caller that breaks the invariant is visible.
                None => {
                    tracing::warn!(
                        service = %service.name,
                        "qBittorrent request body is not cloneable; the 401/403 re-login retry is disabled for this call"
                    );
                }
            }
        }
        self.finish(service, request).await
    }

    async fn finish(
        &self,
        service: &ServiceConfig,
        request: reqwest::RequestBuilder,
    ) -> Result<Value> {
        let response = request
            .send()
            .await
            .with_context(|| format!("{} request failed", service.name))?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("{} response body read failed", service.name))?;
        let text = std::str::from_utf8(&bytes).ok();
        if !status.is_success() {
            return Err(UpstreamError::Http {
                service: service.name.clone(),
                status,
                body_preview: helpers::body_preview(text.unwrap_or("<non-utf8 body>")),
            }
            .into());
        }
        let Some(text) = text else {
            return Ok(serde_json::json!({
                "ok": true,
                "status": status.as_u16(),
                "contentType": content_type,
                "bytes": bytes.len()
            }));
        };
        if text.trim().is_empty() {
            return Ok(serde_json::json!({ "ok": true, "status": status.as_u16() }));
        }
        match serde_json::from_str(text) {
            Ok(value) => Ok(value),
            Err(_) if allows_text_response(service.kind) => Ok(Value::String(text.to_string())),
            Err(_) => Err(UpstreamError::InvalidJson {
                service: service.name.clone(),
                content_type,
                body_preview: helpers::body_preview(text),
            }
            .into()),
        }
    }
}

/// Whether an upstream error is an authentication rejection (401/403) — used to
/// trigger a single qBittorrent re-login-and-retry.
fn is_auth_failure(err: &anyhow::Error) -> bool {
    matches!(
        err.downcast_ref::<UpstreamError>(),
        Some(UpstreamError::Http { status, .. })
            if *status == StatusCode::UNAUTHORIZED || *status == StatusCode::FORBIDDEN
    )
}

fn allows_text_response(kind: ServiceKind) -> bool {
    crate::openapi::is_generated(kind)
        || matches!(kind, ServiceKind::Plex | ServiceKind::Qbittorrent)
}
