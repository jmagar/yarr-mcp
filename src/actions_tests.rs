use serde_json::json;

use super::*;

#[test]
fn action_metadata_is_the_action_source_of_truth() {
    assert_eq!(
        action_names(),
        vec![
            "greet",
            "echo",
            "status",
            "elicit_name",
            "scaffold_intent",
            "help"
        ]
    );
    assert_eq!(rest_action_names(), vec!["greet", "echo", "status", "help"]);
    assert_eq!(
        mcp_only_action_names(),
        vec!["elicit_name", "scaffold_intent"]
    );
    assert!(is_rest_action("greet"));
    assert!(!is_rest_action("scaffold_intent"));
    assert_eq!(required_scope_for_action("help"), None);
    assert_eq!(required_scope_for_action("greet"), Some(READ_SCOPE));
    assert_eq!(required_scope_for_action("unknown"), Some(DENY_SCOPE));
}

#[test]
fn mcp_args_parse_flat_shape() {
    let action = ExampleAction::from_mcp_args(&json!({ "action": "echo", "message": "hello" }))
        .expect("flat MCP args should parse");
    assert_eq!(
        action,
        ExampleAction::Echo {
            message: "hello".into()
        }
    );
}

#[test]
fn rest_args_parse_nested_params_shape() {
    let action = ExampleAction::from_rest("greet", &json!({ "name": "Alice" }))
        .expect("REST params should parse");
    assert_eq!(
        action,
        ExampleAction::Greet {
            name: Some("Alice".into())
        }
    );
}

#[test]
fn missing_action_is_validation_error() {
    let error = ExampleAction::from_mcp_args(&json!({})).unwrap_err();
    assert!(error.to_string().contains("action is required"));
}

#[test]
fn echo_rejects_missing_and_empty_message() {
    let missing = ExampleAction::from_mcp_args(&json!({ "action": "echo" })).unwrap_err();
    assert!(missing.to_string().contains("`message` is required"));

    let empty = ExampleAction::from_rest("echo", &json!({ "message": "" })).unwrap_err();
    assert!(empty.to_string().contains("`message` is required"));
}

#[test]
fn string_params_reject_wrong_json_type() {
    let greet = ExampleAction::from_rest("greet", &json!({ "name": 42 })).unwrap_err();
    assert!(greet.to_string().contains("`name` must be a string"));

    let echo = ExampleAction::from_mcp_args(&json!({
        "action": "echo",
        "message": ["not", "a", "string"]
    }))
    .unwrap_err();
    assert!(echo.to_string().contains("`message` must be a string"));
}

#[test]
fn scaffold_intent_parses_as_mcp_only_action() {
    let action = ExampleAction::from_mcp_args(&json!({ "action": "scaffold_intent" }))
        .expect("scaffold_intent should parse");
    assert_eq!(action, ExampleAction::ScaffoldIntent);
}

#[test]
fn rest_rejects_mcp_only_actions() {
    let error = ExampleAction::from_rest("scaffold_intent", &json!({})).unwrap_err();
    assert!(error.to_string().contains("not available over REST"));

    let error = ExampleAction::from_rest("elicit_name", &json!({})).unwrap_err();
    assert!(error.to_string().contains("not available over REST"));
}

#[test]
fn rest_missing_action_preserves_missing_action_error() {
    let error = ExampleAction::from_rest("", &json!({})).unwrap_err();
    assert_eq!(error.to_string(), "action is required");
}

#[test]
fn unknown_action_mentions_help() {
    let error = ExampleAction::from_rest("missing", &json!({})).unwrap_err();
    assert!(error.to_string().contains("action=help"));
}

#[test]
fn all_parse_errors_are_classified_as_validation_errors() {
    let cases: &[anyhow::Error] = &[
        ExampleAction::from_mcp_args(&json!({})).unwrap_err(),
        ExampleAction::from_mcp_args(&json!({ "action": "echo" })).unwrap_err(),
        ExampleAction::from_rest("echo", &json!({ "message": "" })).unwrap_err(),
        ExampleAction::from_rest("greet", &json!({ "name": 42 })).unwrap_err(),
        ExampleAction::from_rest("scaffold_intent", &json!({})).unwrap_err(),
        ExampleAction::from_rest("missing", &json!({})).unwrap_err(),
    ];
    for (i, err) in cases.iter().enumerate() {
        assert!(
            is_validation_error(err),
            "case {i}: expected validation error, got: {err}"
        );
    }
}

#[test]
fn non_validation_errors_are_not_classified_as_validation_errors() {
    let err = anyhow::anyhow!("something unexpected went wrong");
    assert!(
        !is_validation_error(&err),
        "plain anyhow errors must not be classified as validation errors"
    );
}

#[test]
fn scopes_satisfy_write_implies_read() {
    let write = vec![WRITE_SCOPE.to_string()];
    assert!(scopes_satisfy(&write, READ_SCOPE));
    assert!(scopes_satisfy(&write, WRITE_SCOPE));
}

#[test]
fn scopes_satisfy_read_does_not_imply_write() {
    let read = vec![READ_SCOPE.to_string()];
    assert!(!scopes_satisfy(&read, WRITE_SCOPE));
}
