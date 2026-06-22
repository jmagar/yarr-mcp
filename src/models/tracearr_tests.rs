//! Deserialization fixtures for the Tracearr public-API models.

use super::*;
use serde_json::json;

#[test]
fn health_response_decodes_with_servers() {
    let raw = json!({
        "status": "ok",
        "version": "1.4.22",
        "timestamp": "2024-01-15T12:00:00.000Z",
        "servers": [{
            "id": "11111111-1111-1111-1111-111111111111",
            "name": "Main Plex Server",
            "type": "plex",
            "online": true,
            "activeStreams": 2,
            // unknown upstream key must be ignored:
            "lastSeenAt": "2024-01-15T11:59:00.000Z"
        }],
        // unknown top-level key must be ignored:
        "uptimeSeconds": 999999
    });
    let health: HealthResponse = serde_json::from_value(raw).unwrap();
    assert_eq!(health.status.as_deref(), Some("ok"));
    assert_eq!(health.version.as_deref(), Some("1.4.22"));
    let server = &health.servers[0];
    assert_eq!(server.name.as_deref(), Some("Main Plex Server"));
    // reserved-word `type` -> `kind`
    assert_eq!(server.kind, Some(ServerType::Plex));
    assert_eq!(server.online, Some(true));
    assert_eq!(server.active_streams, Some(2));
}

#[test]
fn stats_today_decodes_float_watch_time() {
    let raw = json!({
        "activeStreams": 3,
        "todayPlays": 41,
        "watchTimeHours": 12.5,
        "alertsLast24h": 0,
        "activeUsersToday": 7,
        "timestamp": "2024-01-15T12:00:00.000Z"
    });
    let stats: StatsTodayResponse = serde_json::from_value(raw).unwrap();
    assert_eq!(stats.today_plays, Some(41));
    assert_eq!(stats.watch_time_hours, Some(12.5));
    assert_eq!(stats.active_users_today, Some(7));
}

#[test]
fn activity_decodes_buckets_and_enum_period() {
    let raw = json!({
        "period": "week",
        "range": { "start": "2024-01-08T00:00:00.000Z", "end": "2024-01-15T00:00:00.000Z" },
        "plays": [{ "date": "2024-01-08 00:00:00", "count": 12 }],
        "concurrent": [{
            "date": "2024-01-08 00:00:00",
            "total": 4, "direct": 2, "directStream": 1, "transcode": 1
        }],
        "byDayOfWeek": [{ "day": 5, "name": "Fri", "count": 9 }],
        "byHourOfDay": [{ "hour": 20, "count": 6 }],
        "platforms": [{ "platform": null, "count": 3 }],
        "quality": {
            "directPlay": 10, "directStream": 5, "transcode": 5, "total": 20,
            "directPlayPercent": 50, "directStreamPercent": 25, "transcodePercent": 25
        }
    });
    let act: ActivityResponse = serde_json::from_value(raw).unwrap();
    assert_eq!(act.period, Some(ActivityPeriod::Week));
    // non-ISO bucket string preserved verbatim
    assert_eq!(act.plays[0].date.as_deref(), Some("2024-01-08 00:00:00"));
    assert_eq!(act.concurrent[0].direct_stream, Some(1));
    assert_eq!(act.by_day_of_week[0].day, Some(5));
    assert_eq!(act.by_hour_of_day[0].hour, Some(20));
    // nullable platform: key present, value null -> None
    assert_eq!(act.platforms[0].platform, None);
    assert_eq!(act.platforms[0].count, Some(3));
    assert_eq!(act.quality.unwrap().transcode_percent, Some(25));
}

