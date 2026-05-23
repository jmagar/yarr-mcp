use crate::token_limit::MAX_RESPONSE_BYTES;
use serde_json::json;

use super::{cap_rest_response, ActionRequest};

#[test]
fn action_request_defaults_to_empty_action_and_null_params() {
    let req: ActionRequest = serde_json::from_str("{}").unwrap();
    assert_eq!(req.action, "");
    assert!(req.params.is_null());
}

#[test]
fn action_request_parses_action_and_params() {
    let req: ActionRequest = serde_json::from_value(json!({
        "action": "api_get",
        "params": {"service": "sonarr", "path": "/api/v3/system/status"}
    }))
    .unwrap();
    assert_eq!(req.action, "api_get");
    assert_eq!(req.params["service"], "sonarr");
}

#[test]
fn action_request_params_defaults_to_null_when_omitted() {
    let req: ActionRequest = serde_json::from_value(json!({ "action": "help" })).unwrap();
    assert_eq!(req.action, "help");
    assert!(req.params.is_null());
}

#[test]
fn cap_rest_response_leaves_small_json_unchanged() {
    let value = json!({"status": "ok"});
    assert_eq!(cap_rest_response(value.clone()).unwrap(), value);
}

#[test]
fn cap_rest_response_returns_json_safe_truncation_envelope() {
    let value = json!({"payload": "x".repeat(MAX_RESPONSE_BYTES + 1)});
    let capped = cap_rest_response(value).unwrap();

    assert_eq!(capped["truncated"], true);
    assert_eq!(
        capped["error"],
        "response exceeded REST response size limit"
    );
    assert_eq!(capped["max_response_bytes"], MAX_RESPONSE_BYTES);
    assert!(capped["hint"]
        .as_str()
        .unwrap_or_default()
        .contains("limit"));
    assert!(
        serde_json::to_vec(&capped).unwrap().len() < MAX_RESPONSE_BYTES,
        "{capped}"
    );
}
