//! Deserialization fixtures for the Tautulli models.

use super::*;
use serde_json::json;

#[test]
fn activity_envelope_decodes_with_string_stream_count() {
    // Verbatim-shaped get_activity: stream_count + per-session numerics are
    // STRINGS; reserved `type` maps to `kind`.
    let raw = json!({
        "response": {
            "result": "success",
            "message": null,
            "data": {
                "stream_count": "1",
                "stream_count_direct_play": 1,
                "stream_count_transcode": 0,
                "total_bandwidth": 10617,
                "sessions": [{
                    "session_key": "12",
                    "user": "LordCommanderSnow",
                    "full_title": "Game of Thrones - The Red Woman",
                    "title": "The Red Woman",
                    "media_type": "episode",
                    "state": "playing",
                    "progress_percent": "0",
                    "view_offset": "1000",
                    "duration": "2998272",
                    "bitrate": "10617",
                    "type": "episode"
                }]
            }
        }
    });
    let env: TautulliEnvelope<GetActivityData> = serde_json::from_value(raw).unwrap();
    let resp = env.response.expect("response present");
    assert_eq!(resp.result(), "success");
    assert_eq!(resp.message(), None);
    let data = resp.data().expect("data present");
    assert_eq!(data.stream_count.as_deref(), Some("1"));
    assert_eq!(data.stream_count_direct_play, Some(1));
    assert_eq!(data.total_bandwidth, Some(10617));
    let session = &data.sessions[0];
    assert_eq!(session.user.as_deref(), Some("LordCommanderSnow"));
    assert_eq!(session.progress_percent.as_deref(), Some("0"));
    assert_eq!(session.view_offset.as_deref(), Some("1000"));
    assert_eq!(session.duration.as_deref(), Some("2998272"));
    assert_eq!(session.media_type, Some(TautulliMediaType::Episode));
    assert_eq!(session.state, Some(TautulliPlaybackState::Playing));
    // Reserved word `type` decodes into `kind`.
    assert_eq!(session.kind, Some(TautulliMediaType::Episode));
}

#[test]
fn failed_command_envelope_surfaces_message_and_null_data() {
    let raw = json!({
        "response": { "result": "error", "message": "Invalid apikey", "data": null }
    });
    let env: TautulliEnvelope<GetActivityData> = serde_json::from_value(raw).unwrap();
    let resp = env.response.unwrap();
    assert_eq!(resp.result(), "error");
    assert_eq!(resp.message(), Some("Invalid apikey"));
    assert!(resp.data().is_none());
}

#[test]
fn history_data_decodes_mixed_casing_and_epoch_ints() {
    // get_history is a DataTables payload: camelCase recordsTotal/recordsFiltered,
    // snake_case STRING total_duration, epoch-int date/started/stopped, and
    // watched_status carrying a fractional partial-watch value.
    let raw = json!({
        "draw": 1,
        "recordsTotal": 1000,
        "recordsFiltered": 250,
        "total_duration": "2 days 22 hrs",
        "filter_duration": "1 hr 30 mins",
        "data": [{
            "row_id": 1007,
            "date": 1_462_687_607_i64,
            "started": 1_462_687_606_i64,
            "stopped": 1_462_687_870_i64,
            "user": "DanyKhaleesi69",
            "user_id": 8_008_135,
            "full_title": "Game of Thrones - The Red Woman",
            "title": "The Red Woman",
            "media_type": "episode",
            "watched_status": 0.5,
            "percent_complete": 84,
            "duration": 2_998_290_i64,
            "play_duration": 263,
            "transcode_decision": "transcode",
            "year": 2016,
            "live": 0
        }]
    });
    let data: GetHistoryData = serde_json::from_value(raw).unwrap();
    assert_eq!(data.draw, Some(1));
    assert_eq!(data.records_total, Some(1000));
    assert_eq!(data.records_filtered, Some(250));
    assert_eq!(data.total_duration.as_deref(), Some("2 days 22 hrs"));
    let row = &data.data[0];
    assert_eq!(row.date, Some(1_462_687_607));
    assert_eq!(row.stopped, Some(1_462_687_870));
    assert_eq!(row.user_id, Some(8_008_135));
    // Fractional partial-watch value round-trips through f64.
    assert_eq!(row.watched_status, Some(0.5));
    assert_eq!(row.percent_complete, Some(84));
    assert_eq!(row.media_type, Some(TautulliMediaType::Episode));
    assert_eq!(
        row.transcode_decision,
        Some(TautulliTranscodeDecision::Transcode)
    );
    assert_eq!(row.year, Some(2016));
}

