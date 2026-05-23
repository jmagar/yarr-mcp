use crate::actions::action_names;

use super::tool_definitions;

#[test]
fn schema_action_enum_comes_from_action_metadata() {
    let tools = tool_definitions();
    let enum_values = tools[0]["inputSchema"]["properties"]["action"]["enum"]
        .as_array()
        .expect("action enum should be an array")
        .iter()
        .map(|value| value.as_str().expect("action enum values are strings"))
        .collect::<Vec<_>>();

    assert_eq!(enum_values, action_names());
}

#[test]
fn echo_message_schema_requires_non_empty_string() {
    let tools = tool_definitions();
    assert_eq!(
        tools[0]["inputSchema"]["properties"]["message"]["minLength"],
        1
    );
}

#[test]
fn schema_conditionally_requires_echo_message() {
    let tools = tool_definitions();
    let all_of = tools[0]["inputSchema"]["allOf"]
        .as_array()
        .expect("schema should include conditional action validation");
    assert!(
        all_of.iter().any(
            |entry| entry["if"]["properties"]["action"]["const"] == "echo"
                && entry["then"]["required"]
                    .as_array()
                    .is_some_and(|required| required.iter().any(|field| field == "message"))
        ),
        "echo action must conditionally require message"
    );
}

#[test]
fn schema_disallows_unknown_top_level_properties() {
    let tools = tool_definitions();
    assert_eq!(tools[0]["inputSchema"]["additionalProperties"], false);
}
