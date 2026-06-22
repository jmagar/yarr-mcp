//! Deserialization fixtures for the ArrManager (Sonarr/Radarr) models.

use super::*;
use serde_json::json;

#[test]
fn series_row_decodes_with_statistics() {
    let raw = json!({
        "id": 7,
        "title": "The Expanse",
        "qualityProfileId": 4,
        "monitored": true,
        "sizeOnDisk": 123456789_u64,
        "status": "continuing",
        "added": "2021-01-01T00:00:00Z",
        "statistics": { "episodeCount": 62, "episodeFileCount": 60, "sizeOnDisk": 123456789_u64 },
        // dropped fields must not break decoding:
        "seasons": [{ "seasonNumber": 1 }],
        "images": []
    });
    let row: ArrResource = serde_json::from_value(raw).unwrap();
    assert_eq!(row.title.as_deref(), Some("The Expanse"));
    assert_eq!(row.quality_profile_id, Some(4));
    let stats = row.statistics.expect("statistics present");
    assert_eq!(stats.episode_count, Some(62));
    assert_eq!(stats.episode_file_count, Some(60));
}

#[test]
fn quality_profile_decodes() {
    let raw = json!({
        "id": 4,
        "name": "HD-1080p",
        "cutoff": 9,
        "upgradeAllowed": true,
        "minFormatScore": 0,
        "items": [{ "quality": { "id": 9, "name": "HDTV-1080p" }, "allowed": true }]
    });
    let p: QualityProfile = serde_json::from_value(raw).unwrap();
    assert_eq!(p.name.as_deref(), Some("HD-1080p"));
    assert_eq!(p.upgrade_allowed, Some(true));
}

#[test]
fn paged_queue_decodes_lowercase_size_time_left() {
    let raw = json!({
        "page": 1,
        "pageSize": 50,
        "totalRecords": 1,
        "records": [{
            "id": 99,
            "title": "Some.Release.1080p",
            "status": "downloading",
            "size": 1000_u64,
            "sizeleft": 250_u64,
            "timeleft": "00:05:00",
            "trackedDownloadStatus": "ok",
            "trackedDownloadState": "downloading",
            "downloadClient": "qbittorrent",
            "indexer": "nzbgeek",
            "quality": { "quality": { "id": 9, "name": "HDTV-1080p" } },
            "series": { "id": 7, "title": "The Expanse" },
            "episode": { "id": 1, "title": "Pilot", "seasonNumber": 1, "episodeNumber": 1 },
            "statusMessages": [{ "title": "Import", "messages": ["No files found"] }]
        }]
    });
    let page: PagedRecords = serde_json::from_value(raw).unwrap();
    assert_eq!(page.total_records, Some(1));
    let rec = &page.records[0];
    assert_eq!(rec.sizeleft, Some(250));
    assert_eq!(rec.timeleft.as_deref(), Some("00:05:00"));
    assert_eq!(
        rec.quality
            .as_ref()
            .unwrap()
            .quality
            .as_ref()
            .unwrap()
            .name
            .as_deref(),
        Some("HDTV-1080p")
    );
    assert_eq!(rec.episode.as_ref().unwrap().season_number, Some(1));
    assert_eq!(rec.status_messages[0].messages, vec!["No files found"]);
}

#[test]
fn history_event_type_decodes() {
    let raw = json!({
        "records": [{ "id": 5, "eventType": "grabbed", "date": "2024-01-01T00:00:00Z" }]
    });
    let page: PagedRecords = serde_json::from_value(raw).unwrap();
    assert_eq!(page.records[0].event_type.as_deref(), Some("grabbed"));
}

#[test]
fn rootfolder_and_health_decode() {
    let folder: RootFolder = serde_json::from_value(json!({
        "id": 1, "path": "/media/tv", "accessible": true, "freeSpace": 5000_u64, "totalSpace": 10000_u64
    }))
    .unwrap();
    assert_eq!(folder.path.as_deref(), Some("/media/tv"));
    assert_eq!(folder.free_space, Some(5000));

    let health: HealthMessage = serde_json::from_value(json!({
        "source": "IndexerStatusCheck",
        "type": "warning",
        "message": "Indexers unavailable",
        "wikiUrl": "https://wiki.servarr.com/sonarr"
    }))
    .unwrap();
    assert_eq!(health.kind.as_deref(), Some("warning"));
    assert_eq!(health.source.as_deref(), Some("IndexerStatusCheck"));
}

#[test]
fn empty_records_default() {
    let page: PagedRecords = serde_json::from_value(json!({ "totalRecords": 0 })).unwrap();
    assert!(page.records.is_empty());
}
