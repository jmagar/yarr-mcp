use rustarr::{RustarrAction, execute_tool_without_peer_for_test, testing::loopback_state};
use serde_json::json;

async fn call_mcp_action(args: serde_json::Value) -> serde_json::Value {
    let state = loopback_state();
    execute_tool_without_peer_for_test(&state, "sonarr", args)
        .await
        .expect("MCP tool dispatch should succeed")
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

// ── curated command routing (doc-based capabilities) ─────────────────────────

#[test]
fn curated_action_requires_service() {
    // Curated commands all target a service; parse rejects a missing one.
    let err = RustarrAction::from_mcp_args(&json!({ "action": "download_queue" }))
        .expect_err("download_queue without service should error");
    assert!(err.to_string().contains("service"));
}

// ── generated OpenAPI operations (spec-backed kinds) ─────────────────────────

#[test]
fn generated_op_action_parses_to_op_variant() {
    // The 6 spec-backed kinds expose generated operations via the `op` action,
    // carrying service + op + args.
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "op",
        "service": "sonarr",
        "op": "get_series",
        "args": {}
    }))
    .expect("op action should parse");
    assert!(matches!(action, RustarrAction::Op { .. }));
}

#[tokio::test]
async fn generated_op_routes_through_executor_for_spec_backed_kind() {
    // The loopback stub configures sonarr (spec-backed). A generated op passes the
    // guard and reaches the executor; the only error is transport against the
    // unreachable stub — never an unknown-action or not-valid-for-kind error.
    let state = loopback_state();
    let result = execute_tool_without_peer_for_test(
        &state,
        "sonarr",
        json!({"action":"op","op":"get_system_status","args":{}}),
    )
    .await;
    if let Err(err) = result {
        let msg = err.to_string();
        assert!(!msg.contains("unknown"), "op must resolve, got: {msg}");
        assert!(
            !msg.contains("is not valid for kind"),
            "op must pass the kind guard, got: {msg}"
        );
    }
}

// ── curated download commands (sabnzbd + qbittorrent only) ───────────────────

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

#[tokio::test]
async fn codemode_dispatches_js_that_calls_actions() {
    // action=codemode runs a JS script that calls a local action via callTool and
    // returns a computed value; the response carries result + calls + logs.
    let state = loopback_state();
    let code = r#"async () => {
        console.log("starting");
        const h = await callTool("help", {});
        return { hasHelp: typeof h.help === "string" };
    }"#;
    let out = execute_tool_without_peer_for_test(
        &state,
        "sonarr",
        json!({ "action": "codemode", "code": code }),
    )
    .await
    .expect("codemode dispatch should succeed");

    assert_eq!(out["result"]["hasHelp"], true);
    assert_eq!(out["calls"][0]["action"], "help");
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
