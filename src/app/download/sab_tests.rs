use crate::config::{ServiceConfig, ServiceKind};
use crate::rustarr::{query_get, slim};
use serde_json::json;

use super::{QUEUE_FIELDS, SAB_API};

fn sab_config() -> ServiceConfig {
    ServiceConfig {
        name: "sabnzbd".into(),
        kind: ServiceKind::Sabnzbd,
        base_url: "http://localhost:8080".into(),
        api_key: Some("secret".into()),
        ..ServiceConfig::default()
    }
}

#[test]
fn queue_uses_mode_query_and_json_output() {
    // SAB is a ?mode= query API — never a /api/v2 REST path.
    let url = query_get(&sab_config(), SAB_API, &[("mode", "queue")]).expect("url builds");
    let query = url.query().expect("query present");
    assert!(
        query.contains("mode=queue"),
        "expected mode=queue, got: {query}"
    );
    assert!(
        query.contains("output=json"),
        "SAB forces output=json: {query}"
    );
    assert!(query.contains("apikey=secret"), "apikey injected: {query}");
    assert_eq!(url.path(), "/api");
}

#[test]
fn add_uses_addurl_mode_with_percent_encoded_name() {
    // A url with reserved chars must be percent-encoded by query_get, never
    // format!'d into the path (S6: no second-parameter injection).
    let url = query_get(
        &sab_config(),
        SAB_API,
        &[("mode", "addurl"), ("name", "http://x/a?b=c&d=e")],
    )
    .expect("url builds");
    let query = url.query().expect("query present");
    assert!(query.contains("mode=addurl"), "got: {query}");
    // The raw `&d=e` must be encoded so it cannot become its own query pair.
    assert!(query.contains("name=http"), "name param present: {query}");
    assert!(
        query.contains("%3F") || query.contains("%26"),
        "reserved chars must be percent-encoded: {query}"
    );
    // Exactly one `mode=` pair — no injection of a second mode.
    assert_eq!(
        query.matches("mode=").count(),
        1,
        "single mode pair: {query}"
    );
}

#[test]
fn remove_with_delete_files_sends_del_files_flag() {
    let with = query_get(
        &sab_config(),
        SAB_API,
        &[
            ("mode", "queue"),
            ("name", "delete"),
            ("value", "SABnzbd_nzo_x"),
            ("del_files", "1"),
        ],
    )
    .expect("url builds");
    assert!(with.query().unwrap().contains("del_files=1"));

    // Default (opt-out): no del_files pair.
    let without = query_get(
        &sab_config(),
        SAB_API,
        &[
            ("mode", "queue"),
            ("name", "delete"),
            ("value", "SABnzbd_nzo_x"),
        ],
    )
    .expect("url builds");
    assert!(!without.query().unwrap().contains("del_files"));
}

#[test]
fn queue_slim_keeps_expected_fields() {
    let slots = json!([{
        "nzo_id": "SABnzbd_nzo_a",
        "filename": "Ubuntu.iso",
        "status": "Downloading",
        "percentage": "42",
        "mb": "700",
        "mbleft": "406",
        "timeleft": "0:05:00",
        "cat": "software",
        "priority": "Normal",
        "internal": "drop me"
    }]);
    let slimmed = slim(slots, QUEUE_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["nzo_id"], "SABnzbd_nzo_a");
    assert_eq!(row["filename"], "Ubuntu.iso");
    assert_eq!(row["percentage"], "42");
    assert!(row.get("internal").is_none(), "bulky fields dropped");
}
