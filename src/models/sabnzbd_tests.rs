//! Colocated unit tests for the SABnzbd models.
//!
//! These decode representative `output=json` fixtures and assert the
//! serialisation quirks the module documents: string-encoded numerics decode
//! into `Option<String>`, genuine ints into `i64`, and unknown upstream fields
//! deserialise-and-ignore.

use super::*;
use serde_json::json;

#[test]
fn version_response_decodes() {
    let v: VersionResponse = serde_json::from_value(json!({ "version": "4.3.3" })).unwrap();
    assert_eq!(v.version.as_deref(), Some("4.3.3"));
}

#[test]
fn queue_response_decodes_string_numerics_and_int_counts() {
    let payload = json!({
        "queue": {
            "version": "4.3.3",
            "status": "Downloading",
            "paused": false,
            "paused_all": false,
            "pause_int": "0",
            "speed": "1.3 M",
            "kbpersec": "1234.5",
            "speedlimit": "100",
            "speedlimit_abs": "",
            "size": "1.2 GB",
            "sizeleft": "600.0 MB",
            "mb": "1234.56",
            "mbleft": "600.00",
            "timeleft": "0:05:00",
            "noofslots": 1,
            "noofslots_total": 3,
            "start": 0,
            "limit": 10,
            "finish": 0,
            "finishaction": null,
            "diskspace1": "161.20",
            "diskspace1_norm": "161.2 G",
            "have_warnings": "0",
            "have_quota": false,
            "cache_art": "12",
            "cache_size": "3.4 MB",
            "slots": [
                {
                    "nzo_id": "SABnzbd_nzo_abc123",
                    "filename": "Some.Release.2024",
                    "status": "Downloading",
                    "index": "0",
                    "percentage": "42",
                    "mb": "1234.56",
                    "mbleft": "700.00",
                    "mbmissing": "0",
                    "size": "1.2 GB",
                    "sizeleft": "700.0 MB",
                    "timeleft": "0:05:00",
                    "cat": "tv",
                    "priority": "Normal",
                    "unpackopts": "3",
                    "script": "None",
                    "password": "",
                    "avg_age": "2895d",
                    "direct_unpack": null,
                    "labels": ["DUPLICATE"],
                    "time_added": 1718000000
                }
            ]
        }
    });

    let resp: QueueResponse = serde_json::from_value(payload).unwrap();
    let q = resp.queue;

    // Booleans are real JSON bools.
    assert_eq!(q.paused, Some(false));
    assert_eq!(q.paused_all, Some(false));
    assert_eq!(q.have_quota, Some(false));

    // String-encoded numerics stay strings.
    assert_eq!(q.kbpersec.as_deref(), Some("1234.5"));
    assert_eq!(q.mb.as_deref(), Some("1234.56"));
    assert_eq!(q.diskspace1.as_deref(), Some("161.20"));
    assert_eq!(q.cache_art.as_deref(), Some("12"));

    // Genuine integral counts decode as i64.
    assert_eq!(q.noofslots, Some(1));
    assert_eq!(q.noofslots_total, Some(3));
    assert_eq!(q.start, Some(0));

    // Nullable finishaction.
    assert_eq!(q.finishaction, None);

    // Slot: string numerics + epoch int + labels Vec.
    assert_eq!(q.slots.len(), 1);
    let slot = &q.slots[0];
    assert_eq!(slot.nzo_id.as_deref(), Some("SABnzbd_nzo_abc123"));
    assert_eq!(slot.index.as_deref(), Some("0"));
    assert_eq!(slot.percentage.as_deref(), Some("42"));
    assert_eq!(slot.unpackopts.as_deref(), Some("3"));
    assert_eq!(slot.time_added, Some(1718000000));
    assert_eq!(slot.labels, vec!["DUPLICATE".to_string()]);
    assert_eq!(slot.direct_unpack, None);
}

