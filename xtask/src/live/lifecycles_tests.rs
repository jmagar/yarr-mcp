use super::*;
use serde_json::json;

#[test]
fn sab_ids_extracts_nzo_ids_from_slots() {
    let queue = json!({ "slots": [{ "nzo_id": "a" }, { "nzo_id": "b" }, { "other": 1 }] });
    let ids: Vec<String> = sab_ids(&queue).collect();
    assert_eq!(ids, vec!["a".to_string(), "b".to_string()]);
    // Missing/!array slots → empty, never panics.
    assert_eq!(sab_ids(&json!({})).count(), 0);
}

#[test]
fn torrent_hashes_extracts_hash_field() {
    let queue = json!([{ "hash": "AABB" }, { "hash": "ccdd" }, { "name": "x" }]);
    let hashes: Vec<&str> = torrent_hashes(&queue).collect();
    assert_eq!(hashes, vec!["AABB", "ccdd"]);
    assert_eq!(torrent_hashes(&json!({})).count(), 0);
}

#[test]
fn assert_sab_status_requires_status_and_id() {
    let ok = json!({ "status": true, "nzo_ids": ["x"] });
    assert!(assert_sab_status(&ok, "x", "pause").is_ok());
    // Wrong id rejected.
    assert!(assert_sab_status(&ok, "y", "pause").is_err());
    // status=false rejected.
    assert!(
        assert_sab_status(&json!({ "status": false, "nzo_ids": ["x"] }), "x", "pause").is_err()
    );
}

#[test]
fn parse_i64_output_reads_last_integer_line() {
    assert_eq!(parse_i64_output("3", "x").unwrap(), 3);
    assert_eq!(parse_i64_output("noise\n  7  \n", "x").unwrap(), 7);
    assert!(parse_i64_output("not-a-number", "x").is_err());
}
