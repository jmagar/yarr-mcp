//! Deserialization fixtures for the MediaServer (Plex) models.

use super::*;
use serde_json::json;

#[test]
fn sessions_container_decodes_with_player_session_user() {
    // GET /status/sessions: MediaContainer.Metadata[] with the session-only
    // Player/Session/User children, mixed-casing keys, and string-encoded
    // ratingKey. The `location` enum and `bandwidth` round-trip.
    let raw = json!({
        "size": 1,
        "Metadata": [{
            "title": "The Matrix",
            "type": "movie",
            "key": "/library/metadata/12345",
            "addedAt": 1_600_000_000_i64,
            "ratingKey": "12345",
            "viewOffset": 372000,
            "duration": 8_160_000,
            "rating": 8.7,
            "audienceRating": 9.1,
            "skipChildren": "1",
            "librarySectionTitle": "Movies",
            "Media": [{
                "id": 98765_i64,
                "duration": 8_160_000,
                "aspectRatio": 2.35,
                "videoResolution": "1080",
                "hasVoiceActivity": false,
                "optimizedForStreaming": 1,
                "Part": [{
                    "id": 54321_i64,
                    "key": "/library/parts/54321/file.mkv",
                    "size": 9_876_543_210_i64,
                    "container": "mkv",
                    "optimizedForStreaming": true,
                    "Stream": [{ "id": 1_i64, "streamType": 1, "codec": "h264" }]
                }]
            }],
            "Player": { "title": "Living Room TV", "state": "playing", "userID": 7, "local": true },
            "Session": { "id": "abc-123", "bandwidth": 4200, "location": "lan" },
            "User": { "id": "7", "title": "alice", "thumb": "https://plex.tv/u/7.png" }
        }]
    });
    let container: MediaContainer = serde_json::from_value(raw).unwrap();
    assert_eq!(container.size, Some(1));
    assert_eq!(container.metadata.len(), 1);

    let item = &container.metadata[0];
    assert_eq!(item.title, "The Matrix");
    assert_eq!(item.kind, "movie");
    // ratingKey is string-encoded despite looking numeric.
    assert_eq!(item.rating_key.as_deref(), Some("12345"));
    assert_eq!(item.view_offset, Some(372000));
    assert_eq!(item.rating, Some(8.7));
    assert_eq!(item.skip_children, Some(BoolOrIntStr::Str("1".to_string())));
    assert_eq!(item.library_section_title.as_deref(), Some("Movies"));

    let media = &item.media[0];
    assert_eq!(media.id, 98765);
    assert_eq!(media.aspect_ratio, Some(2.35));
    assert_eq!(media.has_voice_activity, Some(BoolOrIntStr::Bool(false)));
    assert_eq!(media.optimized_for_streaming, Some(1));

    let part = &media.part[0];
    assert_eq!(part.id, 54321);
    assert_eq!(part.size, Some(9_876_543_210));
    // Part.optimizedForStreaming is a bool, unlike Media's int.
    assert_eq!(part.optimized_for_streaming, Some(true));
    assert_eq!(part.stream[0].id, 1);

    let player = item.player.as_ref().expect("player present");
    // Player.userID is an integer, unlike User.id which is a string.
    assert_eq!(player.user_id, Some(7));
    assert_eq!(player.state.as_deref(), Some("playing"));

    let session = item.session.as_ref().expect("session present");
    assert_eq!(session.bandwidth, Some(4200));
    assert_eq!(session.location, Some(SessionLocation::Lan));

    let user = item.user.as_ref().expect("user present");
    assert_eq!(user.id.as_deref(), Some("7"));
    assert_eq!(user.title.as_deref(), Some("alice"));
}

#[test]
fn library_sections_container_decodes_directory_as_library_section() {
    // GET /library/sections/all: MediaContainer.Directory[] items are
    // LibrarySection, with epoch PlexDateTime timestamps and a Location array.
    let raw = json!({
        "size": 2,
        "title1": "Plex Library",
        "allowSync": "0",
        "Directory": [{
            "uuid": "e69655a2-1111-2222-3333-444455556666",
            "language": "en-US",
            "type": "movie",
            "title": "Movies",
            "agent": "tv.plex.agents.movie",
            "scanner": "Plex Movie",
            "key": "1",
            "allowSync": true,
            "refreshing": false,
            "createdAt": 1_500_000_000_i64,
            "scannedAt": 1_700_000_000_i64,
            "Location": [{ "id": 1, "path": "/data/movies" }]
        }]
    });
    let container: MediaContainer = serde_json::from_value(raw).unwrap();
    assert_eq!(container.title1.as_deref(), Some("Plex Library"));
    assert_eq!(
        container.allow_sync,
        Some(BoolOrIntStr::Str("0".to_string()))
    );

    let section = &container.directory[0];
    assert_eq!(section.uuid, "e69655a2-1111-2222-3333-444455556666");
    assert_eq!(section.language, "en-US");
    assert_eq!(section.kind, "movie");
    assert_eq!(section.title.as_deref(), Some("Movies"));
    assert_eq!(section.allow_sync, Some(BoolOrIntStr::Bool(true)));
    assert_eq!(section.created_at, Some(1_500_000_000));
    assert_eq!(section.scanned_at, Some(1_700_000_000));
    assert_eq!(section.location[0].path.as_deref(), Some("/data/movies"));
}

