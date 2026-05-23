//! Transport-only HTTP client for upstream media services.

use anyhow::{Context, Result};
use reqwest::{Client, Method, Url};
use serde_json::Value;

use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};

#[cfg(test)]
#[path = "rustarr_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct RustarrClient {
    client: Client,
}

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
        let text = response
            .text()
            .await
            .with_context(|| format!("{} response body read failed", service.name))?;
        if !status.is_success() {
            anyhow::bail!("{} returned HTTP {}", service.name, status.as_u16());
        }
        serde_json::from_str(&text).or_else(|_| Ok(Value::String(text)))
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
        if !response.status().is_success() {
            anyhow::bail!(
                "{} login returned HTTP {}",
                service.name,
                response.status().as_u16()
            );
        }
        Ok(())
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
    if path
        .split(['/', '?', '&'])
        .any(|segment| segment == ".." || segment.eq_ignore_ascii_case("%2e%2e"))
    {
        anyhow::bail!("path must not contain parent directory segments");
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
