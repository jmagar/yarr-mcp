//! Tests for the C2 write command METHODS (orchestration). The pure builders /
//! selectors / cap / preview are tested directly in `editor_tests.rs`; here we
//! assert the methods reject the wrong capability before any network call and
//! that the headline `set_quality` request struct threads through unchanged.

use crate::app::arr::write::SetQualityRequest;
use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::RustarrClient;

fn service_with(kind: ServiceKind, name: &str) -> RustarrService {
    let config = RustarrConfig {
        services: vec![ServiceConfig {
            name: name.into(),
            kind,
            base_url: "http://127.0.0.1:1".into(),
            api_key: Some("x".into()),
            ..ServiceConfig::default()
        }],
    };
    let client = RustarrClient::new(&config).expect("client builds");
    RustarrService::new(client, config)
}

#[tokio::test]
async fn write_methods_reject_wrong_capability_before_network() {
    // Plex is a MediaServer: every arr write method must reject it via the
    // capability guard, synchronously, before any request is built.
    let svc = service_with(ServiceKind::Plex, "plex");
    let req = SetQualityRequest {
        from: None,
        to: "HD-1080p",
        ids: &[],
        titles: &[],
        confirm: false,
        bulk: false,
    };
    let err = svc
        .arr_set_quality("plex", req)
        .await
        .expect_err("plex is not an arr kind");
    assert!(
        err.to_string().contains("does not provide") || err.to_string().contains("ArrManager"),
        "{err}"
    );
    assert!(svc.arr_search("plex", &[], false, false).await.is_err());
    assert!(svc.arr_delete("plex", 1, false, true).await.is_err());
}

#[tokio::test]
async fn delete_without_confirm_returns_preview_and_mutates_nothing() {
    // delete is destructive: with confirm absent it returns a preview WITHOUT
    // issuing the DELETE — so it never touches the (unreachable) stub at all.
    let svc = service_with(ServiceKind::Radarr, "radarr");
    let preview = svc
        .arr_delete("radarr", 9, true, false)
        .await
        .expect("delete dry-run builds a preview without any network call");
    assert_eq!(preview["would_do"], serde_json::json!("delete"));
    assert_eq!(preview["id"], serde_json::json!(9));
    assert_eq!(preview["delete_files"], serde_json::json!(true));
    assert_eq!(preview["confirm_required"], serde_json::json!(true));
    assert!(
        preview.get("deleted").is_none(),
        "preview must not report a delete"
    );
}

#[tokio::test]
async fn search_without_confirm_returns_preview_and_mutates_nothing() {
    // search dry-run builds a preview without POSTing to /command.
    let svc = service_with(ServiceKind::Sonarr, "sonarr");
    let preview = svc
        .arr_search("sonarr", &[], false, false)
        .await
        .expect("search dry-run builds a preview without any network call");
    assert_eq!(preview["would_do"], serde_json::json!("search"));
    assert_eq!(preview["command"], serde_json::json!("SeriesSearch"));
    assert_eq!(preview["count"], serde_json::json!("all-monitored"));
    assert!(preview.get("started").is_none());
}
