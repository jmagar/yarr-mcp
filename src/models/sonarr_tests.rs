//! Colocated unit tests for the Sonarr models.
//!
//! These prove the main resources decode from representative Sonarr `/api/v3`
//! payloads, including: enum string values, the `type` → `kind` rename on
//! [`HealthResource`], the string-valued [`HistoryResource::data`] map, the
//! `sortDirection` enum (not a string) on a paging wrapper, an "extra unknown
//! fields are ignored" case, and an "empty object → all `None` / empty `Vec`"
//! case.

use super::*;
use serde_json::json;

#[test]
fn series_resource_decodes_with_nested_statistics_and_enums() {
    let v = json!({
        "id": 42,
        "title": "The Expanse",
        "status": "ended",
        "ended": true,
        "year": 2015,
        "qualityProfileId": 6,
        "monitored": true,
        "seriesType": "standard",
        "sizeOnDisk": 123456789012i64,
        "tags": [1, 2, 3],
        "genres": ["Drama", "Sci-Fi"],
        "ratings": { "votes": 1000, "value": 8.7 },
        "statistics": {
            "seasonCount": 6,
            "episodeFileCount": 62,
            "episodeCount": 62,
            "totalEpisodeCount": 62,
            "sizeOnDisk": 123456789012i64,
            "releaseGroups": ["NTb"],
            "percentOfEpisodes": 100.0
        },
        "seasons": [
            { "seasonNumber": 1, "monitored": true }
        ]
    });

    let s: SeriesResource = serde_json::from_value(v).unwrap();
    assert_eq!(s.id, Some(42));
    assert_eq!(s.title.as_deref(), Some("The Expanse"));
    assert_eq!(s.status, Some(SeriesStatusType::Ended));
    assert_eq!(s.series_type, Some(SeriesTypes::Standard));
    assert_eq!(s.tags, vec![1, 2, 3]);
    assert_eq!(s.genres, vec!["Drama".to_string(), "Sci-Fi".to_string()]);

    let stats = s.statistics.expect("statistics present");
    assert_eq!(stats.size_on_disk, Some(123_456_789_012));
    assert_eq!(stats.percent_of_episodes, Some(100.0));

    let ratings = s.ratings.expect("ratings present");
    assert_eq!(ratings.value, Some(8.7));

    assert_eq!(s.seasons.len(), 1);
    assert_eq!(s.seasons[0].season_number, Some(1));
}

#[test]
fn health_resource_renames_type_to_kind() {
    let v = json!({
        "id": 1,
        "source": "ImportListStatusCheck",
        "type": "warning",
        "message": "Lists unavailable due to failures",
        "wikiUrl": "https://wiki.servarr.com/sonarr/system#lists-are-unavailable"
    });

    let h: HealthResource = serde_json::from_value(v).unwrap();
    assert_eq!(h.kind, Some(HealthCheckResult::Warning));
    assert_eq!(h.source.as_deref(), Some("ImportListStatusCheck"));
}

#[test]
fn history_resource_data_is_string_valued_map() {
    // Values are ALWAYS strings even when they encode numbers.
    let v = json!({
        "id": 7,
        "episodeId": 100,
        "seriesId": 42,
        "sourceTitle": "The.Expanse.S01E01.1080p.WEB.h264-NTb",
        "date": "2023-01-15T04:00:00Z",
        "eventType": "downloadFolderImported",
        "data": {
            "indexer": "Cardigann",
            "releaseGroup": "NTb",
            "size": "1234567890",
            "age": "0",
            "tvdbId": "264586"
        }
    });

    let h: HistoryResource = serde_json::from_value(v).unwrap();
    assert_eq!(
        h.event_type,
        Some(EpisodeHistoryEventType::DownloadFolderImported)
    );
    let data = h.data.expect("data map present");
    assert_eq!(data.get("size"), Some(&Some("1234567890".to_string())));
    assert_eq!(data.get("tvdbId"), Some(&Some("264586".to_string())));
}

#[test]
fn queue_paging_wrapper_decodes_sort_direction_enum_and_double_size() {
    // sortDirection is an enum, not a string; QueueResource.size is a double.
    let v = json!({
        "page": 1,
        "pageSize": 20,
        "sortKey": "timeleft",
        "sortDirection": "ascending",
        "totalRecords": 1,
        "records": [
            {
                "id": 5,
                "title": "The.Expanse.S02E03.1080p.WEB.h264-NTb",
                "size": 2147483648.5,
                "sizeleft": 1073741824.0,
                "timeleft": "00:13:37",
                "status": "downloading",
                "trackedDownloadStatus": "ok",
                "trackedDownloadState": "downloading",
                "protocol": "usenet",
                "statusMessages": [
                    { "title": "Sample", "messages": ["a", "b"] }
                ]
            }
        ]
    });

    let page: QueueResourcePagingResource = serde_json::from_value(v).unwrap();
    assert_eq!(page.sort_direction, Some(SortDirection::Ascending));
    assert_eq!(page.total_records, Some(1));
    assert_eq!(page.records.len(), 1);

    let rec = &page.records[0];
    assert_eq!(rec.size, Some(2_147_483_648.5));
    assert_eq!(rec.timeleft.as_deref(), Some("00:13:37"));
    assert_eq!(rec.status, Some(QueueStatus::Downloading));
    assert_eq!(rec.protocol, Some(DownloadProtocol::Usenet));
    assert_eq!(rec.status_messages.len(), 1);
    assert_eq!(
        rec.status_messages[0].messages,
        vec!["a".to_string(), "b".to_string()]
    );
}

#[test]
fn unknown_fields_are_ignored() {
    // Fields not in the schema must deserialize-and-ignore.
    let v = json!({
        "id": 9,
        "title": "Mystery Show",
        "thisFieldDoesNotExist": "ignore me",
        "anotherUnknown": { "nested": [1, 2, 3] },
        "year": 2020
    });

    let s: SeriesResource = serde_json::from_value(v).unwrap();
    assert_eq!(s.id, Some(9));
    assert_eq!(s.title.as_deref(), Some("Mystery Show"));
    assert_eq!(s.year, Some(2020));
}

#[test]
fn empty_object_yields_all_none_and_empty_vecs() {
    let s: SeriesResource = serde_json::from_value(json!({})).unwrap();
    assert_eq!(s.id, None);
    assert_eq!(s.title, None);
    assert_eq!(s.status, None);
    assert!(s.seasons.is_empty());
    assert!(s.tags.is_empty());
    assert!(s.genres.is_empty());
    assert!(s.images.is_empty());
    assert!(s.alternate_titles.is_empty());

    // Same for a paging wrapper: records defaults to an empty Vec.
    let page: HistoryResourcePagingResource = serde_json::from_value(json!({})).unwrap();
    assert_eq!(page.page, None);
    assert_eq!(page.sort_direction, None);
    assert!(page.records.is_empty());
}

#[test]
fn database_type_preserves_mixed_case_values() {
    let sqlite: DatabaseType = serde_json::from_value(json!("sqLite")).unwrap();
    assert_eq!(sqlite, DatabaseType::SqLite);
    let pg: DatabaseType = serde_json::from_value(json!("postgreSQL")).unwrap();
    assert_eq!(pg, DatabaseType::PostgreSql);
}
