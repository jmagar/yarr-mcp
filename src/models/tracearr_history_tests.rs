use super::*;
#[test]
fn terminate_success_and_error_arms_decode() {
    let ok: TerminateStreamOutcome = serde_json::from_value(json!({
        "success": true,
        "terminationLogId": "log-1",
        "message": "Stream termination command sent successfully"
    }))
    .unwrap();
    match ok {
        TerminateStreamOutcome::Success(ok) => {
            assert_eq!(ok.success, Some(true));
            assert_eq!(ok.termination_log_id.as_deref(), Some("log-1"));
        }
        TerminateStreamOutcome::Error(_) => panic!("success body decoded as error"),
    }

    let err: TerminateStreamOutcome = serde_json::from_value(json!({
        "success": false,
        "error": "Server unreachable",
        "terminationLogId": "log-2"
    }))
    .unwrap();
    match err {
        TerminateStreamOutcome::Error(err) => {
            assert_eq!(err.success, Some(false));
            assert_eq!(err.error.as_deref(), Some("Server unreachable"));
        }
        TerminateStreamOutcome::Success(_) => panic!("error body decoded as success"),
    }
}

#[test]
fn violation_rule_kind_decodes_known_and_unknown_values() {
    let known: ViolationRule = serde_json::from_value(json!({
        "id": "rule-1",
        "type": "concurrent_streams",
        "name": "Max 2 concurrent streams"
    }))
    .unwrap();
    assert_eq!(known.kind, Some(ViolationRuleKind::ConcurrentStreams));

    let unknown: ViolationRule = serde_json::from_value(json!({
        "id": "rule-2",
        "type": "new_rule_from_upstream",
        "name": "Future rule"
    }))
    .unwrap();
    assert_eq!(unknown.kind, Some(ViolationRuleKind::Unknown));
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
        viol.rule.as_ref().unwrap().kind,
        Some(ViolationRuleKind::ConcurrentStreams)
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
