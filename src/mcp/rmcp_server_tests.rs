use serde_json::json;

use crate::{
    actions::{READ_SCOPE, WRITE_SCOPE, required_scope_for_action},
    app::RustarrService,
    config::{RustarrConfig, ServiceConfig, ServiceKind},
    rustarr::RustarrClient,
    token_limit::MAX_RESPONSE_BYTES,
};

use super::{
    declined_result, inject_confirm, internal_tool_error_message,
    reject_unknown_action_before_scope, rmcp_tool_definitions_for_service, scope_satisfied,
    tool_result_from_json,
};

fn scopes(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| x.to_string()).collect()
}

#[test]
fn read_scope_satisfies_read_requirement() {
    assert!(scope_satisfied(&scopes(&[READ_SCOPE]), READ_SCOPE));
}

#[test]
fn write_scope_satisfies_read_requirement() {
    assert!(
        scope_satisfied(&scopes(&[WRITE_SCOPE]), READ_SCOPE),
        "write scope should satisfy read requirement (write includes read)"
    );
}

#[test]
fn empty_scopes_denied() {
    assert!(!scope_satisfied(&[], READ_SCOPE));
}

#[test]
fn unrelated_scope_denied() {
    assert!(!scope_satisfied(&scopes(&["other:scope"]), READ_SCOPE));
}

#[test]
fn read_scope_does_not_satisfy_write() {
    assert!(
        !scope_satisfied(&scopes(&[READ_SCOPE]), WRITE_SCOPE),
        "read scope must not satisfy write requirement"
    );
}

#[test]
fn api_get_requires_write_scope() {
    assert_eq!(required_scope_for_action("api_get"), Some(WRITE_SCOPE));
}

#[test]
fn api_post_requires_write_scope() {
    assert_eq!(required_scope_for_action("api_post"), Some(WRITE_SCOPE));
}

#[test]
fn help_requires_no_scope() {
    assert_eq!(required_scope_for_action("help"), None);
}

#[test]
fn unknown_action_gets_deny_scope() {
    use crate::actions::DENY_SCOPE;
    assert_eq!(
        required_scope_for_action("nonexistent_action"),
        Some(DENY_SCOPE)
    );
}

#[test]
fn unknown_action_is_rejected_as_validation_before_scope() {
    let error = reject_unknown_action_before_scope("nonexistent_action")
        .expect_err("unknown action should be invalid params");
    assert!(error.message.contains("unknown rustarr action"));
}

#[test]
fn internal_tool_errors_include_stable_kind() {
    let message = internal_tool_error_message("status");
    assert!(message.contains("kind=execution_error"));
    assert!(message.contains("action='status'"));
}

#[test]
fn tool_result_from_json_applies_response_cap() {
    let result = tool_result_from_json(json!({
        "payload": "x".repeat(MAX_RESPONSE_BYTES + 1)
    }))
    .expect("tool result should serialize");
    let text = result.content[0]
        .raw
        .as_text()
        .expect("tool result should contain text")
        .text
        .as_str();
    // The over-cap result is now a parseable JSON envelope with a truncation
    // marker (AN-6), not a notice appended after the JSON.
    let parsed: serde_json::Value =
        serde_json::from_str(text).expect("truncated tool result is valid JSON");
    assert_eq!(parsed["truncated"], true);
    assert!(text.len() <= MAX_RESPONSE_BYTES);
}

#[test]
fn inject_confirm_sets_confirm_true_on_object_args() {
    // On a confirmed elicitation the gate injects confirm=true so the shared
    // dispatch + app-layer gate see the same confirmation the CLI's --confirm
    // produces. Existing args are preserved.
    let mut args = json!({ "action": "delete", "id": 5 });
    inject_confirm(&mut args);
    assert_eq!(args["confirm"], json!(true));
    assert_eq!(args["id"], json!(5));
}

#[test]
fn inject_confirm_is_noop_on_non_object() {
    // A non-object never gets confirm injected, so it can't silently authorize a
    // delete — the app layer still blocks it.
    let mut args = json!("not an object");
    inject_confirm(&mut args);
    assert_eq!(args, json!("not an object"));
}

#[test]
fn declined_result_reports_declined_and_nothing_changed() {
    let result = declined_result("delete").expect("declined result builds");
    let text = result.content[0]
        .raw
        .as_text()
        .expect("declined result should contain text")
        .text
        .as_str();
    let parsed: serde_json::Value =
        serde_json::from_str(text).expect("declined result is valid JSON");
    assert_eq!(parsed["declined"], json!(true));
    assert_eq!(parsed["action"], json!("delete"));
    assert!(
        parsed["note"]
            .as_str()
            .is_some_and(|n| n.contains("nothing was changed")),
        "declined result must state nothing changed: {parsed}"
    );
}

#[test]
fn mcp_advertises_exactly_one_yarr_tool() {
    // ONE tool regardless of how many services are configured — the whole fleet is
    // reached inside a `yarr` script, so the agent never carries N tool schemas.
    let config = RustarrConfig {
        services: vec![
            ServiceConfig {
                name: "sonarr".into(),
                kind: ServiceKind::Sonarr,
                base_url: "http://localhost:8989".into(),
                api_key: Some("test".into()),
                ..ServiceConfig::default()
            },
            ServiceConfig {
                name: "plex".into(),
                kind: ServiceKind::Plex,
                base_url: "http://localhost:32400".into(),
                token: Some("test".into()),
                ..ServiceConfig::default()
            },
        ],
    };
    let client = RustarrClient::new(&config).expect("client builds");
    let service = RustarrService::new(client, config);

    let tools = rmcp_tool_definitions_for_service(&service).expect("tool definitions");
    assert_eq!(tools.len(), 1, "exactly one MCP tool");
    assert_eq!(tools[0].name.as_ref(), "yarr");
    // Its only input is `code`.
    let schema = &tools[0].input_schema;
    let required = schema
        .get("required")
        .and_then(|r| r.as_array())
        .expect("required list");
    assert_eq!(required, &[serde_json::json!("code")]);
    assert!(
        schema
            .get("properties")
            .and_then(|p| p.get("code"))
            .is_some()
    );
}
