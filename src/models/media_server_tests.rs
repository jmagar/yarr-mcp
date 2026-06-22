//! Deserialization fixtures for the MediaServer (Plex/Jellyfin) models.

use super::*;
use serde_json::json;

#[test]
fn plex_sessions_unwrap_metadata() {
    let raw = json!({
        "MediaContainer": {
            "size": 1,
            "Metadata": [{
                "ratingKey": "12345",
                "title": "The Matrix",
                "type": "movie",
                "year": 1999,
                "viewOffset": 60000,
                "User": { "id": "1", "title": "alice" },
                "Player": { "title": "Living Room", "state": "playing", "product": "Plex for Apple TV" }
            }]
        }
    });
    let resp: PlexResponse = serde_json::from_value(raw).unwrap();
    let container = resp.media_container.expect("container present");
    assert_eq!(container.metadata.len(), 1);
    let item = &container.metadata[0];
    assert_eq!(item.title.as_deref(), Some("The Matrix"));
    assert_eq!(item.kind.as_deref(), Some("movie"));
    assert_eq!(item.view_offset, Some(60000));
    assert_eq!(item.user.as_ref().unwrap().title.as_deref(), Some("alice"));
}

#[test]
fn plex_libraries_unwrap_directory() {
    let raw = json!({
        "MediaContainer": {
            "Directory": [
                { "key": "1", "title": "Movies", "type": "movie" },
                { "key": "2", "title": "TV Shows", "type": "show" }
            ]
        }
    });
    let resp: PlexResponse = serde_json::from_value(raw).unwrap();
    let dirs = resp.media_container.unwrap().directory;
    assert_eq!(dirs.len(), 2);
    assert_eq!(dirs[1].title.as_deref(), Some("TV Shows"));
    assert_eq!(dirs[1].kind.as_deref(), Some("show"));
}

#[test]
fn plex_search_results_wrap_metadata() {
    let raw = json!({
        "MediaContainer": {
            "SearchResult": [{
                "score": 0.9,
                "Metadata": { "ratingKey": "777", "title": "Dune", "type": "movie", "year": 2021, "librarySectionTitle": "Movies" }
            }]
        }
    });
    let resp: PlexResponse = serde_json::from_value(raw).unwrap();
    let results = resp.media_container.unwrap().search_result;
    let meta = results[0].metadata.as_ref().unwrap();
    assert_eq!(meta.title.as_deref(), Some("Dune"));
    assert_eq!(meta.library_section_title.as_deref(), Some("Movies"));
}

#[test]
fn plex_identity_decodes() {
    let raw = json!({ "MediaContainer": { "machineIdentifier": "abc123", "version": "1.40.0" } });
    let resp: PlexResponse = serde_json::from_value(raw).unwrap();
    let c = resp.media_container.unwrap();
    assert_eq!(c.machine_identifier.as_deref(), Some("abc123"));
    assert_eq!(c.version.as_deref(), Some("1.40.0"));
}

#[test]
fn jellyfin_session_decodes_pascal_case() {
    let raw = json!({
        "UserName": "bob",
        "DeviceName": "Web Browser",
        "Client": "Jellyfin Web",
        "NowPlayingItem": { "Id": "uuid-1", "Name": "Inception", "Type": "Movie", "ProductionYear": 2010 },
        "PlayState": { "PositionTicks": 12000000000_i64, "IsPaused": false, "PlayMethod": "DirectPlay" }
    });
    let s: JellyfinSession = serde_json::from_value(raw).unwrap();
    assert_eq!(s.user_name.as_deref(), Some("bob"));
    let item = s.now_playing_item.expect("now playing");
    assert_eq!(item.name.as_deref(), Some("Inception"));
    assert_eq!(item.kind.as_deref(), Some("Movie"));
    assert_eq!(s.play_state.unwrap().is_paused, Some(false));
}

#[test]
fn jellyfin_virtual_folder_and_items_decode() {
    let folder: VirtualFolder = serde_json::from_value(json!({
        "ItemId": "lib-1", "Name": "Movies", "CollectionType": "movies"
    }))
    .unwrap();
    assert_eq!(folder.item_id.as_deref(), Some("lib-1"));
    assert_eq!(folder.collection_type.as_deref(), Some("movies"));

    let items: JellyfinItemsResponse = serde_json::from_value(json!({
        "Items": [{ "Id": "uuid-9", "Name": "Severance", "Type": "Series", "SeriesName": null, "ProductionYear": 2022 }],
        "TotalRecordCount": 1
    }))
    .unwrap();
    assert_eq!(items.items.len(), 1);
    assert_eq!(items.items[0].kind.as_deref(), Some("Series"));
    assert_eq!(items.items[0].production_year, Some(2022));
}