#[test]
fn user_row_ignores_unknown_fields() {
    // get_users_table row carrying an extra, undocumented key — it must
    // deserialize-and-ignore rather than fail.
    let raw = json!({
        "row_id": 1,
        "user_id": 8_008_135,
        "username": "DanyKhaleesi69",
        "friendly_name": "Dany",
        "email": "dany@example.com",
        "user_thumb": "https://plex.tv/users/abc/avatar",
        "plays": 42,
        "duration": 123_456,
        "last_seen": 1_462_687_607_i64,
        "is_active": 1,
        "shared_libraries": ["1", "2", "3"],
        "an_undocumented_future_field": "ignored",
        "another": { "nested": true }
    });
    let user: TautulliUserRow = serde_json::from_value(raw).unwrap();
    assert_eq!(user.username.as_deref(), Some("DanyKhaleesi69"));
    assert_eq!(user.email.as_deref(), Some("dany@example.com"));
    assert_eq!(user.plays, Some(42));
    assert_eq!(user.last_seen, Some(1_462_687_607));
    assert_eq!(user.shared_libraries, vec!["1", "2", "3"]);
}

#[test]
fn libraries_decode_string_counts_vs_int_section_id() {
    // get_libraries: section_id/count/parent_count/child_count are STRINGS.
    let lib: LibraryRow = serde_json::from_value(json!({
        "section_id": "2",
        "section_name": "TV Shows",
        "section_type": "show",
        "count": "62",
        "parent_count": "240",
        "child_count": "3745",
        "is_active": 1
    }))
    .unwrap();
    assert_eq!(lib.section_id.as_deref(), Some("2"));
    assert_eq!(lib.count.as_deref(), Some("62"));
    assert_eq!(lib.child_count.as_deref(), Some("3745"));
    assert_eq!(lib.is_active, Some(1));
    assert_eq!(lib.agent, None);

    // get_library_names: section_id is a real int here.
    let name: LibraryNameRow = serde_json::from_value(json!({
        "section_id": 2,
        "section_name": "TV Shows",
        "section_type": "show"
    }))
    .unwrap();
    assert_eq!(name.section_id, Some(2));
    assert_eq!(name.section_type.as_deref(), Some("show"));
}

#[test]
fn empty_objects_yield_all_none_and_empty_vecs() {
    // Empty payloads must decode with every field None and every Vec empty.
    let activity: GetActivityData = serde_json::from_value(json!({})).unwrap();
    assert_eq!(activity.stream_count, None);
    assert_eq!(activity.total_bandwidth, None);
    assert!(activity.sessions.is_empty());

    let history: GetHistoryData = serde_json::from_value(json!({})).unwrap();
    assert_eq!(history.records_total, None);
    assert_eq!(history.total_duration, None);
    assert!(history.data.is_empty());

    let session: StreamSession = serde_json::from_value(json!({})).unwrap();
    assert_eq!(session.user, None);
    assert_eq!(session.kind, None);

    let user: TautulliUserRow = serde_json::from_value(json!({})).unwrap();
    assert_eq!(user.username, None);
    assert_eq!(user.plays, None);
    assert!(user.shared_libraries.is_empty());

    let server: GetServerInfoData = serde_json::from_value(json!({})).unwrap();
    assert_eq!(server.pms_name, None);
    assert_eq!(server.pms_port, None);
}
