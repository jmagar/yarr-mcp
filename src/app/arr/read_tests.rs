use super::{
    ArrListOptions, LIST_FIELDS, QUALITY_PROFILE_FIELDS, QUEUE_FIELDS, arr_path, arr_resource_noun,
    shape_arr_list, slim_paged_records,
};
use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::{RustarrClient, slim};
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

#[test]
fn quality_profile_slim_keeps_identifiers_and_cutoffs() {
    let raw = json!([{
        "id": 1,
        "name": "HD - 720p/1080p",
        "cutoff": 4,
        "cutoffFormatScore": 0,
        "minFormatScore": 0,
        "minUpgradeFormatScore": 1,
        "upgradeAllowed": false,
        "items": [{"quality": {"name": "HDTV-720p"}}],
        "formatItems": [{"name": "dropme"}]
    }]);
    let slimmed = slim(raw, QUALITY_PROFILE_FIELDS);
    let row = &slimmed[0];
    for keep in QUALITY_PROFILE_FIELDS {
        assert!(row.get(keep).is_some(), "slim should keep {keep}");
    }
    assert!(row.get("items").is_none(), "quality tree should be dropped");
    assert!(
        row.get("formatItems").is_none(),
        "custom format tree should be dropped"
    );
}

#[test]
fn queue_slim_keeps_import_context_and_drops_bulk_payloads() {
    let raw = json!({
        "page": 1,
        "pageSize": 50,
        "totalRecords": 1,
        "records": [{
            "id": 9,
            "title": "Example.S01E01.1080p.WEB-DL",
            "status": "completed",
            "trackedDownloadStatus": "warning",
            "trackedDownloadState": "importPending",
            "quality": {"quality": {"name": "WEBDL-1080p"}},
            "series": {"title": "Example Show", "overview": "drop me"},
            "episode": {"title": "Pilot", "seasonNumber": 1, "episodeNumber": 1},
            "statusMessages": [{"title": "Example", "messages": [
                "Episode was not found",
                "This message is intentionally long enough to exercise the compact text clipping behavior. This message is intentionally long enough to exercise the compact text clipping behavior. This message is intentionally long enough to exercise the compact text clipping behavior.",
                "kept",
                "dropped"
            ]}],
            "customFormats": [{"name": "huge"}],
            "outputPath": "/data/usenet/tv/example",
            "downloadId": "secret-ish-noise"
        }]
    });

    let slimmed = slim_paged_records(raw, QUEUE_FIELDS);
    let record = &slimmed["records"][0];
    assert_eq!(record["title"], "Example.S01E01.1080p.WEB-DL");
    assert_eq!(record["trackedDownloadStatus"], "warning");
    assert_eq!(record["quality"], "WEBDL-1080p");
    assert_eq!(record["seriesTitle"], "Example Show");
    assert_eq!(record["episodeTitle"], "Pilot");
    assert_eq!(record["seasonNumber"], 1);
    assert_eq!(record["episodeNumber"], 1);
    assert_eq!(
        record["statusMessages"][0]["messages"]
            .as_array()
            .expect("messages array")
            .len(),
        3
    );
    assert!(record.get("customFormats").is_none());
    assert!(record.get("outputPath").is_none());
    assert!(record.get("downloadId").is_none());
    assert!(record.get("series").is_none());
    assert!(record.get("episode").is_none());
}

#[test]
fn list_shape_returns_quality_summary_and_bounded_items() {
    let raw = json!([
        {
            "id": 1,
            "title": "Alpha",
            "qualityProfileId": 5,
            "monitored": true,
            "sizeOnDisk": 10,
            "status": "released",
            "added": "2020-01-01T00:00:00Z",
            "overview": "drop me"
        },
        {
            "id": 2,
            "title": "Beta",
            "qualityProfileId": 5,
            "monitored": false,
            "sizeOnDisk": 0,
            "status": "released",
            "added": "2020-01-02T00:00:00Z"
        },
        {
            "id": 3,
            "title": "Gamma",
            "qualityProfileId": 7,
            "monitored": true,
            "sizeOnDisk": 20,
            "status": "announced",
            "added": "2020-01-03T00:00:00Z"
        }
    ]);
    let profiles = json!([
        { "id": 5, "name": "Ultra-HD" },
        { "id": 7, "name": "Any" }
    ]);

    let shaped = shape_arr_list(
        ServiceKind::Radarr,
        raw,
        profiles,
        ArrListOptions {
            limit: Some(2),
            offset: 1,
            fields: Vec::new(),
        },
    );

    assert_eq!(shaped["total"], 3);
    assert_eq!(shaped["offset"], 1);
    assert_eq!(shaped["limit"], 2);
    assert_eq!(shaped["returned"], 2);
    assert_eq!(shaped["has_more"], false);
    assert_eq!(shaped["summary"]["monitored"], 2);
    assert_eq!(shaped["summary"]["unmonitored"], 1);
    assert_eq!(shaped["summary"]["missing_items"], 1);
    assert_eq!(shaped["summary"]["size_on_disk"], 30);
    assert_eq!(
        shaped["summary"]["by_quality_profile"],
        json!([
            { "id": 5, "name": "Ultra-HD", "count": 2 },
            { "id": 7, "name": "Any", "count": 1 }
        ])
    );
    assert_eq!(shaped["items"].as_array().unwrap().len(), 2);
    assert_eq!(shaped["items"][0]["title"], "Beta");
    assert_eq!(shaped["items"][1]["title"], "Gamma");
    assert!(shaped["items"][0].get("overview").is_none());
}

#[test]
fn list_shape_honors_requested_item_fields_without_breaking_summary() {
    let raw = json!([
        {
            "id": 1,
            "title": "Series",
            "qualityProfileId": 4,
            "monitored": true,
            "status": "continuing",
            "statistics": {
                "episodeCount": 10,
                "episodeFileCount": 7
            }
        }
    ]);
    let profiles = json!([{ "id": 4, "name": "HD-1080p" }]);

    let shaped = shape_arr_list(
        ServiceKind::Sonarr,
        raw,
        profiles,
        ArrListOptions {
            limit: Some(1),
            offset: 0,
            fields: vec!["title".into(), "qualityProfileId".into()],
        },
    );

    assert_eq!(shaped["summary"]["missing_episodes"], 3);
    assert_eq!(shaped["summary"]["episode_count"], 10);
    assert_eq!(shaped["summary"]["episode_file_count"], 7);
    assert_eq!(
        shaped["items"],
        json!([{ "title": "Series", "qualityProfileId": 4 }])
    );
}

#[tokio::test]
async fn arr_methods_reject_wrong_capability() {
    // Plex is a MediaServer, not an ArrManager: every arr read method must reject
    // it BEFORE attempting any network call (the capability guard fails synchronously).
    let svc = service_with(&[("plex", ServiceKind::Plex)]);
    let err = svc
        .arr_list("plex", ArrListOptions::default())
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
        let result = svc.arr_list(service, ArrListOptions::default()).await;
        if let Err(err) = result {
            let msg = err.to_string();
            assert!(
                !msg.contains("does not provide"),
                "{service} should pass the capability guard, got: {msg}"
            );
        }
    }
}
