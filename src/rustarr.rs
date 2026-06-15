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

use anyhow::{Context, Result};
use reqwest::{Client, Method, StatusCode};
use serde_json::Value;

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
}

#[derive(Debug)]
pub enum UpstreamError {
    Http {
        service: String,
        status: StatusCode,
        body_preview: String,
    },
    InvalidJson {
        service: String,
        content_type: Option<String>,
        body_preview: String,
    },
    QbittorrentLoginRejected {
        service: String,
    },
}

impl std::fmt::Display for UpstreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http {
                service,
                status,
                body_preview,
            } => write!(
                f,
                "{service} returned HTTP {} ({body_preview})",
                status.as_u16()
            ),
            Self::InvalidJson {
                service,
                content_type,
                body_preview,
            } => write!(
                f,
                "{service} returned non-JSON response (content-type: {}; body: {body_preview})",
                content_type.as_deref().unwrap_or("unknown")
            ),
            Self::QbittorrentLoginRejected { service } => {
                write!(f, "{service} login rejected username/password")
            }
        }
    }
}

impl std::error::Error for UpstreamError {}

impl RustarrClient {
    pub fn new(_cfg: &RustarrConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            // S1: the shared client carries no cookie jar, so no service can
            // inherit another's session cookie.
            .cookie_store(false)
            .build()
            .context("failed to build HTTP client")?;
        let qbit_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .cookie_store(true)
            .build()
            .context("failed to build qBittorrent HTTP client")?;
        Ok(Self {
            client,
            qbit_client,
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
            auth::ensure_qbittorrent_session(&self.qbit_client, service).await?;
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
        self.finish(service, request).await
    }

    /// Send a pre-built request (used by query-style helpers) and parse it.
    pub async fn send_get(
        &self,
        service: &ServiceConfig,
        url: reqwest::Url,
        accept_mime: Option<&str>,
    ) -> Result<Value> {
        let http = if service.kind == ServiceKind::Qbittorrent {
            auth::ensure_qbittorrent_session(&self.qbit_client, service).await?;
            &self.qbit_client
        } else {
            &self.client
        };
        let mut request = http.get(url);
        request = auth::apply_auth(request, service);
        if let Some(accept) = accept_mime {
            request = request.header(reqwest::header::ACCEPT, accept);
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
        let text = response
            .text()
            .await
            .with_context(|| format!("{} response body read failed", service.name))?;
        if !status.is_success() {
            return Err(UpstreamError::Http {
                service: service.name.clone(),
                status,
                body_preview: helpers::body_preview(&text),
            }
            .into());
        }
        if text.trim().is_empty() {
            return Ok(serde_json::json!({ "ok": true, "status": status.as_u16() }));
        }
        match serde_json::from_str(&text) {
            Ok(value) => Ok(value),
            Err(_) if allows_text_response(service.kind) => Ok(Value::String(text)),
            Err(_) => Err(UpstreamError::InvalidJson {
                service: service.name.clone(),
                content_type,
                body_preview: helpers::body_preview(&text),
            }
            .into()),
        }
    }
}

fn allows_text_response(kind: ServiceKind) -> bool {
    matches!(kind, ServiceKind::Plex | ServiceKind::Qbittorrent)
}