#[test]
fn search_hub_carries_metadata_with_reason_fields() {
    // GET /hubs/search: MediaContainer.Hub[] each grouping Metadata[]; search
    // reason/score fields ride along via additionalProperties.
    let raw = json!({
        "Hub": [{
            "title": "Movies",
            "type": "movie",
            "hubIdentifier": "movie",
            "size": 1,
            "Metadata": [{
                "title": "Terminator",
                "type": "movie",
                "key": "/library/metadata/999",
                "addedAt": 1_650_000_000_i64,
                "reason": "actor",
                "reasonTitle": "Arnold Schwarzenegger",
                "reasonID": "49",
                "score": 0.92
            }]
        }]
    });
    let container: MediaContainer = serde_json::from_value(raw).unwrap();
    let hub = &container.hub[0];
    assert_eq!(hub.title.as_deref(), Some("Movies"));
    assert_eq!(hub.kind.as_deref(), Some("movie"));

    let hit = &hub.metadata[0];
    assert_eq!(hit.title, "Terminator");
    assert_eq!(hit.reason.as_deref(), Some("actor"));
    assert_eq!(hit.reason_title.as_deref(), Some("Arnold Schwarzenegger"));
    assert_eq!(hit.reason_id.as_deref(), Some("49"));
    assert_eq!(hit.score, Some(0.92));
}

#[test]
fn identity_container_decodes_scalar_fields() {
    // GET /identity exposes the inline MediaContainer scalars.
    let raw = json!({
        "size": 0,
        "machineIdentifier": "abc123def456",
        "version": "1.40.0.1234-abcdef",
        "claimed": true
    });
    let container: MediaContainer = serde_json::from_value(raw).unwrap();
    assert_eq!(
        container.machine_identifier.as_deref(),
        Some("abc123def456")
    );
    assert_eq!(container.version.as_deref(), Some("1.40.0.1234-abcdef"));
    assert_eq!(container.claimed, Some(true));
}

#[test]
fn unknown_fields_are_ignored() {
    // PMS routinely returns undocumented extras (additionalProperties:true on
    // Metadata/Media/Part). They must deserialize-and-ignore, not fail.
    let raw = json!({
        "Metadata": [{
            "title": "Mystery",
            "type": "episode",
            "key": "/library/metadata/7",
            "addedAt": 1_700_000_000_i64,
            "someBrandNewField": "ignored",
            "anotherUndocumentedThing": { "nested": [1, 2, 3] },
            "Media": [{ "id": 5_i64, "futureCodecField": "av1" }]
        }]
    });
    let container: MediaContainer = serde_json::from_value(raw).unwrap();
    let item = &container.metadata[0];
    assert_eq!(item.title, "Mystery");
    assert_eq!(item.kind, "episode");
    assert_eq!(item.media[0].id, 5);
}

#[test]
fn empty_objects_yield_none_and_empty_vecs() {
    // An empty MediaContainer leaves every optional None and every array empty.
    let container: MediaContainer = serde_json::from_value(json!({})).unwrap();
    assert!(container.identifier.is_none());
    assert!(container.size.is_none());
    assert!(container.machine_identifier.is_none());
    assert!(container.metadata.is_empty());
    assert!(container.directory.is_empty());
    assert!(container.hub.is_empty());

    // Minimal required-field objects decode with everything else defaulted.
    let player: Player = serde_json::from_value(json!({})).unwrap();
    assert!(player.title.is_none());
    assert!(player.user_id.is_none());

    let tag: Tag = serde_json::from_value(json!({ "tag": "Action" })).unwrap();
    assert_eq!(tag.tag, "Action");
    assert!(tag.id.is_none());
    assert!(tag.rating_key.is_none());

    let image: Image = serde_json::from_value(json!({ "type": "coverPoster" })).unwrap();
    assert_eq!(image.kind, Some(ImageType::CoverPoster));
    assert!(image.url.is_none());
}
