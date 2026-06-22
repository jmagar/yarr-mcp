use rustarr::{RustarrAction, execute_tool_without_peer_for_test, testing::loopback_state};
use serde_json::json;

async fn call_mcp_action(args: serde_json::Value) -> serde_json::Value {
    let state = loopback_state();
    execute_tool_without_peer_for_test(&state, "sonarr", args)
        .await
        .expect("MCP tool dispatch should succeed")
}

#[tokio::test]
async fn integrations_returns_supported_services() {
    let result = call_mcp_action(json!({ "action": "integrations" })).await;
    // `supported` is now a list of {kind, capability} objects (registry-derived).
    assert!(
        result["supported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["kind"] == "sonarr")
    );
}

#[test]
fn service_status_action_parses_for_mcp_dispatch() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "service_status",
        "service": "sonarr"
    }))
    .expect("service_status should parse");
    assert!(matches!(action, RustarrAction::ServiceStatus { .. }));
}

#[tokio::test]
async fn help_returns_text() {
    let result = call_mcp_action(json!({ "action": "help" })).await;
    assert!(result["help"].as_str().unwrap().contains("api_get"));
}

#[test]
fn api_post_action_parses_for_mcp_dispatch() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "api_post",
        "service": "sonarr",
        "path": "/api/v3/command",
        "body": {"name": "RefreshSeries"}
    }))
    .expect("api_post should parse");
    assert!(matches!(action, RustarrAction::ApiPost { .. }));
}

#[test]
fn api_put_action_parses_for_mcp_dispatch() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "api_put",
        "service": "sonarr",
        "path": "/api/v3/series/editor",
        "body": {"seriesIds": [1], "qualityProfileId": 4}
    }))
    .expect("api_put should parse");
    assert!(matches!(action, RustarrAction::ApiPut { .. }));
}

#[test]
fn api_delete_action_parses_for_mcp_dispatch() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "api_delete",
        "service": "sonarr",
        "path": "/api/v3/series/9?deleteFiles=false",
        "confirm": true
    }))
    .expect("api_delete should parse");
    assert!(matches!(
        action,
        RustarrAction::ApiDelete {
            body: None,
            confirm: true,
            ..
        }
    ));
}

#[tokio::test]
async fn api_delete_requires_confirm() {
    let state = loopback_state();
    let error = state
        .service
        .api_delete("sonarr", "/api/v3/series/9", None, false)
        .await
        .expect_err("api_delete without confirm should be rejected");
    assert!(error.to_string().contains("confirm=true"));
}

#[tokio::test]
async fn mcp_dispatch_rejects_missing_action() {
    let state = loopback_state();
    let error = execute_tool_without_peer_for_test(&state, "sonarr", json!({}))
        .await
        .expect_err("missing action should be rejected");
    assert!(error.to_string().contains("action is required"));
}

// ── curated command routing (C1) ─────────────────────────────────────────────

#[test]
fn curated_list_action_parses_to_curated_variant() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "list",
        "service": "sonarr"
    }))
    .expect("curated list action should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated { name: "list", .. }
    ));
}

#[test]
fn curated_action_requires_service() {
    // Curated commands all target a service; parse rejects a missing one.
    let err = RustarrAction::from_mcp_args(&json!({ "action": "list" }))
        .expect_err("list without service should error");
    assert!(err.to_string().contains("service"));
}

#[tokio::test]
async fn curated_list_routes_to_handler_for_arr_kind() {
    // The loopback stub configures sonarr (an ArrManager kind), so action=list
    // passes the action×kind guard and reaches the descriptor handler. The only
    // possible error is the transport failing against the unreachable stub URL —
    // it must NOT be the action-not-valid-for-kind validation error.
    let state = loopback_state();
    let result =
        execute_tool_without_peer_for_test(&state, "sonarr", json!({"action":"list"})).await;
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(
            !msg.contains("is not valid for kind"),
            "list on sonarr must pass the kind guard, got: {msg}"
        );
    }
}

