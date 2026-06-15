use rustarr::{
    actions::RustarrAction, mcp::execute_tool_without_peer_for_test, testing::loopback_state,
};
use serde_json::json;

async fn call_mcp_action(args: serde_json::Value) -> serde_json::Value {
    let state = loopback_state();
    execute_tool_without_peer_for_test(&state, "rustarr", args)
        .await
        .expect("MCP tool dispatch should succeed")
}

#[tokio::test]
async fn integrations_returns_supported_services() {
    let result = call_mcp_action(json!({ "action": "integrations" })).await;
    // `supported` is now a list of {kind, capability} objects (registry-derived).
    assert!(result["supported"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["kind"] == "sonarr"));
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
        "body": {"seriesIds": [1], "qualityProfileId": 4},
        "confirm": true
    }))
    .expect("api_put should parse");
    assert!(matches!(
        action,
        RustarrAction::ApiPut { confirm: true, .. }
    ));
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
async fn api_put_requires_confirm() {
    let state = loopback_state();
    let error = state
        .service
        .api_put("sonarr", "/api/v3/series/editor", json!({}), false)
        .await
        .expect_err("api_put without confirm should be rejected");
    assert!(error.to_string().contains("confirm=true"));
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
    let error = execute_tool_without_peer_for_test(&state, "rustarr", json!({}))
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
    let result = execute_tool_without_peer_for_test(
        &state,
        "rustarr",
        json!({"action":"list","service":"sonarr"}),
    )
    .await;
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
    use rustarr::actions::{action_allowed_for_kind, valid_actions_for_kind};
    use rustarr::config::ServiceKind;

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
    use rustarr::actions::{required_scope_for_action, WRITE_SCOPE};
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
    use rustarr::actions::{action_allowed_for_kind, valid_actions_for_kind};
    use rustarr::config::ServiceKind;
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
    use rustarr::actions::{required_scope_for_action, READ_SCOPE, WRITE_SCOPE};
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
    let err = execute_tool_without_peer_for_test(
        &state,
        "rustarr",
        json!({"action":"indexers","service":"sonarr"}),
    )
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
    use rustarr::actions::action_allowed_for_kind;
    use rustarr::config::ServiceKind;
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
        "rustarr",
        json!({"action":"set_quality","service":"sonarr","to":"HD-1080p"}),
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
