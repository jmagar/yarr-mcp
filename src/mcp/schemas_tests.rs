use crate::actions::all_action_names;

use super::tool_definitions;

#[test]
fn service_named_tools_are_advertised() {
    let tools = tool_definitions();
    let names = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "sonarr",
            "radarr",
            "prowlarr",
            "overseerr",
            "tautulli",
            "plex",
            "tracearr",
            "sabnzbd",
            "qbittorrent",
            "jellyfin",
            "bazarr",
        ]
    );
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
    assert!(all_action_names().contains(&"list"));
    assert!(enum_values.contains(&"list"));
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
                        !required.iter().any(|field| field == "service")
                            && required.iter().any(|field| field == "path")
                    })
        }),
        "api_get action must conditionally require path; service is implied by tool name"
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

#[test]
fn generic_only_tool_filters_curated_actions() {
    let tools = tool_definitions();
    let bazarr = tools
        .iter()
        .find(|tool| tool["name"] == "bazarr")
        .expect("bazarr tool should be advertised");
    let enum_values = bazarr["inputSchema"]["properties"]["action"]["enum"]
        .as_array()
        .expect("action enum should be an array");
    assert!(
        !enum_values.iter().any(|value| value == "list"),
        "bazarr should not advertise arr curated commands"
    );
}
