//! Deserialization fixtures for the Stats (Tautulli) models.

use super::*;
use serde_json::json;

#[test]
fn activity_envelope_decodes_with_string_stream_count() {
    let raw = json!({
        "response": {
            "result": "success",
            "message": null,
            "data": {
                "stream_count": "2",
                "sessions": [{
                    "user": "alice",
                    "full_title": "The Matrix",
                    "title": "The Matrix",
                    "state": "playing",
                    "progress_percent": "37",
                    "media_type": "movie"
                }]
            }
        }
    });
    let env: TautulliEnvelope<Activity> = serde_json::from_value(raw).unwrap();
    let resp = env.response.expect("response present");
    assert_eq!(resp.result.as_deref(), Some("success"));
    let data = resp.data.expect("data present");
    assert_eq!(data.stream_count.as_deref(), Some("2"));
    assert_eq!(data.sessions[0].user.as_deref(), Some("alice"));
    assert_eq!(data.sessions[0].progress_percent.as_deref(), Some("37"));
}

#[test]
fn failed_command_envelope_surfaces_message() {
    let raw = json!({
        "response": { "result": "error", "message": "Invalid apikey", "data": null }
    });
    let env: TautulliEnvelope<Activity> = serde_json::from_value(raw).unwrap();
    let resp = env.response.unwrap();
    assert_eq!(resp.result.as_deref(), Some("error"));
    assert_eq!(resp.message.as_deref(), Some("Invalid apikey"));
    assert!(resp.data.is_none());
}

#[test]
fn history_page_decodes_epoch_date() {
    let raw = json!({
        "records_total": 100,
        "records_filtered": 100,
        "data": [{
            "date": 1_700_000_000_i64,
            "user": "bob",
            "full_title": "Dune - Part Two",
            "title": "Dune - Part Two",
            "media_type": "movie",
            "watched_status": 1.0,
            "percent_complete": 100
        }]
    });
    let page: HistoryPage = serde_json::from_value(raw).unwrap();
    assert_eq!(page.records_total, Some(100));
    let row = &page.data[0];
    assert_eq!(row.date, Some(1_700_000_000));
    assert_eq!(row.watched_status, Some(1.0));
    assert_eq!(row.percent_complete, Some(100));
}

#[test]
fn users_and_libraries_decode() {
    let user: TautulliUser =
        serde_json::from_value(json!({ "user_id": 1, "username": "alice", "plays": 42 })).unwrap();
    assert_eq!(user.username.as_deref(), Some("alice"));
    assert_eq!(user.plays, Some(42));

    let lib: LibraryName = serde_json::from_value(json!({
        "section_id": 1, "section_name": "Movies", "section_type": "movie",
        "agent": "tv.plex.agents.movie", "count": 500, "parent_count": 0, "child_count": 0
    }))
    .unwrap();
    assert_eq!(lib.section_name.as_deref(), Some("Movies"));
    assert_eq!(lib.count, Some(500));
}
