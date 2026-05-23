//! Transport-only HTTP client for upstream media services.

use anyhow::{Context, Result};
use reqwest::{Client, Method, StatusCode, Url};
use serde_json::Value;

use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};

#[cfg(test)]
#[path = "rustarr_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct RustarrClient {
    client: Client,
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
            .cookie_store(true)
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { client })
    }

    pub async fn get_json(&self, service: &ServiceConfig, path: &str) -> Result<Value> {
        self.request_json(Method::GET, service, path, None).await
    }

    pub async fn post_json(
        &self,
        service: &ServiceConfig,
        path: &str,
        body: Value,
    ) -> Result<Value> {
        self.request_json(Method::POST, service, path, Some(body))
            .await
    }

    async fn request_json(
        &self,
        method: Method,
        service: &ServiceConfig,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        if service.kind == ServiceKind::Qbittorrent {
            self.ensure_qbittorrent_session(service).await?;
        }
        let url = build_url(service, path)?;
        let mut request = self.client.request(method, url);
        request = apply_auth(request, service);
        if let Some(body) = body {
            request = request.json(&body);
        }
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
                body_preview: body_preview(&text),
            }
            .into());
        }
        match serde_json::from_str(&text) {
            Ok(value) => Ok(value),
            Err(_) if allows_text_response(service.kind) => Ok(Value::String(text)),
            Err(_) => Err(UpstreamError::InvalidJson {
                service: service.name.clone(),
                content_type,
                body_preview: body_preview(&text),
            }
            .into()),
        }
    }

    async fn ensure_qbittorrent_session(&self, service: &ServiceConfig) -> Result<()> {
        let Some(username) = service.username.as_deref() else {
            return Ok(());
        };
        let Some(password) = service.password.as_deref() else {
            return Ok(());
        };
        let url = build_url(service, "/api/v2/auth/login")?;
        let response = self
            .client
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
        if text.trim() != "Ok." {
            return Err(UpstreamError::QbittorrentLoginRejected {
                service: service.name.clone(),
            }
            .into());
        }
        Ok(())
    }
}

fn allows_text_response(kind: ServiceKind) -> bool {
    matches!(kind, ServiceKind::Plex | ServiceKind::Qbittorrent)
}

fn body_preview(text: &str) -> String {
    let mut preview: String = text
        .chars()
        .filter(|ch| !ch.is_control() || ch.is_whitespace())
        .take(160)
        .collect();
    for needle in ["apikey=", "api_key=", "token=", "X-Plex-Token="] {
        while let Some(index) = preview
            .to_ascii_lowercase()
            .find(&needle.to_ascii_lowercase())
        {
            let end = preview[index..]
                .find(['&', ' ', '\n', '\r'])
                .map(|offset| index + offset)
                .unwrap_or(preview.len());
            preview.replace_range(index..end, "[redacted]");
        }
    }
    if preview.trim().is_empty() {
        "<empty body>".into()
    } else {
        preview
    }
}

fn apply_auth(
    mut request: reqwest::RequestBuilder,
    service: &ServiceConfig,
) -> reqwest::RequestBuilder {
    match service.kind {
        ServiceKind::Sonarr
        | ServiceKind::Radarr
        | ServiceKind::Prowlarr
        | ServiceKind::Lidarr
        | ServiceKind::Readarr
        | ServiceKind::Overseerr
        | ServiceKind::Bazarr
        | ServiceKind::Tracearr
        | ServiceKind::Wizarr
        | ServiceKind::Notifiarr => {
            if let Some(key) = service.api_key.as_deref() {
                request = request.header("X-Api-Key", key);
            }
        }
        ServiceKind::Jellyfin => {
            if let Some(key) = service.api_key.as_deref().or(service.token.as_deref()) {
                request = request.header("X-Emby-Token", key);
            }
        }
        ServiceKind::Qbittorrent
        | ServiceKind::Sabnzbd
        | ServiceKind::Tautulli
        | ServiceKind::Plex => {}
    }
    if let Some(token) = service.token.as_deref() {
        request = request.bearer_auth(token);
    }
    request
}