#[test]
fn curated_list_rejected_for_non_arr_kind() {
    // The shared action×kind guard rejects list on a non-arr kind (plex) and the
    // error carries the valid-action list so the agent is taught what it can run.
    use rustarr::{ServiceKind, action_allowed_for_kind, valid_actions_for_kind};

    assert!(action_allowed_for_kind("list", ServiceKind::Sonarr));
    assert!(!action_allowed_for_kind("list", ServiceKind::Plex));
    let valid = valid_actions_for_kind(ServiceKind::Plex);
    assert!(!valid.contains(&"list"), "plex must not advertise list");
    assert!(
        valid.contains(&"service_status"),
        "plex still has infra actions"
    );
}

// ── curated write commands (C2) ──────────────────────────────────────────────

#[test]
fn set_quality_requires_write_scope() {
    use rustarr::{WRITE_SCOPE, required_scope_for_action};
    // Every C2 write command requires rustarr:write; read scope is insufficient.
    for action in [
        "set_quality",
        "search",
        "refresh",
        "monitor",
        "unmonitor",
        "add",
        "delete",
    ] {
        assert_eq!(
            required_scope_for_action(action),
            Some(WRITE_SCOPE),
            "{action} must require write scope"
        );
    }
}

#[test]
fn set_quality_action_parses_to_curated_with_to_param() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "set_quality",
        "service": "sonarr",
        "to": "HD-1080p"
    }))
    .expect("set_quality should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated {
            name: "set_quality",
            ..
        }
    ));
}

// ── curated indexer commands (C4: prowlarr only) ─────────────────────────────

#[test]
fn indexers_action_parses_to_curated_variant() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "indexers",
        "service": "prowlarr"
    }))
    .expect("curated indexers action should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated {
            name: "indexers",
            ..
        }
    ));
}

#[test]
fn indexer_commands_valid_only_for_prowlarr() {
    use rustarr::{ServiceKind, action_allowed_for_kind, valid_actions_for_kind};
    for action in [
        "indexers",
        "indexer_search",
        "indexer_stats",
        "indexer_test",
    ] {
        assert!(
            action_allowed_for_kind(action, ServiceKind::Prowlarr),
            "{action} must be valid for prowlarr"
        );
        // Rejected for an ArrManager kind, with the valid-action list to teach the agent.
        assert!(
            !action_allowed_for_kind(action, ServiceKind::Sonarr),
            "{action} must NOT be valid for sonarr"
        );
    }
    let valid = valid_actions_for_kind(ServiceKind::Sonarr);
    assert!(
        !valid.contains(&"indexers"),
        "sonarr must not advertise indexers"
    );
    let prowlarr_valid = valid_actions_for_kind(ServiceKind::Prowlarr);
    assert!(prowlarr_valid.contains(&"indexers"));
    assert!(prowlarr_valid.contains(&"indexer_search"));
}

#[test]
fn indexer_test_requires_write_scope_others_read() {
    use rustarr::{READ_SCOPE, WRITE_SCOPE, required_scope_for_action};
    assert_eq!(required_scope_for_action("indexers"), Some(READ_SCOPE));
    assert_eq!(
        required_scope_for_action("indexer_search"),
        Some(READ_SCOPE)
    );
    assert_eq!(required_scope_for_action("indexer_stats"), Some(READ_SCOPE));
    assert_eq!(required_scope_for_action("indexer_test"), Some(WRITE_SCOPE));
}

#[tokio::test]
async fn indexers_on_sonarr_rejected_with_valid_actions() {
    // The loopback stub configures sonarr (an ArrManager kind). Routing an
    // Indexer-only action at it must fail the action×kind guard with a teaching
    // error that lists what sonarr CAN run.
    let state = loopback_state();
    let err = execute_tool_without_peer_for_test(&state, "sonarr", json!({"action":"indexers"}))
        .await
        .expect_err("indexers on sonarr must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("is not valid for kind") || msg.contains("not valid"),
        "expected an action-not-valid-for-kind error, got: {msg}"
    );
    // The teaching list names sonarr's valid actions (e.g. infra + arr list).
    assert!(
        msg.contains("list"),
        "valid-action list should be present: {msg}"
    );
}

