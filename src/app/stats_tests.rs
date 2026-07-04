use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::yarr::{RustarrClient, query_get, slim};
use serde_json::json;

use super::{
    HISTORY_FIELDS, LIBRARY_FIELDS, SESSION_FIELDS, TAUTULLI_API, USER_FIELDS, unwrap_tautulli,
};

fn service_with(kinds: &[(&str, ServiceKind)]) -> RustarrService {
    let services = kinds
        .iter()
        .map(|(name, kind)| ServiceConfig {
            name: (*name).into(),
            kind: *kind,
            base_url: "http://localhost:1".into(),
            api_key: Some("secret".into()),
            ..ServiceConfig::default()
        })
        .collect();
    let config = RustarrConfig { services };
    let client = RustarrClient::new(&config).expect("client builds");
    RustarrService::new(client, config)
}

/// Drive an async op to completion on a fresh current-thread runtime so a sync
/// test can hold [`crate::testing::ENV_LOCK`] across the run (the destructive gate
/// reads `YARR_ALLOW_DESTRUCTIVE`) without holding the lock across `.await`.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

fn tautulli_config() -> ServiceConfig {
    ServiceConfig {
        name: "tautulli".into(),
        kind: ServiceKind::Tautulli,
        base_url: "http://localhost:8181".into(),
        api_key: Some("secret".into()),
        ..ServiceConfig::default()
    }
}

// ── cmd= query building (via query_get) ──────────────────────────────────────────

#[test]
fn activity_uses_cmd_query_on_api_v2() {
    // Tautulli is a ?cmd= query API — never a REST resource path.
    let url = query_get(&tautulli_config(), TAUTULLI_API, &[("cmd", "get_activity")])
        .expect("url builds");
    let query = url.query().expect("query present");
    assert!(query.contains("cmd=get_activity"), "got: {query}");
    assert_eq!(url.path(), "/api/v2");
}

#[test]
fn query_get_injects_apikey_exactly_once() {
    // apikey is injected by query_get's shared query-auth path; the app layer must
    // NEVER add it again. Assert it appears at most once (here: exactly once).
    let url =
        query_get(&tautulli_config(), TAUTULLI_API, &[("cmd", "get_users")]).expect("url builds");
    let query = url.query().expect("query present");
    assert_eq!(
        query.matches("apikey=").count(),
        1,
        "apikey must appear exactly once (no double key): {query}"
    );
    assert!(query.contains("apikey=secret"), "got: {query}");
}

#[test]
fn history_params_are_percent_encoded_no_cmd_injection() {
    // A user value with reserved chars must be percent-encoded by query_get, never
    // format!'d into the path (S6: no second cmd= injection).
    let url = query_get(
        &tautulli_config(),
        TAUTULLI_API,
        &[
            ("cmd", "get_history"),
            ("user", "evil&cmd=delete_all"),
            ("start", "0"),
            ("length", "25"),
        ],
    )
    .expect("url builds");
    let query = url.query().expect("query present");
    // Exactly one cmd= pair — the injected `&cmd=delete_all` is encoded.
    assert_eq!(
        query.matches("cmd=").count(),
        1,
        "single cmd pair (injection blocked): {query}"
    );
    assert!(
        query.contains("%26") || query.contains("%3D"),
        "reserved chars must be percent-encoded: {query}"
    );
    assert!(
        query.contains("start=0") && query.contains("length=25"),
        "got: {query}"
    );
    // apikey still injected exactly once even with extra params.
    assert_eq!(query.matches("apikey=").count(), 1, "got: {query}");
}

#[test]
fn libraries_use_inventory_command_not_analytics_datatable() {
    let url = query_get(
        &tautulli_config(),
        TAUTULLI_API,
        &[("cmd", "get_library_names")],
    )
    .expect("url builds");
    let query = url.query().expect("query present");
    assert!(
        query.contains("cmd=get_library_names"),
        "library inventory should use stable get_library_names command: {query}"
    );
    assert!(
        !query.contains("cmd=get_libraries"),
        "get_libraries is the analytics datatable endpoint: {query}"
    );
}

// ── envelope unwrap ──────────────────────────────────────────────────────────────

#[test]
fn unwrap_returns_data_on_success() {
    let raw = json!({
        "response": {
            "result": "success",
            "data": { "stream_count": 2 },
            "message": null
        }
    });
    let data = unwrap_tautulli(raw).expect("success envelope unwraps");
    assert_eq!(data["stream_count"], 2);
}

#[test]
fn unwrap_errors_on_failure_result_surfacing_message() {
    let raw = json!({
        "response": {
            "result": "error",
            "data": null,
            "message": "Invalid apikey"
        }
    });
    let err = unwrap_tautulli(raw).expect_err("error result must fail");
    assert!(
        err.to_string().contains("Invalid apikey"),
        "should surface upstream message, got: {err}"
    );
}

