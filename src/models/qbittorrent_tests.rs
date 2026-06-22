//! Deserialization fixtures for the qBittorrent (WebUI API v5.0) models.

use super::*;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn torrent_info_decodes_native_numerics_and_camelcase_exception() {
    let raw = json!({
        "added_on": 1_700_000_000_i64,
        "amount_left": 0,
        "auto_tmm": true,
        "availability": 1.0,
        "category": "movies",
        "completed": 1_048_576,
        "completion_on": 1_700_000_500_i64,
        "content_path": "/downloads/Foo",
        "dl_limit": -1,
        "dlspeed": 0,
        "downloaded": 1_048_576,
        "downloaded_session": 1_048_576,
        "eta": 8_640_000,
        "f_l_piece_prio": false,
        "force_start": false,
        "hash": "abc123",
        "isPrivate": true,
        "last_activity": 1_700_000_400_i64,
        "magnet_uri": "magnet:?xt=urn:btih:abc123",
        "max_ratio": -1.0,
        "max_seeding_time": -1,
        "name": "Foo",
        "num_complete": 50,
        "num_incomplete": 3,
        "num_leechs": 1,
        "num_seeds": 12,
        "priority": -1,
        "progress": 1.0,
        "ratio": 2.5,
        "ratio_limit": -1.0,
        "reannounce": 1700,
        "save_path": "/downloads",
        "seeding_time": 3600,
        "seeding_time_limit": -2,
        "seen_complete": 1_700_000_450_i64,
        "seq_dl": false,
        "size": 1_048_576,
        "state": "stalledUP",
        "super_seeding": false,
        "tags": "hd,keep",
        "time_active": 7200,
        "total_size": 1_048_576,
        "tracker": "https://tracker.example/announce",
        "up_limit": -1,
        "uploaded": 2_621_440,
        "uploaded_session": 2_621_440,
        "upspeed": 1024
    });
    let t: TorrentInfo = serde_json::from_value(raw).unwrap();
    assert_eq!(t.hash.as_deref(), Some("abc123"));
    assert_eq!(t.added_on, Some(1_700_000_000));
    // Native numerics, sentinel -1 preserved.
    assert_eq!(t.dl_limit, Some(-1));
    assert_eq!(t.priority, Some(-1));
    assert_eq!(t.progress, Some(1.0));
    assert_eq!(t.ratio, Some(2.5));
    // camelCase exception mapped to snake_case field.
    assert_eq!(t.is_private, Some(true));
    // tags is a single comma-joined string, not an array.
    assert_eq!(t.tags.as_deref(), Some("hd,keep"));
    // Mixed-case enum literal decodes to the matching variant.
    assert_eq!(t.state, Some(TorrentState::StalledUP));
}

#[test]
fn torrent_state_unknown_is_forward_compatible() {
    // A 5.x `stopped*` value the dossier enum does not list falls back cleanly.
    let raw = json!({ "state": "stoppedDL" });
    let t: TorrentInfo = serde_json::from_value(raw).unwrap();
    assert_eq!(t.state, Some(TorrentState::Unknown));
}

#[test]
fn transfer_info_partial_fields_are_optional_and_status_enum_decodes() {
    // The non-sync variant omits queueing / use_alt_speed_limits / refresh_interval.
    let raw = json!({
        "dl_info_speed": 1024,
        "dl_info_data": 1_048_576,
        "up_info_speed": 512,
        "up_info_data": 524_288,
        "dl_rate_limit": 0,
        "up_rate_limit": 0,
        "dht_nodes": 321,
        "connection_status": "connected"
    });
    let info: TransferInfo = serde_json::from_value(raw).unwrap();
    assert_eq!(info.dl_info_speed, Some(1024));
    assert_eq!(info.connection_status, Some(ConnectionStatus::Connected));
    assert!(info.queueing.is_none());
    assert!(info.use_alt_speed_limits.is_none());
    assert!(info.refresh_interval.is_none());
}

#[test]
fn categories_decode_as_keyed_map_with_camelcase_savepath() {
    // The endpoint returns an object keyed by category name, not an array.
    let raw = json!({
        "movies": { "name": "movies", "savePath": "/data/movies" },
        "tv":     { "name": "tv",     "savePath": "/data/tv" }
    });
    let cats: HashMap<String, Category> = serde_json::from_value(raw).unwrap();
    assert_eq!(cats.len(), 2);
    let movies = &cats["movies"];
    assert_eq!(movies.name.as_deref(), Some("movies"));
    // camelCase key mapped to snake_case field.
    assert_eq!(movies.save_path.as_deref(), Some("/data/movies"));
}

#[test]
fn build_info_decodes_integer_bitness_and_ignores_unknown_fields() {
    let raw = json!({
        "qt": "6.7.0",
        "libtorrent": "2.0.10",
        "boost": "1.84.0",
        "openssl": "3.2.1",
        "bitness": 64,
        // Unknown upstream field must be ignored, not error.
        "future_field": "ignored"
    });
    let build: BuildInfo = serde_json::from_value(raw).unwrap();
    assert_eq!(build.qt.as_deref(), Some("6.7.0"));
    // bitness is an integer, not a string.
    assert_eq!(build.bitness, Some(64));
}

#[test]
fn empty_objects_decode_to_all_none() {
    let t: TorrentInfo = serde_json::from_value(json!({})).unwrap();
    assert!(t.hash.is_none());
    assert!(t.state.is_none());
    assert!(t.tags.is_none());

    let props: TorrentProperties = serde_json::from_value(json!({})).unwrap();
    assert!(props.save_path.is_none());
    assert!(props.is_private.is_none());

    let info: TransferInfo = serde_json::from_value(json!({})).unwrap();
    assert!(info.connection_status.is_none());

    let cat: Category = serde_json::from_value(json!({})).unwrap();
    assert!(cat.name.is_none());
    assert!(cat.save_path.is_none());

    let build: BuildInfo = serde_json::from_value(json!({})).unwrap();
    assert!(build.bitness.is_none());
}
