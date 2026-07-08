use crate::app::YarrService;
use crate::config::{ServiceConfig, ServiceKind, YarrConfig};
use crate::yarr::YarrClient;

fn service_with(kinds: &[(&str, ServiceKind)]) -> YarrService {
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
    let config = YarrConfig { services };
    let client = YarrClient::new(&config).expect("client builds");
    YarrService::new(client, config)
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
async fn add_pause_resume_remove_reject_non_download_kind() {
    // All DownloadClient writes (including the destructive `remove`) run
    // immediately now, so the capability check is the first gate: a
    // non-download kind (plex) is rejected before any request is built.
    let svc = service_with(&[("plex", ServiceKind::Plex)]);
    for err in [
        svc.download_add("plex", "http://x/a.nzb").await,
        svc.download_pause("plex", Some("h")).await,
        svc.download_resume("plex", Some("h")).await,
        svc.download_remove("plex", "h", false).await,
    ] {
        let msg = err.expect_err("write op on plex must reject").to_string();
        assert!(msg.contains("DownloadClient"), "got: {msg}");
    }
}