#[test]
fn history_response_decodes_int_bytes_and_stage_log() {
    let payload = json!({
        "history": {
            "noofslots": 2,
            "ppslots": 0,
            "total_size": "3.4 T",
            "month_size": "120.0 G",
            "week_size": "30.0 G",
            "day_size": "5.0 G",
            "last_history_update": 1718000123,
            "version": "4.3.3",
            "slots": [
                {
                    "nzo_id": "SABnzbd_nzo_done1",
                    "name": "Some.Movie.2024",
                    "nzb_name": "Some.Movie.2024.nzb",
                    "status": "Completed",
                    "category": "movies",
                    "size": "1.2 G",
                    "bytes": 1288490188,
                    "downloaded": 1288490188,
                    "fail_message": "",
                    "storage": "/data/complete/movies/Some.Movie.2024",
                    "path": "/data/incomplete/Some.Movie.2024",
                    "download_time": 320,
                    "postproc_time": 45,
                    "completed": 1718000100,
                    "time_added": 1717999000,
                    "pp": "D",
                    "script": "None",
                    "report": null,
                    "duplicate_key": null,
                    "meta": null,
                    "completeness": null,
                    "loaded": false,
                    "archive": false,
                    "retry": false,
                    "stage_log": [
                        { "name": "Download", "actions": ["Downloaded in 5 min"] },
                        { "name": "Unpack", "actions": ["Unpacked 1 file"] }
                    ]
                }
            ]
        }
    });

    let resp: HistoryResponse = serde_json::from_value(payload).unwrap();
    let h = resp.history;

    assert_eq!(h.noofslots, Some(2));
    assert_eq!(h.ppslots, Some(0));
    assert_eq!(h.total_size.as_deref(), Some("3.4 T"));
    assert_eq!(h.last_history_update, Some(1718000123));

    assert_eq!(h.slots.len(), 1);
    let slot = &h.slots[0];
    // size is a formatted string; bytes/downloaded/times/completed are real ints.
    assert_eq!(slot.size.as_deref(), Some("1.2 G"));
    assert_eq!(slot.bytes, Some(1288490188));
    assert_eq!(slot.downloaded, Some(1288490188));
    assert_eq!(slot.download_time, Some(320));
    assert_eq!(slot.completed, Some(1718000100));
    assert_eq!(slot.pp.as_deref(), Some("D"));
    // retry is typed as a bool on the wire.
    assert_eq!(slot.retry, Some(false));
    // nullable fields decode to None.
    assert_eq!(slot.report, None);
    assert_eq!(slot.completeness, None);
    assert_eq!(slot.stage_log.len(), 2);
    assert_eq!(slot.stage_log[0].name.as_deref(), Some("Download"));
    assert_eq!(
        slot.stage_log[1].actions,
        vec!["Unpacked 1 file".to_string()]
    );
}

#[test]
fn extra_unknown_fields_are_ignored() {
    // Upstream may add fields we don't model; they must deserialise-and-ignore.
    let payload = json!({
        "queue": {
            "paused": true,
            "paused_all": false,
            "have_quota": false,
            "noofslots": 0,
            "noofslots_total": 0,
            "slots": [],
            "future_field_we_dont_model": "surprise",
            "another_unknown": { "nested": [1, 2, 3] }
        }
    });

    let resp: QueueResponse = serde_json::from_value(payload).unwrap();
    assert_eq!(resp.queue.paused, Some(true));
    assert_eq!(resp.queue.noofslots, Some(0));
    assert!(resp.queue.slots.is_empty());
}

#[test]
fn minimal_objects_default_to_none_and_empty_vecs() {
    // A queue with only its required fields: every Option is None, Vecs empty.
    let q: Queue = serde_json::from_value(json!({
        "paused": false,
        "paused_all": false,
        "have_quota": false,
        "noofslots": 0,
        "noofslots_total": 0
    }))
    .unwrap();
    assert_eq!(q.version, None);
    assert_eq!(q.status, None);
    assert_eq!(q.speed, None);
    assert_eq!(q.start, None);
    assert!(q.slots.is_empty());

    // A history slot with only nzo_id: every absent optional field is None, Vecs empty.
    let slot: HistorySlot = serde_json::from_value(json!({ "nzo_id": "x" })).unwrap();
    assert_eq!(slot.nzo_id.as_deref(), Some("x"));
    assert_eq!(slot.name, None);
    assert_eq!(slot.bytes, None);
    assert_eq!(slot.loaded, None);
    assert_eq!(slot.archive, None);
    assert_eq!(slot.retry, None);
    assert!(slot.stage_log.is_empty());

    // A history with only required fields.
    let h: History = serde_json::from_value(json!({ "noofslots": 0 })).unwrap();
    assert_eq!(h.noofslots, Some(0));
    assert_eq!(h.ppslots, None);
    assert!(h.slots.is_empty());
}