#[test]
fn stream_flattens_mixins_and_nested_details() {
    let raw = json!({
        "id": "session-1",
        // ServerInfo flattened at top level:
        "serverId": "srv-1",
        "serverName": "Main Plex Server",
        "username": "John Doe",
        "userThumb": null,
        "userAvatarUrl": null,
        // MediaInfo flattened at top level:
        "mediaTitle": "Inception",
        "mediaType": "movie",
        "showTitle": null,
        "seasonNumber": null,
        "year": 2010,
        "durationMs": 8880000,
        "state": "playing",
        "progressMs": 1200000,
        "startedAt": "2024-01-15T12:00:00.000Z",
        // StreamDetails flattened at top level:
        "isTranscode": true,
        "videoDecision": "transcode",
        "audioDecision": "directplay",
        "bitrate": 12000,
        "sourceVideoCodec": "hevc",
        "sourceVideoDetails": {
            "bitrate": 25000.0,
            "framerate": "23.976",
            "dynamicRange": "HDR10",
            "aspectRatio": 1.78,
            "colorDepth": 10.0
        },
        "transcodeInfo": {
            "containerDecision": "transcode",
            "sourceContainer": "mkv",
            "speed": 2.5,
            "throttled": false,
            "reasons": ["video bitrate too high"]
        },
        // DisplayValues flattened at top level:
        "resolution": "4K",
        "sourceVideoCodecDisplay": "HEVC",
        // DeviceInfo flattened at top level:
        "device": "Apple TV",
        "player": "Infuse",
        "platform": "tvOS",
        // unknown extra key ignored:
        "sessionKey": "abc123"
    });
    let stream: Stream = serde_json::from_value(raw).unwrap();
    assert_eq!(stream.id.as_deref(), Some("session-1"));
    assert_eq!(stream.server_info.server_id.as_deref(), Some("srv-1"));
    assert_eq!(stream.media_info.media_title.as_deref(), Some("Inception"));
    assert_eq!(stream.media_info.media_type, Some(MediaType::Movie));
    assert_eq!(stream.state, Some(PlaybackState::Playing));
    assert_eq!(
        stream.stream_details.video_decision,
        Some(TranscodeDecision::Transcode)
    );
    assert_eq!(
        stream.stream_details.audio_decision,
        Some(TranscodeDecision::Directplay)
    );
    let svd = stream
        .stream_details
        .source_video_details
        .expect("source video details present");
    assert_eq!(svd.aspect_ratio, Some(1.78));
    assert_eq!(svd.framerate.as_deref(), Some("23.976"));
    let ti = stream
        .stream_details
        .transcode_info
        .expect("transcode info present");
    assert_eq!(ti.speed, Some(2.5));
    assert_eq!(ti.reasons, vec!["video bitrate too high"]);
    assert_eq!(stream.display_values.resolution.as_deref(), Some("4K"));
    assert_eq!(stream.device_info.device.as_deref(), Some("Apple TV"));
}

#[test]
fn streams_response_summary_only_arm_has_no_data() {
    let raw = json!({
        // union quirk: summary=true omits `data` entirely
        "summary": {
            "total": 5,
            "transcodes": 2,
            "directStreams": 1,
            "directPlays": 2,
            "totalBitrate": "45.2 Mbps",
            "byServer": [{
                "serverId": "srv-1",
                "serverName": "Main Plex Server",
                "total": 5,
                "transcodes": 2,
                "directStreams": 1,
                "directPlays": 2,
                "totalBitrate": "22.5 Mbps"
            }]
        }
    });
    let resp: StreamsResponse = serde_json::from_value(raw).unwrap();
    assert!(resp.data.is_none());
    let summary = resp.summary.expect("summary present");
    // totalBitrate is a pre-formatted string, not numeric
    assert_eq!(summary.total_bitrate.as_deref(), Some("45.2 Mbps"));
    assert_eq!(
        summary.by_server[0].server_info.server_id.as_deref(),
        Some("srv-1")
    );
    assert_eq!(
        summary.by_server[0].total_bitrate.as_deref(),
        Some("22.5 Mbps")
    );
}

#[test]
fn terminate_success_and_error_arms_decode() {
    let ok: TerminateStreamResponse = serde_json::from_value(json!({
        "success": true,
        "terminationLogId": "log-1",
        "message": "Stream termination command sent successfully"
    }))
    .unwrap();
    assert_eq!(ok.success, Some(true));
    assert_eq!(ok.termination_log_id.as_deref(), Some("log-1"));

    let err: TerminateStreamErrorResponse = serde_json::from_value(json!({
        "success": false,
        "error": "Server unreachable",
        "terminationLogId": "log-2"
    }))
    .unwrap();
    assert_eq!(err.success, Some(false));
    assert_eq!(err.error.as_deref(), Some("Server unreachable"));
}

#[test]
fn users_response_envelope_and_flattened_server_info() {
    let raw = json!({
        "data": [{
            "id": "user-1",
            "username": "john_doe",
            "displayName": "John Doe",
            "thumbUrl": null,
            "avatarUrl": null,
            "role": "owner",
            "trustScore": 95,
            "totalViolations": 1,
            "serverId": "srv-1",
            "serverName": "Main Plex Server",
            "lastActivityAt": "2024-01-15T11:00:00.000Z",
            "sessionCount": 120,
            "createdAt": "2022-06-01T00:00:00.000Z"
        }],
        "meta": { "total": 1, "page": 1, "pageSize": 25 }
    });
    let resp: UsersResponse = serde_json::from_value(raw).unwrap();
    let user = &resp.data[0];
    assert_eq!(user.username.as_deref(), Some("john_doe"));
    assert_eq!(user.role, Some(UserRole::Owner));
    assert_eq!(user.trust_score, Some(95));
    assert_eq!(
        user.server_info.server_name.as_deref(),
        Some("Main Plex Server")
    );
    let meta = resp.meta.expect("meta present");
    assert_eq!(meta.total, Some(1));
    assert_eq!(meta.page_size, Some(25));
}