#[test]
fn write_commands_valid_only_for_arr_kinds() {
    use rustarr::{ServiceKind, action_allowed_for_kind};
    assert!(action_allowed_for_kind("set_quality", ServiceKind::Sonarr));
    assert!(action_allowed_for_kind("delete", ServiceKind::Radarr));
    assert!(!action_allowed_for_kind("set_quality", ServiceKind::Plex));
}

#[tokio::test]
async fn set_quality_without_confirm_takes_dry_run_path_not_a_confirm_error() {
    // The loopback stub configures sonarr. set_quality with confirm absent must
    // reach the dry-run path (which resolves profiles via a GET that fails against
    // the unreachable stub) — it must NEVER surface a confirm/required error, and
    // must NEVER attempt the mutating PUT (a PUT error would be a write failure,
    // but the read happens first, so a transport error here is from the GET).
    let state = loopback_state();
    let result = execute_tool_without_peer_for_test(
        &state,
        "sonarr",
        json!({"action":"set_quality","to":"HD-1080p"}),
    )
    .await;
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(
            !msg.contains("confirm"),
            "dry-run set_quality must not raise a confirm error: {msg}"
        );
        assert!(
            !msg.contains("is not valid for kind"),
            "set_quality on sonarr must pass the kind guard: {msg}"
        );
    }
}

// ── curated download commands (C5: sabnzbd + qbittorrent only) ───────────────

#[test]
fn download_queue_parses_to_curated_variant() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "download_queue",
        "service": "qbittorrent"
    }))
    .expect("curated download_queue action should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated {
            name: "download_queue",
            ..
        }
    ));
}

#[test]
fn download_commands_valid_only_for_download_kinds() {
    use rustarr::{ServiceKind, action_allowed_for_kind, valid_actions_for_kind};
    for action in [
        "download_queue",
        "download_add",
        "download_pause",
        "download_resume",
        "download_remove",
    ] {
        // Allowed for both download-client kinds ...
        assert!(
            action_allowed_for_kind(action, ServiceKind::Qbittorrent),
            "{action} must be valid for qbittorrent"
        );
        assert!(
            action_allowed_for_kind(action, ServiceKind::Sabnzbd),
            "{action} must be valid for sabnzbd"
        );
        // ... and rejected for an unrelated kind (plex / sonarr).
        assert!(
            !action_allowed_for_kind(action, ServiceKind::Plex),
            "{action} must NOT be valid for plex"
        );
        assert!(
            !action_allowed_for_kind(action, ServiceKind::Sonarr),
            "{action} must NOT be valid for sonarr"
        );
    }
    let qbit_valid = valid_actions_for_kind(ServiceKind::Qbittorrent);
    assert!(qbit_valid.contains(&"download_queue"));
    assert!(qbit_valid.contains(&"download_remove"));
    let plex_valid = valid_actions_for_kind(ServiceKind::Plex);
    assert!(!plex_valid.contains(&"download_queue"));
}

#[test]
fn download_scopes_queue_read_others_write() {
    use rustarr::{READ_SCOPE, WRITE_SCOPE, required_scope_for_action};
    assert_eq!(
        required_scope_for_action("download_queue"),
        Some(READ_SCOPE)
    );
    for action in [
        "download_add",
        "download_pause",
        "download_resume",
        "download_remove",
    ] {
        assert_eq!(
            required_scope_for_action(action),
            Some(WRITE_SCOPE),
            "{action} must be write scope"
        );
    }
}

