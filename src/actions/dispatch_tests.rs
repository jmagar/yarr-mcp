use super::*;
use crate::actions::RustarrAction;
use crate::testing::loopback_state;

#[tokio::test]
async fn help_action_dispatches_to_rest_help() {
    let state = loopback_state();
    let result = execute_service_action(&state.service, &RustarrAction::Help)
        .await
        .unwrap();
    assert!(result.get("actions").is_some());
    assert!(result.get("examples").is_some());
}

#[tokio::test]
async fn integrations_action_dispatches() {
    let state = loopback_state();
    let result = execute_service_action(&state.service, &RustarrAction::Integrations)
        .await
        .unwrap();
    assert!(result.get("supported").is_some());
    assert!(result.get("configured").is_some());
}

#[test]
fn shared_guard_allows_infra_actions_for_configured_kind() {
    // loopback_state configures a sonarr (ArrManager) service. Every infra action
    // is allowed for it via the shared guard.
    let state = loopback_state();
    for action in ["service_status", "api_get", "api_post"] {
        validate_action_for_service(&state.service, action, "sonarr")
            .unwrap_or_else(|e| panic!("{action} should be allowed for sonarr: {e}"));
    }
}

#[test]
fn shared_guard_rejects_action_invalid_for_kind_with_valid_actions() {
    // A non-infra, non-curated (unknown) action fails closed and the error carries
    // the valid-action list so its Display teaches the agent (AN-2). `set_quality`
    // is now a real arr command (valid for sonarr), so use an unknown name here.
    let state = loopback_state();
    let err = validate_action_for_service(&state.service, "totally_unknown", "sonarr")
        .expect_err("totally_unknown is not valid for sonarr");
    let msg = err.to_string();
    assert!(msg.contains("not valid for kind=sonarr"), "msg: {msg}");
    assert!(msg.contains("valid actions for sonarr"), "msg: {msg}");
    // The valid-action list includes the infra actions for that kind.
    assert!(msg.contains("service_status"), "msg: {msg}");
    assert!(crate::actions::is_validation_error(&err));
}

#[test]
fn shared_guard_allows_curated_write_command_for_arr_kind() {
    // C2: a curated arr write command (set_quality) passes the kind guard for an
    // ArrManager kind (sonarr) — it is no longer "invalid for kind".
    let state = loopback_state();
    validate_action_for_service(&state.service, "set_quality", "sonarr")
        .expect("set_quality should be valid for sonarr");
}

#[test]
fn shared_guard_skips_unknown_service_name() {
    // An unconfigured service name resolves to no kind; the guard defers to the
    // downstream service lookup rather than producing a kind error.
    let state = loopback_state();
    validate_action_for_service(&state.service, "set_quality", "not-configured")
        .expect("guard is a no-op for unknown service names");
}
