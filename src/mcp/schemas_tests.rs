use crate::actions::all_action_names;

use super::tool_definitions;

#[test]
fn exactly_one_tool_named_rustarr() {
    let tools = tool_definitions();
    assert_eq!(tools.len(), 1, "exactly one MCP tool must be advertised");
    assert_eq!(tools[0]["name"], "rustarr");
}

#[test]
fn schema_action_enum_comes_from_action_metadata() {
    let tools = tool_definitions();
    let enum_values = tools[0]["inputSchema"]["properties"]["action"]["enum"]
        .as_array()
        .expect("action enum should be an array")
        .iter()
        .map(|value| value.as_str().expect("action enum values are strings"))
        .collect::<Vec<_>>();

    // The enum is the union of generic action specs and curated command names.
    assert_eq!(enum_values, all_action_names());
}

#[test]
fn path_schema_requires_non_empty_string() {
    let tools = tool_definitions();
    assert_eq!(
        tools[0]["inputSchema"]["properties"]["path"]["minLength"],
        1
    );
}

#[test]
fn schema_conditionally_requires_api_get_fields() {
    let tools = tool_definitions();
    let all_of = tools[0]["inputSchema"]["allOf"]
        .as_array()
        .expect("schema should include conditional action validation");
    assert!(
        all_of.iter().any(|entry| {
            entry["if"]["properties"]["action"]["const"] == "api_get"
                && entry["then"]["required"]
                    .as_array()
                    .is_some_and(|required| {
                        required.iter().any(|field| field == "service")
                            && required.iter().any(|field| field == "path")
                    })
        }),
        "api_get action must conditionally require service and path"
    );
}

#[test]
fn schema_disallows_unknown_top_level_properties() {
    let tools = tool_definitions();
    assert_eq!(tools[0]["inputSchema"]["additionalProperties"], false);
}

#[test]
fn schema_exposes_verbosity_opt_ins() {
    let tools = tool_definitions();
    let props = &tools[0]["inputSchema"]["properties"];
    assert_eq!(props["verbose"]["type"], "boolean");
    assert_eq!(props["fields"]["type"], "array");
}
