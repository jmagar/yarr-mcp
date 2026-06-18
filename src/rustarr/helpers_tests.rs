use super::*;
use crate::config::{ServiceConfig, ServiceKind};

fn svc(kind: ServiceKind) -> ServiceConfig {
    ServiceConfig {
        name: kind.as_str().into(),
        kind,
        base_url: "http://localhost:8989".into(),
        api_key: Some("key".into()),
        token: Some("token".into()),
        ..ServiceConfig::default()
    }
}

#[test]
fn rejects_unsafe_paths() {
    assert!(validate_safe_path("").is_err());
    assert!(validate_safe_path("https://evil.test/api").is_err());
    assert!(validate_safe_path("/api/../config").is_err());
    assert!(validate_safe_path("/api/%2e%2e/config").is_err());
    assert!(validate_safe_path("/api/%2fconfig").is_err());
    assert!(validate_safe_path("/api?apikey=secret").is_err());
}

#[test]
fn rejects_service_paths_outside_allowed_prefixes() {
    assert!(build_url(&svc(ServiceKind::Sonarr), "/api/v1/system/status").is_err());
    assert!(build_url(&svc(ServiceKind::Sonarr), "/api/v30/system/status").is_err());
    assert!(build_url(&svc(ServiceKind::Sabnzbd), "/api2").is_err());
    assert!(build_url(&svc(ServiceKind::Qbittorrent), "/api/v3/system/status").is_err());
}

#[test]
fn allows_exact_prefixes_and_prefix_path_boundaries() {
    assert!(build_url(&svc(ServiceKind::Sonarr), "/api/v3").is_ok());
    assert!(build_url(&svc(ServiceKind::Sonarr), "/api/v3/system/status").is_ok());
    assert!(build_url(&svc(ServiceKind::Sabnzbd), "/api?mode=version").is_ok());
}

#[test]
fn jellyfin_sessions_path_is_allowed() {
    // C6: /Sessions must be reachable for Jellyfin.
    assert!(build_url(&svc(ServiceKind::Jellyfin), "/Sessions").is_ok());
    assert!(build_url(&svc(ServiceKind::Jellyfin), "/System/Info/Public").is_ok());
}

#[test]
fn builds_arr_url_without_secret_in_path() {
    let url = build_url(&svc(ServiceKind::Sonarr), "/api/v3/system/status").unwrap();
    assert_eq!(url.as_str(), "http://localhost:8989/api/v3/system/status");
}

#[test]
fn allows_tracearr_health_status_path() {
    let url = build_url(&svc(ServiceKind::Tracearr), "/health").unwrap();
    assert_eq!(url.as_str(), "http://localhost:8989/health");
}

#[test]
fn allows_tracearr_api_v1_paths() {
    let url = build_url(&svc(ServiceKind::Tracearr), "/api/v1/servers").unwrap();
    assert_eq!(url.as_str(), "http://localhost:8989/api/v1/servers");
    assert!(build_url(&svc(ServiceKind::Tracearr), "/api/v2/servers").is_err());
}

#[test]
fn appends_sabnzbd_query_auth() {
    let url = build_url(&svc(ServiceKind::Sabnzbd), "/api?mode=version").unwrap();
    assert!(url.as_str().contains("mode=version"));
    assert!(url.as_str().contains("output=json"));
    assert!(url.as_str().contains("apikey=key"));
}

#[test]
fn build_url_does_not_double_encode_query_values() {
    // An already-encoded value (foo%20bar) must round-trip to a single encoding,
    // not foo%2520bar.
    let url = build_url(&svc(ServiceKind::Sonarr), "/api/v3/series?q=foo%20bar").unwrap();
    assert!(
        url.query_pairs().any(|(k, v)| k == "q" && v == "foo bar"),
        "got: {url}"
    );
    assert!(!url.as_str().contains("%2520"), "double-encoded: {url}");
}

#[test]
fn build_url_preserves_key_only_flag() {
    // A key-only flag (?flag) must survive query reconstruction.
    let url = build_url(&svc(ServiceKind::Sonarr), "/api/v3/series?flag").unwrap();
    assert!(
        url.query_pairs().any(|(k, v)| k == "flag" && v.is_empty()),
        "key-only flag dropped: {url}"
    );
}

#[test]
fn appends_plex_token_in_query_only() {
    let url = build_url(&svc(ServiceKind::Plex), "/identity").unwrap();
    assert!(url.as_str().contains("X-Plex-Token=token"));
}

