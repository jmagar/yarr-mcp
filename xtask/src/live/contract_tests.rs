use super::*;

fn op_result(outcome: &'static str, detail: &str) -> OpResult {
    OpResult {
        name: "get_example",
        method: "GET",
        path: "/api/example",
        outcome,
        detail: detail.into(),
    }
}

#[test]
fn contract_status_fails_when_any_operation_is_rejected() {
    let results = vec![
        op_result("ok", "2xx + matches Example"),
        op_result("rejected", "sonarr request failed"),
    ];

    let status = contract_status(&results);

    assert!(!status.passed);
    assert_eq!(status.rejected, 1);
    assert!(
        status
            .detail
            .contains("1 contract-rejected (fails coverage)")
    );
}

#[test]
fn contract_status_passes_only_when_all_non_skipped_operations_are_exercised() {
    let results = vec![
        op_result("ok", "2xx + matches Example"),
        op_result("schema_mismatch", "live server drift"),
        op_result("skipped", "self-destructive control endpoint"),
    ];

    let status = contract_status(&results);

    assert!(status.passed);
    assert_eq!(status.ok, 1);
    assert_eq!(status.schema_mismatch, 1);
    assert_eq!(status.skipped, 1);
}

#[test]
fn transient_contract_errors_are_retryable() {
    for detail in [
        "sonarr request failed\n\nCaused by:\n    tcp connect error",
        "radarr request failed\n\nCaused by:\n    connection closed before message completed",
    ] {
        assert!(is_retryable_contract_error(detail), "{detail}");
    }
}

#[test]
fn upstream_validation_errors_and_wall_clock_timeouts_are_not_retryable() {
    for detail in [
        "sonarr returned HTTP 400 ({\"message\":\"seriesId must be provided\"})",
        "overseerr returned HTTP 403 ({\"error\":\"You do not have permission\"})",
        "plex returned HTTP 404 (<html>Not Found</html>)",
        "target/release/rustarr sonarr op get_indexer_schema --args {} timed out after 120s",
    ] {
        assert!(!is_retryable_contract_error(detail), "{detail}");
    }
}

#[test]
fn ui_feed_and_route_graph_endpoints_are_not_json_contracts() {
    for path in [
        "/login",
        "/logout",
        "/feed/v3/calendar/sonarr.ics",
        "/api/v3/system/routes",
        "/api/v1/system/routes",
    ] {
        assert!(is_known_non_contract_endpoint(path), "{path}");
    }

    assert!(!is_known_non_contract_endpoint("/api/v3/series"));
}

#[test]
fn optional_unseeded_feature_endpoints_are_skipped_by_kind() {
    for path in [
        "/LiveTv/Timers",
        "/SyncPlay/List",
        "/Items/RemoteSearch/Movie",
        "/QuickConnect/Connect",
    ] {
        assert!(
            is_unseeded_optional_feature_endpoint(ServiceKind::Jellyfin, path),
            "{path}"
        );
    }

    for path in [
        "/livetv/epg/channels",
        "/media/subscriptions",
        "/media/grabbers/devices",
        "/downloadQueue",
    ] {
        assert!(
            is_unseeded_optional_feature_endpoint(ServiceKind::Plex, path),
            "{path}"
        );
    }

    assert!(!is_unseeded_optional_feature_endpoint(
        ServiceKind::Jellyfin,
        "/System/Info/Public"
    ));
    assert!(!is_unseeded_optional_feature_endpoint(
        ServiceKind::Plex,
        "/identity"
    ));
}

#[test]
fn get_collection_operations_with_optional_resource_id_queries_seed_first() {
    let collection = OperationSpec {
        name: "get_series",
        method: HttpMethod::Get,
        path: "/api/v3/series",
        path_params: &[],
        query_params: &[],
        has_body: false,
        request_type: None,
        response_type: None,
        tag: "Series",
        summary: "",
    };
    let by_query_id = OperationSpec {
        name: "get_episode",
        method: HttpMethod::Get,
        path: "/api/v3/episode",
        path_params: &[],
        query_params: &["seriesId"],
        has_body: false,
        request_type: None,
        response_type: None,
        tag: "Episode",
        summary: "",
    };

    assert_eq!(seed_phase(&collection), 0);
    assert_eq!(seed_phase(&by_query_id), 0);
}
