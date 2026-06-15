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
