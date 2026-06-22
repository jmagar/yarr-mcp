use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::RustarrClient;

fn service_with(kinds: &[(&str, ServiceKind)]) -> RustarrService {
    let services = kinds
        .iter()
        .map(|(name, kind)| ServiceConfig {
            name: (*name).into(),
            kind: *kind,
            base_url: "http://localhost:1".into(),
            token: Some("test".into()),
            ..ServiceConfig::default()
        })
        .collect();
    let config = RustarrConfig { services };
    let client = RustarrClient::new(&config).expect("client builds");
    RustarrService::new(client, config)
}

#[tokio::test]
async fn sessions_rejects_non_media_kind() {
    // sonarr is ArrManager, not MediaServer — the capability check must reject it
    // before any request is built (wrong-capability reject).
    let svc = service_with(&[("sonarr", ServiceKind::Sonarr)]);
    let err = svc
        .media_sessions("sonarr")
        .await
        .expect_err("media_sessions on sonarr must be rejected");
    assert!(
        err.to_string().contains("MediaServer"),
        "error should mention the MediaServer capability, got: {err}"
    );
}

#[tokio::test]
async fn libraries_rejects_non_media_kind() {
    let svc = service_with(&[("sonarr", ServiceKind::Sonarr)]);
    let err = svc
        .media_libraries("sonarr")
        .await
        .expect_err("media_libraries on sonarr must be rejected");
    assert!(err.to_string().contains("MediaServer"), "got: {err}");
}

#[tokio::test]
async fn scan_rejects_non_media_kind() {
    // media_scan is non-destructive and ungated now, so the capability check is
    // the first gate: a non-media kind (sonarr) is rejected before any request is
    // built.
    let svc = service_with(&[("sonarr", ServiceKind::Sonarr)]);
    let err = svc
        .media_scan("sonarr", Some("3"))
        .await
        .expect_err("media_scan on sonarr must be rejected");
    assert!(err.to_string().contains("MediaServer"), "got: {err}");
}

#[tokio::test]
async fn plex_scan_requires_library_id() {
    // Plex refresh targets a specific section, so --library is mandatory; the
    // missing-library error surfaces after the capability check passes.
    let svc = service_with(&[("plex", ServiceKind::Plex)]);
    let err = svc
        .media_scan("plex", None)
        .await
        .expect_err("plex media_scan without --library must be rejected");
    assert!(
        err.to_string().contains("library"),
        "expected a missing-library error, got: {err}"
    );
}
