//! Deserialization fixtures for the DownloadClient (SABnzbd/qBittorrent) models.

use super::*;
use serde_json::json;

#[test]
fn sab_queue_unwraps_slots_with_string_numbers() {
    let raw = json!({
        "queue": {
            "slots": [{
                "nzo_id": "SABnzbd_nzo_abc",
                "filename": "Some.Release",
                "status": "Downloading",
                "percentage": "96",
                "mb": "1024.00",
                "mbleft": "40.96",
                "timeleft": "0:00:30",
                "cat": "tv",
                "priority": "Normal"
            }],
            "speed": "5 M"
        }
    });
    let resp: SabQueueResponse = serde_json::from_value(raw).unwrap();
    let slots = resp.queue.expect("queue present").slots;
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].nzo_id.as_deref(), Some("SABnzbd_nzo_abc"));
    // SABnzbd reports these numerics as strings:
    assert_eq!(slots[0].percentage.as_deref(), Some("96"));
    assert_eq!(slots[0].mbleft.as_deref(), Some("40.96"));
}

#[test]
fn qbit_torrent_info_decodes() {
    let raw = json!({
        "hash": "0123abcd",
        "name": "Some.Movie.2024",
        "state": "downloading",
        "progress": 0.42,
        "dlspeed": 1048576,
        "size": 8123456789_i64,
        "category": "radarr",
        "ratio": 1.5
    });
    let t: TorrentInfo = serde_json::from_value(raw).unwrap();
    assert_eq!(t.hash.as_deref(), Some("0123abcd"));
    assert_eq!(t.progress, Some(0.42));
    assert_eq!(t.dlspeed, Some(1048576));
    assert_eq!(t.category.as_deref(), Some("radarr"));
}

#[test]
fn empty_sab_queue_defaults() {
    let resp: SabQueueResponse =
        serde_json::from_value(json!({ "queue": { "slots": [] } })).unwrap();
    assert!(resp.queue.unwrap().slots.is_empty());
}
