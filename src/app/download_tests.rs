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
            api_key: Some("test".into()),
            ..ServiceConfig::default()
        })
        .collect();
    let config = RustarrConfig { services };
    let client = RustarrClient::new(&config).expect("client builds");
    RustarrService::new(client, config)
}

#[tokio::test]
async fn queue_rejects_non_download_kind() {
    // plex is MediaServer, not DownloadClient — the capability check must reject
    // it before any request is built (wrong-capability reject).
    let svc = service_with(&[("plex", ServiceKind::Plex)]);
    let err = svc
        .download_queue("plex")
        .await
        .expect_err("download_queue on plex must be rejected");
    assert!(
        err.to_string().contains("DownloadClient"),
        "error should mention the DownloadClient capability, got: {err}"
    );
}

#[tokio::test]
async fn add_requires_confirm() {
    // The write guard runs before the capability/transport, so an unreachable
    // sabnzbd still surfaces the confirm error first.
    let svc = service_with(&[("sabnzbd", ServiceKind::Sabnzbd)]);
    let err = svc
        .download_add("sabnzbd", "http://x/a.nzb", false)
        .await
        .expect_err("download_add without confirm must be rejected");
    assert!(err.to_string().contains("confirm"), "got: {err}");
}

#[tokio::test]
async fn pause_resume_remove_require_confirm() {
    let svc = service_with(&[("qbittorrent", ServiceKind::Qbittorrent)]);
    for err in [
        svc.download_pause("qbittorrent", Some("h"), false).await,
        svc.download_resume("qbittorrent", Some("h"), false).await,
        svc.download_remove("qbittorrent", "h", false, false).await,
    ] {
        let msg = err
            .expect_err("write op without confirm must reject")
            .to_string();
        assert!(
            msg.contains("confirm"),
            "expected confirm error, got: {msg}"
        );
    }
}

#[tokio::test]
async fn write_ops_reject_non_download_kind_even_with_confirm() {
    // With confirm=true the next gate is the capability check, which must reject a
    // non-download kind before any request is built.
    let svc = service_with(&[("plex", ServiceKind::Plex)]);
    let err = svc
        .download_remove("plex", "h", false, true)
        .await
        .expect_err("download_remove on plex must be rejected");
    assert!(err.to_string().contains("DownloadClient"), "got: {err}");
}
