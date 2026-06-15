//! Mechanical transport helpers: URL building, query-string assembly, path
//! validation, response slimming, and log redaction.
//!
//! Everything here is shape-agnostic. The *choice* of which fields to keep
//! (`slim`) or which query params to send (`query_get`) is made in the app
//! layer; this module only provides the primitives. No business logic.

use anyhow::{Context, Result};
use reqwest::Url;
use serde_json::Value;

use crate::capability::AuthStyle;
use crate::config::{ServiceConfig, ServiceKind};

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod tests;

/// Build the upstream URL for `path`, validating it against the kind's allowlist
/// and injecting query-string auth (SABnzbd/Tautulli `apikey`, Plex
/// `X-Plex-Token`) for query-auth kinds.
///
/// This is the single owner of *query-string* auth injection. Header auth lives
/// in [`super::auth::apply_auth`]; the two never both append the api key.
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
    let needs_query_auth = matches!(
        service.kind.descriptor().auth_style,
        AuthStyle::QueryApiKey | AuthStyle::PlexToken
    );
    if !query_part.is_empty() || needs_query_auth {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query_part
            .split('&')
            .filter(|item| !item.is_empty())
            .filter_map(|item| item.split_once('='))
        {
            pairs.append_pair(key, value);
        }
        append_query_auth(&mut pairs, service);
    }
    Ok(url)
}

/// Inject query-string auth/quirks for query-auth kinds. SABnzbd additionally
/// forces `output=json`. Header-auth kinds are no-ops here.
fn append_query_auth(
    pairs: &mut url::form_urlencoded::Serializer<'_, url::UrlQuery<'_>>,
    service: &ServiceConfig,
) {
    match service.kind.descriptor().auth_style {
        AuthStyle::QueryApiKey => {
            if service.kind == ServiceKind::Sabnzbd {
                pairs.append_pair("output", "json");
            }
            if let Some(key) = service.api_key.as_deref() {
                pairs.append_pair("apikey", key);
            }
        }
        AuthStyle::PlexToken => {
            if let Some(token) = service.token.as_deref().or(service.api_key.as_deref()) {
                pairs.append_pair("X-Plex-Token", token);
            }
        }
        AuthStyle::ApiKeyHeader | AuthStyle::CookieSession | AuthStyle::JellyfinToken => {}
    }
}

/// Build a URL for a query-style API (SABnzbd `?mode=`, Tautulli `?cmd=`),
/// percent-encoding every param value via `append_pair`.
///
/// User-supplied text MUST reach upstream through this (or `build_url`), never
/// via `format!` into a path string — otherwise a value like
/// `"foo&monitored=false"` would inject a second query parameter (S6).
pub fn query_get(service: &ServiceConfig, base: &str, params: &[(&str, &str)]) -> Result<Url> {
    validate_safe_path(base)?;
    validate_service_path(service.kind, base)?;
    let mut url = Url::parse(service.base_url.trim_end_matches('/'))
        .with_context(|| format!("{} base_url is invalid", service.name))?;
    url.set_path(&format!(
        "{}/{}",
        url.path().trim_end_matches('/'),
        base.trim_start_matches('/')
    ));
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in params {
            pairs.append_pair(key, value);
        }
        append_query_auth(&mut pairs, service);
    }
    Ok(url)
}

/// Field-selection over a JSON value. Given an object, keep only `keep_fields`;
/// given an array, slim each element; otherwise return the value unchanged.
///
/// Mechanical only — the caller decides which fields matter.
pub fn slim(value: Value, keep_fields: &[&str]) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| slim(item, keep_fields))
                .collect(),
        ),
        Value::Object(map) => {
            let mut kept = serde_json::Map::new();
            for field in keep_fields {
                if let Some(v) = map.get(*field) {
                    kept.insert((*field).to_string(), v.clone());
                }
            }
            Value::Object(kept)
        }
        other => other,
    }
}

/// Truncated, secret-redacted preview of a response body for error messages.
pub fn body_preview(text: &str) -> String {
    let mut preview: String = text
        .chars()
        .filter(|ch| !ch.is_control() || ch.is_whitespace())
        .take(160)
        .collect();
    for needle in [
        "apikey=",
        "api_key=",
        "token=",
        "x-plex-token=",
        "x-emby-token=",
    ] {
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

/// Reject traversal, absolute URLs, encoded separators, and inline secrets.
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

/// Enforce the per-kind path allowlist (from `KindDescriptor.path_allowlist`).
///
/// The allowlist keeps strict v1/v3 separation so e.g. Lidarr (v1) cannot reach
/// `/api/v3/*` (S7).
pub fn validate_service_path(kind: ServiceKind, path: &str) -> Result<()> {
    let path_part = path.split_once('?').map(|(path, _)| path).unwrap_or(path);
    let allowed = kind.descriptor().path_allowlist;
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