#[test]
fn query_get_percent_encodes_param_values() {
    // S6: an injection-style value must be percent-encoded, not a second param.
    let url = query_get(
        &svc(ServiceKind::Tautulli),
        "/api/v2",
        &[("cmd", "get_history"), ("search", "foo&monitored=false")],
    )
    .unwrap();
    let s = url.as_str();
    assert!(s.contains("search=foo%26monitored%3Dfalse"), "got: {s}");
    // Must NOT have leaked a real `monitored` query parameter.
    assert!(
        !url.query_pairs().any(|(k, _)| k == "monitored"),
        "injection leaked a monitored param: {s}"
    );
    // Tautulli apikey is injected by the helper, not the caller.
    assert!(url.query_pairs().any(|(k, v)| k == "apikey" && v == "key"));
}

#[test]
fn query_get_appends_sabnzbd_output_json() {
    let url = query_get(&svc(ServiceKind::Sabnzbd), "/api", &[("mode", "queue")]).unwrap();
    assert!(url.query_pairs().any(|(k, v)| k == "output" && v == "json"));
    assert!(url.query_pairs().any(|(k, v)| k == "apikey" && v == "key"));
}

#[test]
fn slim_keeps_only_requested_fields_on_object() {
    let value = serde_json::json!({ "id": 1, "title": "x", "secret": "s" });
    let out = slim(value, &["id", "title"]);
    assert_eq!(out, serde_json::json!({ "id": 1, "title": "x" }));
}

#[test]
fn slim_maps_over_arrays() {
    let value = serde_json::json!([
        { "id": 1, "x": 9 },
        { "id": 2, "x": 8 },
    ]);
    let out = slim(value, &["id"]);
    assert_eq!(out, serde_json::json!([{ "id": 1 }, { "id": 2 }]));
}

#[test]
fn slim_leaves_scalars_untouched() {
    assert_eq!(slim(serde_json::json!(7), &["id"]), serde_json::json!(7));
}

#[test]
fn body_preview_redacts_emby_token() {
    let preview = body_preview("error x-emby-token=abc123 more");
    assert!(!preview.contains("abc123"), "got: {preview}");
    assert!(preview.contains("[redacted]"));
}

#[test]
fn body_preview_redacts_form_encoded_password_and_api_key() {
    // A form-encoded / query-string `password=` (qBittorrent's login form) and
    // `x-api-key=` must be redacted on the query pass, not just the JSON pass.
    let preview = body_preview("invalid request: username=admin&password=hunter2&x-api-key=sekret");
    assert!(!preview.contains("hunter2"), "password leaked: {preview}");
    assert!(!preview.contains("sekret"), "x-api-key leaked: {preview}");
    assert!(preview.contains("[redacted]"), "got: {preview}");
    // The non-secret username survives.
    assert!(preview.contains("admin"), "got: {preview}");
}

#[test]
fn body_preview_redacts_json_secrets() {
    // LOW-1: JSON-shaped `"key":"value"` secrets must be redacted too.
    let preview = body_preview(r#"{"apiKey":"abc123","status":"ok"}"#);
    assert!(!preview.contains("abc123"), "apiKey leaked: {preview}");
    assert!(preview.contains("[redacted]"), "got: {preview}");
    // Non-secret fields survive.
    assert!(
        preview.contains("status"),
        "lost non-secret field: {preview}"
    );
}

#[test]
fn body_preview_redacts_json_secrets_case_insensitive_and_spaced() {
    let preview = body_preview(r#"{ "Password" : "hunter2", "token": "t0k" }"#);
    assert!(!preview.contains("hunter2"), "password leaked: {preview}");
    assert!(!preview.contains("t0k"), "token leaked: {preview}");
    assert_eq!(preview.matches("[redacted]").count(), 2, "got: {preview}");
}

#[test]
fn body_preview_redacts_x_api_key_json() {
    let preview = body_preview(r#"{"x-api-key":"sekret"}"#);
    assert!(!preview.contains("sekret"), "got: {preview}");
    assert!(preview.contains("[redacted]"));
}

#[test]
fn body_preview_leaves_non_secret_json_untouched() {
    let preview = body_preview(r#"{"title":"My Movie","year":2020}"#);
    assert!(preview.contains("My Movie"), "got: {preview}");
    assert!(!preview.contains("[redacted]"), "over-redacted: {preview}");
}