#[tokio::test]
async fn download_queue_on_sonarr_rejected_with_valid_actions() {
    // The loopback stub configures sonarr (an ArrManager kind). Routing a
    // DownloadClient-only action at it must fail the action×kind guard with a
    // teaching error that lists what sonarr CAN run.
    let state = loopback_state();
    let err =
        execute_tool_without_peer_for_test(&state, "sonarr", json!({"action":"download_queue"}))
            .await
            .expect_err("download_queue on sonarr must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("is not valid for kind") || msg.contains("not valid"),
        "expected an action-not-valid-for-kind error, got: {msg}"
    );
    assert!(
        msg.contains("list"),
        "valid-action list should be present: {msg}"
    );
}

// ── curated media-server commands (C6: plex + jellyfin only) ─────────────────

#[test]
fn media_sessions_parses_to_curated_and_routes_to_media_kinds() {
    // media_sessions on plex parses to the curated variant and is kind-valid;
    // it is rejected for non-media kinds. (Scopes are covered in the colocated
    // actions/commands/media_server tests.)
    use rustarr::{ServiceKind, action_allowed_for_kind};
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "media_sessions", "service": "plex"
    }))
    .expect("curated media_sessions action should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated {
            name: "media_sessions",
            ..
        }
    ));
    for a in [
        "media_sessions",
        "media_libraries",
        "media_search",
        "media_scan",
    ] {
        let media = action_allowed_for_kind(a, ServiceKind::Plex)
            && action_allowed_for_kind(a, ServiceKind::Jellyfin);
        let other = action_allowed_for_kind(a, ServiceKind::Sonarr)
            || action_allowed_for_kind(a, ServiceKind::Qbittorrent);
        assert!(media, "{a} must be valid for plex+jellyfin");
        assert!(!other, "{a} must be rejected for non-media kinds");
    }
}

#[tokio::test]
async fn media_sessions_on_sonarr_rejected_with_valid_actions() {
    // The loopback stub configures sonarr (an ArrManager kind). Routing a
    // MediaServer-only action at it must fail the action×kind guard with a
    // teaching error that lists what sonarr CAN run.
    let state = loopback_state();
    let err =
        execute_tool_without_peer_for_test(&state, "sonarr", json!({"action":"media_sessions"}))
            .await
            .expect_err("media_sessions on sonarr must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("is not valid for kind") || msg.contains("not valid"),
        "expected an action-not-valid-for-kind error, got: {msg}"
    );
    assert!(
        msg.contains("list"),
        "valid-action list should be present: {msg}"
    );
}

#[tokio::test]
async fn codemode_dispatches_js_that_calls_actions() {
    // action=codemode runs a JS script that calls integrations via callTool and
    // returns a computed value; the response carries result + calls + logs.
    let state = loopback_state();
    let code = r#"async () => {
        console.log("starting");
        const info = await callTool("integrations", {});
        return { kinds: info.supported.length };
    }"#;
    let out = execute_tool_without_peer_for_test(
        &state,
        "sonarr",
        json!({ "action": "codemode", "code": code }),
    )
    .await
    .expect("codemode dispatch should succeed");

    assert_eq!(out["result"]["kinds"], 11);
    assert_eq!(out["calls"][0]["action"], "integrations");
    assert_eq!(out["logs"][0], "starting");
}

#[tokio::test]
async fn codemode_requires_code_param() {
    let state = loopback_state();
    let err = execute_tool_without_peer_for_test(&state, "sonarr", json!({ "action": "codemode" }))
        .await
        .expect_err("codemode without code must error");
    assert!(err.to_string().contains("code"), "got: {err}");
}

#[tokio::test]
async fn snippet_list_is_a_routed_action() {
    // The loopback stub has no data dir, so snippet_list routes to the handler and
    // errors with a data-dir message (proving it's recognized, not "unknown action").
    let state = loopback_state();
    let err =
        execute_tool_without_peer_for_test(&state, "sonarr", json!({ "action": "snippet_list" }))
            .await
            .expect_err("snippet_list without a data dir should error");
    assert!(err.to_string().contains("data dir"), "got: {err}");
}
