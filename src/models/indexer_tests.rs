//! Deserialization fixtures for the Indexer (Prowlarr) models.

use super::*;
use serde_json::json;

#[test]
fn indexer_row_decodes() {
    let raw = json!({
        "id": 3,
        "name": "NZBgeek",
        "enable": true,
        "protocol": "usenet",
        "priority": 25,
        // dropped definition payload must not break decoding:
        "fields": [{ "name": "apiKey", "value": "secret" }],
        "capabilities": { "categories": [] }
    });
    let ix: Indexer = serde_json::from_value(raw).unwrap();
    assert_eq!(ix.name.as_deref(), Some("NZBgeek"));
    assert_eq!(ix.protocol.as_deref(), Some("usenet"));
    assert_eq!(ix.enable, Some(true));
}

#[test]
fn search_release_decodes() {
    let raw = json!({
        "title": "Some.Movie.2024.1080p.BluRay",
        "indexer": "1337x",
        "indexerId": 5,
        "protocol": "torrent",
        "seeders": 120,
        "leechers": 4,
        "size": 8123456789_u64,
        "publishDate": "2024-06-01T00:00:00Z",
        "infoHash": "abcdef0123456789",
        "magnetUrl": "magnet:?xt=urn:btih:abcdef"
    });
    let rel: SearchRelease = serde_json::from_value(raw).unwrap();
    assert_eq!(rel.indexer_id, Some(5));
    assert_eq!(rel.seeders, Some(120));
    assert_eq!(rel.info_hash.as_deref(), Some("abcdef0123456789"));
}

#[test]
fn indexer_stats_unwraps_indexers_array() {
    let raw = json!({
        "indexers": [{
            "indexerId": 5,
            "indexerName": "1337x",
            "numberOfQueries": 100,
            "numberOfGrabs": 12,
            "numberOfFailedQueries": 3,
            "averageResponseTime": 845.5
        }],
        "userAgents": [{ "userAgent": "Sonarr", "numberOfQueries": 50 }]
    });
    let stats: IndexerStatsResponse = serde_json::from_value(raw).unwrap();
    assert_eq!(stats.indexers.len(), 1);
    let row = &stats.indexers[0];
    assert_eq!(row.indexer_name.as_deref(), Some("1337x"));
    assert_eq!(row.number_of_queries, Some(100));
    assert_eq!(row.average_response_time, Some(845.5));
}
