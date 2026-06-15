use super::{arr_path, arr_resource_noun, LIST_FIELDS};
use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::{slim, RustarrClient};
use serde_json::json;

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

#[test]
fn sonarr_uses_series_noun_under_v3() {
    assert_eq!(arr_resource_noun(ServiceKind::Sonarr), "series");
    assert_eq!(arr_path(ServiceKind::Sonarr, "series"), "/api/v3/series");
    assert_eq!(
        arr_path(ServiceKind::Sonarr, "qualityprofile"),
        "/api/v3/qualityprofile"
    );
}

#[test]
fn radarr_uses_movie_noun_under_v3() {
    assert_eq!(arr_resource_noun(ServiceKind::Radarr), "movie");
    assert_eq!(arr_path(ServiceKind::Radarr, "movie"), "/api/v3/movie");
}

#[test]
fn lidarr_uses_artist_noun_under_v1() {
    // C3: same `arr_path`/`arr_resource_noun` code, but the descriptor drives the
    // v1 prefix and `artist` noun — no hardcoded `/api/v3` or `series` leaks in.
    assert_eq!(arr_resource_noun(ServiceKind::Lidarr), "artist");
    assert_eq!(arr_path(ServiceKind::Lidarr, "artist"), "/api/v1/artist");
    assert_eq!(
        arr_path(ServiceKind::Lidarr, "qualityprofile"),
        "/api/v1/qualityprofile"
    );
}

#[test]
fn readarr_uses_author_noun_under_v1() {
    assert_eq!(arr_resource_noun(ServiceKind::Readarr), "author");
    assert_eq!(arr_path(ServiceKind::Readarr, "author"), "/api/v1/author");
    assert_eq!(
        arr_path(ServiceKind::Readarr, "qualityprofile"),
        "/api/v1/qualityprofile"
    );
}

#[test]
fn list_path_differs_by_kind() {
    // Same code, different resource noun resolved from the descriptor.
    let sonarr = arr_path(ServiceKind::Sonarr, arr_resource_noun(ServiceKind::Sonarr));
    let radarr = arr_path(ServiceKind::Radarr, arr_resource_noun(ServiceKind::Radarr));
    assert_eq!(sonarr, "/api/v3/series");
    assert_eq!(radarr, "/api/v3/movie");
    assert_ne!(sonarr, radarr);
}

#[test]
fn list_slim_keeps_only_expected_fields() {
    let raw = json!([{
        "id": 1,
        "title": "Example",
        "qualityProfileId": 4,
        "monitored": true,
        "sizeOnDisk": 12345,
        "status": "continuing",
        "added": "2020-01-01T00:00:00Z",
        "overview": "a very long overview that should be dropped",
        "images": ["dropme"],
        "seasons": [{"big": "object"}]
    }]);
    let slimmed = slim(raw, LIST_FIELDS);
    let row = &slimmed[0];
    for keep in LIST_FIELDS {
        assert!(row.get(keep).is_some(), "slim should keep {keep}");
    }
    assert!(row.get("overview").is_none(), "slim should drop overview");
    assert!(row.get("images").is_none(), "slim should drop images");
    assert!(row.get("seasons").is_none(), "slim should drop seasons");
}

#[tokio::test]
async fn arr_methods_reject_wrong_capability() {
    // Plex is a MediaServer, not an ArrManager: every arr read method must reject
    // it BEFORE attempting any network call (the capability guard fails synchronously).
    let svc = service_with(&[("plex", ServiceKind::Plex)]);
    let err = svc
        .arr_list("plex")
        .await
        .expect_err("plex is not an arr kind");
    let msg = err.to_string();
    assert!(
        msg.contains("ArrManager") || msg.contains("does not provide"),
        "error should explain the capability mismatch: {msg}"
    );
    assert!(svc.arr_quality_profiles("plex").await.is_err());
    assert!(svc.arr_health("plex").await.is_err());
}

#[tokio::test]
async fn arr_methods_accept_arr_capability_service() {
    // sonarr/radarr resolve past the capability guard (the network call then
    // fails against the unreachable stub URL, which is expected and not asserted).
    let svc = service_with(&[
        ("sonarr", ServiceKind::Sonarr),
        ("radarr", ServiceKind::Radarr),
    ]);
    // The capability guard passes; the only error possible is a transport error.
    for service in ["sonarr", "radarr"] {
        let result = svc.arr_list(service).await;
        if let Err(err) = result {
            let msg = err.to_string();
            assert!(
                !msg.contains("does not provide"),
                "{service} should pass the capability guard, got: {msg}"
            );
        }
    }
}