#[test]
fn unwrap_errors_on_missing_envelope() {
    let raw = json!({ "not_a_response": true });
    let err = unwrap_tautulli(raw).expect_err("missing envelope must fail");
    assert!(err.to_string().contains("envelope"), "got: {err}");
}

// ── slim ──────────────────────────────────────────────────────────────────────

#[test]
fn session_slim_keeps_expected_fields() {
    let sessions = json!([{
        "user": "jacob",
        "full_title": "The Matrix",
        "title": "The Matrix",
        "state": "playing",
        "progress_percent": "42",
        "media_type": "movie",
        "stream_bitrate": "20000",
        "ip_address": "10.0.0.5"
    }]);
    let slimmed = slim(sessions, SESSION_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["user"], "jacob");
    assert_eq!(row["full_title"], "The Matrix");
    assert_eq!(row["progress_percent"], "42");
    assert!(row.get("stream_bitrate").is_none(), "bulky fields dropped");
    assert!(row.get("ip_address").is_none(), "PII-ish fields dropped");
}

#[test]
fn history_slim_keeps_expected_fields() {
    let rows = json!([{
        "date": 1700000000,
        "user": "jacob",
        "full_title": "Dune",
        "title": "Dune",
        "media_type": "movie",
        "watched_status": 1,
        "percent_complete": 100,
        "row_id": 999,
        "session_key": "drop me"
    }]);
    let slimmed = slim(rows, HISTORY_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["user"], "jacob");
    assert_eq!(row["watched_status"], 1);
    assert!(row.get("session_key").is_none(), "bulky fields dropped");
}

#[test]
fn user_and_library_slim_keep_expected_fields() {
    let users = json!([{ "user_id": 1, "username": "jacob", "plays": 42, "email": "drop" }]);
    let su = slim(users, USER_FIELDS);
    assert_eq!(su[0]["username"], "jacob");
    assert!(su[0].get("email").is_none());

    let libs = json!([{
        "section_id": 1, "section_name": "Movies", "section_type": "movie",
        "count": 500, "agent": "tv.plex.agents.movie", "thumb": "drop"
    }]);
    let sl = slim(libs, LIBRARY_FIELDS);
    assert_eq!(sl[0]["section_name"], "Movies");
    assert_eq!(sl[0]["agent"], "tv.plex.agents.movie");
    assert!(sl[0].get("thumb").is_none());
}

// ── wrong-kind reject ─────────────────────────────────────────────────────────

#[tokio::test]
async fn stats_activity_rejects_non_stats_kind() {
    // sonarr is ArrManager, not Stats — capability check rejects before any request.
    let svc = service_with(&[("sonarr", ServiceKind::Sonarr)]);
    let err = svc
        .stats_activity("sonarr")
        .await
        .expect_err("stats_activity on sonarr must be rejected");
    assert!(
        err.to_string().contains("Stats"),
        "error should mention the Stats capability, got: {err}"
    );
}

#[tokio::test]
async fn stats_history_rejects_non_stats_kind() {
    let svc = service_with(&[("plex", ServiceKind::Plex)]);
    let err = svc
        .stats_history("plex", Some(0), Some(25), None)
        .await
        .expect_err("stats_history on plex must be rejected");
    assert!(err.to_string().contains("Stats"));
}

// Sync + `ENV_LOCK` (see `block_on` above): the destructive confirm gate reads
// `YARR_ALLOW_DESTRUCTIVE`, so serialise against env-mutating tests to avoid a
// concurrent test lifting the gate and racing this assertion to "request failed".
#[test]
fn delete_image_cache_requires_confirm() {
    let _g = crate::testing::ENV_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::remove_var("YARR_ALLOW_DESTRUCTIVE") };
    // delete_image_cache is destructive, so the confirm gate runs before the
    // capability/transport: an unreachable tautulli still surfaces confirm first.
    let svc = service_with(&[("tautulli", ServiceKind::Tautulli)]);
    let err = block_on(svc.stats_delete_image_cache("tautulli", false))
        .expect_err("stats_delete_image_cache without confirm must reject");
    assert!(err.to_string().contains("confirm"), "got: {err}");
}

#[tokio::test]
async fn refresh_commands_reject_non_stats_kind() {
    // The refreshes are non-destructive and ungated now, so the capability check
    // is the first gate: a non-stats kind (plex) is rejected before any request is
    // built.
    let svc = service_with(&[("plex", ServiceKind::Plex)]);
    for result in [
        svc.stats_refresh_libraries("plex").await,
        svc.stats_refresh_users("plex").await,
    ] {
        let err = result.expect_err("refresh on plex must reject");
        assert!(err.to_string().contains("Stats"), "got: {err}");
    }
}