#[test]
fn violation_decodes_arbitrary_data_and_renamed_rule_type() {
    let raw = json!({
        "data": [{
            "id": "viol-1",
            "serverId": "srv-1",
            "serverName": "Main Plex Server",
            "severity": "high",
            "acknowledged": false,
            "data": { "streamCount": 3, "limit": 2 },
            "createdAt": "2024-01-15T10:00:00.000Z",
            "rule": {
                "id": "rule-1",
                "type": "concurrent_streams",
                "name": "Max 2 concurrent streams"
            },
            "user": {
                "id": "user-1",
                "username": "john_doe",
                "thumbUrl": null,
                "avatarUrl": null
            }
        }],
        "meta": { "total": 1, "page": 1, "pageSize": 25 }
    });
    let resp: ViolationsResponse = serde_json::from_value(raw).unwrap();
    let viol = &resp.data[0];
    assert_eq!(viol.severity, Some(Severity::High));
    // arbitrary JSON object preserved
    assert_eq!(viol.data.as_ref().unwrap()["streamCount"], json!(3));
    // reserved-word `type` -> `kind` on the inline rule
    assert_eq!(
        viol.rule.as_ref().unwrap().kind.as_deref(),
        Some("concurrent_streams")
    );
    assert_eq!(
        viol.user.as_ref().unwrap().username.as_deref(),
        Some("john_doe")
    );
}

#[test]
fn session_history_decodes_with_nullable_stopped_at() {
    let raw = json!({
        "data": [{
            "id": "hist-1",
            "serverId": "srv-1",
            "serverName": "Main Plex Server",
            "state": "stopped",
            "mediaTitle": "Breaking Bad",
            "mediaType": "episode",
            "showTitle": "Breaking Bad",
            "seasonNumber": 1,
            "episodeNumber": 1,
            "durationMs": 2820000,
            "progressMs": 2820000,
            "totalDurationMs": 2820000,
            "startedAt": "2024-01-14T20:00:00.000Z",
            "stoppedAt": null,
            "watched": true,
            "segmentCount": 2,
            "device": "Living Room TV",
            "user": { "id": "user-1", "username": "john_doe", "thumbUrl": null, "avatarUrl": null }
        }],
        "meta": { "total": 1, "page": 1, "pageSize": 25 }
    });
    let resp: HistoryResponse = serde_json::from_value(raw).unwrap();
    let hist = &resp.data[0];
    assert_eq!(hist.state, Some(PlaybackState::Stopped));
    assert_eq!(hist.media_info.media_type, Some(MediaType::Episode));
    assert_eq!(hist.media_info.show_title.as_deref(), Some("Breaking Bad"));
    assert_eq!(hist.stopped_at, None);
    assert_eq!(hist.watched, Some(true));
    assert_eq!(hist.segment_count, Some(2));
    assert_eq!(
        hist.user.as_ref().unwrap().username.as_deref(),
        Some("john_doe")
    );
}

#[test]
fn empty_object_yields_all_none_and_empty_vecs() {
    let health: HealthResponse = serde_json::from_value(json!({})).unwrap();
    assert!(health.status.is_none());
    assert!(health.version.is_none());
    assert!(health.servers.is_empty());

    let act: ActivityResponse = serde_json::from_value(json!({})).unwrap();
    assert!(act.period.is_none());
    assert!(act.range.is_none());
    assert!(act.plays.is_empty());
    assert!(act.concurrent.is_empty());
    assert!(act.platforms.is_empty());
    assert!(act.quality.is_none());

    let resp: StreamsResponse = serde_json::from_value(json!({})).unwrap();
    assert!(resp.data.is_none());
    assert!(resp.summary.is_none());

    // flattened-mixin struct with no keys: every field None
    let stream: Stream = serde_json::from_value(json!({})).unwrap();
    assert!(stream.id.is_none());
    assert!(stream.server_info.server_id.is_none());
    assert!(stream.media_info.media_title.is_none());
    assert!(stream.stream_details.is_transcode.is_none());
    assert!(stream.display_values.resolution.is_none());
    assert!(stream.device_info.device.is_none());
}
