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
fn appends_sabnzbd_query_auth() {
    let url = build_url(&svc(ServiceKind::Sabnzbd), "/api?mode=version").unwrap();
    assert!(url.as_str().contains("mode=version"));
    assert!(url.as_str().contains("output=json"));
    assert!(url.as_str().contains("apikey=key"));
}

#[test]
fn accepts_qbittorrent_login_success_variants() {
    assert!(qbittorrent_login_accepted(StatusCode::OK, "Ok."));
    assert!(qbittorrent_login_accepted(StatusCode::OK, " Ok.\n"));
    assert!(qbittorrent_login_accepted(StatusCode::NO_CONTENT, ""));
    assert!(!qbittorrent_login_accepted(StatusCode::OK, "Fails."));
    assert!(!qbittorrent_login_accepted(StatusCode::UNAUTHORIZED, "Ok."));
}

#[test]
fn all_required_service_kinds_are_unique() {
    let mut names = ServiceKind::ALL.map(ServiceKind::as_str).to_vec();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 15);
    assert!(names.contains(&"tautulli"));
}