pub fn build_url(service: &ServiceConfig, path: &str) -> Result<Url> {
    validate_safe_path(path)?;
    validate_service_path(service.kind, path)?;
    let mut url = Url::parse(service.base_url.trim_end_matches('/'))
        .with_context(|| format!("{} base_url is invalid", service.name))?;
    let (path_part, query_part) = path.split_once('?').unwrap_or((path, ""));
    url.set_path(&format!(
        "{}/{}",
        url.path().trim_end_matches('/'),
        path_part.trim_start_matches('/')
    ));
    if !query_part.is_empty()
        || matches!(
            service.kind,
            ServiceKind::Sabnzbd | ServiceKind::Tautulli | ServiceKind::Plex
        )
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query_part
            .split('&')
            .filter(|item| !item.is_empty())
            .filter_map(|item| item.split_once('='))
        {
            pairs.append_pair(key, value);
        }
        match service.kind {
            ServiceKind::Sabnzbd => {
                pairs.append_pair("output", "json");
                if let Some(key) = service.api_key.as_deref() {
                    pairs.append_pair("apikey", key);
                }
            }
            ServiceKind::Tautulli => {
                if let Some(key) = service.api_key.as_deref() {
                    pairs.append_pair("apikey", key);
                }
            }
            ServiceKind::Plex => {
                if let Some(token) = service.token.as_deref().or(service.api_key.as_deref()) {
                    pairs.append_pair("X-Plex-Token", token);
                }
            }
            _ => {}
        }
    }
    Ok(url)
}

pub fn validate_safe_path(path: &str) -> Result<()> {
    if path.trim().is_empty() {
        anyhow::bail!("path is required");
    }
    if path.starts_with("http://") || path.starts_with("https://") || path.starts_with("//") {
        anyhow::bail!("path must be relative to the configured service base_url");
    }
    for segment in path.split(['/', '?', '&']) {
        let decoded = percent_decode(segment)?;
        if segment == ".." || decoded == ".." {
            anyhow::bail!("path must not contain parent directory segments");
        }
        if decoded.contains('/') || decoded.contains('\\') {
            anyhow::bail!("path must not contain encoded path separators");
        }
    }
    let lower = path.to_ascii_lowercase();
    for secret_name in ["apikey=", "api_key=", "token=", "x-plex-token="] {
        if lower.contains(secret_name) {
            anyhow::bail!(
                "path must not include query-string secrets; configure credentials instead"
            );
        }
    }
    Ok(())
}

fn validate_service_path(kind: ServiceKind, path: &str) -> Result<()> {
    let path_part = path.split_once('?').map(|(path, _)| path).unwrap_or(path);
    const ARR_V3: &[&str] = &["/api/v3"];
    const ARR_V1: &[&str] = &["/api/v1"];
    const API: &[&str] = &["/api", "/api/v2"];
    const QBIT: &[&str] = &["/api/v2"];
    const JELLYFIN: &[&str] = &["/System", "/Items", "/Users", "/Library"];
    const PLEX: &[&str] = &["/identity", "/library", "/status", "/servers"];
    let allowed = match kind {
        ServiceKind::Sonarr | ServiceKind::Radarr => ARR_V3,
        ServiceKind::Prowlarr | ServiceKind::Lidarr | ServiceKind::Readarr => ARR_V1,
        ServiceKind::Overseerr => ARR_V1,
        ServiceKind::Sabnzbd
        | ServiceKind::Tautulli
        | ServiceKind::Bazarr
        | ServiceKind::Tracearr
        | ServiceKind::Wizarr
        | ServiceKind::Notifiarr => API,
        ServiceKind::Qbittorrent => QBIT,
        ServiceKind::Jellyfin => JELLYFIN,
        ServiceKind::Plex => PLEX,
    };
    if allowed.iter().any(|prefix| {
        path_part == *prefix
            || path_part
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    }) {
        Ok(())
    } else {
        anyhow::bail!(
            "path is outside the allowed API prefixes for {}",
            kind.as_str()
        )
    }
}

fn percent_decode(segment: &str) -> Result<String> {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                anyhow::bail!("path contains invalid percent encoding");
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3])
                .map_err(|_| anyhow::anyhow!("path contains invalid percent encoding"))?;
            let value = u8::from_str_radix(hex, 16)
                .map_err(|_| anyhow::anyhow!("path contains invalid percent encoding"))?;
            out.push(value);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| anyhow::anyhow!("path contains invalid UTF-8 encoding"))
}
