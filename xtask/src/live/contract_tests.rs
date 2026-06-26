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
