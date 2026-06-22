//! Deserialization fixtures for the Requests (Overseerr) models.

use super::*;
use serde_json::json;

#[test]
fn request_page_decodes_with_page_info() {
    let raw = json!({
        "pageInfo": { "pages": 2, "pageSize": 10, "results": 15, "page": 1 },
        "results": [{
            "id": 42,
            "type": "movie",
            "status": 2,
            "media": { "id": 9, "tmdbId": 27205, "mediaType": "movie", "status": 5 },
            "requestedBy": { "id": 1, "displayName": "alice", "email": "alice@example.com" }
        }]
    });
    let page: RequestsPage = serde_json::from_value(raw).unwrap();
    assert_eq!(page.page_info.as_ref().unwrap().results, Some(15));
    let req = &page.results[0];
    assert_eq!(req.kind.as_deref(), Some("movie"));
    assert_eq!(req.status, Some(2));
    assert_eq!(req.media.as_ref().unwrap().tmdb_id, Some(27205));
    assert_eq!(
        req.requested_by.as_ref().unwrap().display_name.as_deref(),
        Some("alice")
    );
}

#[test]
fn search_response_decodes_movie_and_tv_variants() {
    let raw = json!({
        "page": 1,
        "totalPages": 3,
        "totalResults": 60,
        "results": [
            { "id": 27205, "mediaType": "movie", "title": "Inception", "releaseDate": "2010-07-16", "overview": "A thief…" },
            { "id": 1396, "mediaType": "tv", "name": "Breaking Bad", "firstAirDate": "2008-01-20", "overview": "A chemistry teacher…" }
        ]
    });
    let resp: SearchResponse = serde_json::from_value(raw).unwrap();
    assert_eq!(resp.total_results, Some(60));
    assert_eq!(resp.results[0].title.as_deref(), Some("Inception"));
    assert_eq!(resp.results[0].release_date.as_deref(), Some("2010-07-16"));
    assert_eq!(resp.results[1].name.as_deref(), Some("Breaking Bad"));
    assert_eq!(
        resp.results[1].first_air_date.as_deref(),
        Some("2008-01-20")
    );
}

#[test]
fn empty_results_default() {
    let page: RequestsPage = serde_json::from_value(json!({ "pageInfo": {} })).unwrap();
    assert!(page.results.is_empty());
}
