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

/// Drive an async op to completion on a fresh current-thread runtime so a sync
/// test can hold [`crate::testing::ENV_LOCK`] across the run (the destructive gate
/// reads `YARR_ALLOW_DESTRUCTIVE`) without holding the lock across `.await`.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
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
async fn add_pause_resume_reject_non_download_kind() {
    // add/pause/resume are non-destructive and ungated now, so the capability
    // check is the first gate: a non-download kind (plex) is rejected before any
    // request is built (proving no confirm gate precedes the capability check).
    let svc = service_with(&[("plex", ServiceKind::Plex)]);
    for err in [
        svc.download_add("plex", "http://x/a.nzb").await,
        svc.download_pause("plex", Some("h")).await,
        svc.download_resume("plex", Some("h")).await,
    ] {
        let msg = err.expect_err("write op on plex must reject").to_string();
        assert!(msg.contains("DownloadClient"), "got: {msg}");
    }
}

// Sync (not `#[tokio::test]`) so it can hold the process-wide `ENV_LOCK` across the
// whole run without tripping clippy's `await_holding_lock`: the destructive gate
// reads `YARR_ALLOW_DESTRUCTIVE`, and a concurrent env-mutating test setting it
// truthy would otherwise lift the gate and race this assertion to "request failed".
#[test]
fn remove_requires_confirm() {
    let _g = crate::testing::ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::remove_var("YARR_ALLOW_DESTRUCTIVE") };
    // remove is destructive, so the confirm gate runs before the capability /
    // transport: an unreachable qbittorrent still surfaces the confirm error first.
    let svc = service_with(&[("qbittorrent", ServiceKind::Qbittorrent)]);
    let err = block_on(svc.download_remove("qbittorrent", "h", false, false))
        .expect_err("download_remove without confirm must be rejected");
    assert!(err.to_string().contains("confirm"), "got: {err}");
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
