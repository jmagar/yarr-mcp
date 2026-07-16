use super::*;
#[test]
fn mcp_advertises_exactly_one_yarr_tool() {
    // ONE tool regardless of how many services are configured — the whole fleet is
    // reached inside a `yarr` script, so the agent never carries N tool schemas.
    let config = YarrConfig {
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
    let client = YarrClient::new(&config).expect("client builds");
    let service = YarrService::new(client, config);
    let state = AppState {
        config: McpConfig::default(),
        auth_policy: AuthPolicy::LoopbackDev,
        service,
    };

    let tools = rmcp_tool_definitions_for_service(&state).expect("tool definitions");
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

#[test]
fn flat_mode_advertises_one_tool_per_configured_service_only() {
    // `flat` mode replaces the single `yarr` tool with one action-dispatched
    // tool per *configured* service kind — not all eleven kinds rustarr can
    // wrap. Sonarr + Plex are configured here; Radarr isn't, so it must not
    // appear even though `radarr` is a valid ServiceKind.
    let config = YarrConfig {
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
    let client = YarrClient::new(&config).expect("client builds");
    let service = YarrService::new(client, config);
    let state = AppState {
        config: McpConfig {
            tool_mode: ToolMode::Flat,
            ..McpConfig::default()
        },
        auth_policy: AuthPolicy::LoopbackDev,
        service,
    };

    let tools = rmcp_tool_definitions_for_service(&state).expect("tool definitions");
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    assert_eq!(
        tools.len(),
        2,
        "one tool per configured service, not eleven"
    );
    assert!(names.contains(&"sonarr"));
    assert!(names.contains(&"plex"));
    assert!(!names.contains(&"radarr"), "radarr isn't configured");
    assert!(
        !names.contains(&"yarr"),
        "flat mode has no yarr tool at all"
    );

    // Each is action-dispatched: `action` is a required schema property, with
    // an enum of many valid actions — not the single-purpose `yarr` tool whose
    // only required input is an opaque `code` string. (`codemode` legitimately
    // remains one of the many valid `action` values here — flat mode adds
    // direct per-action calls, it doesn't remove the option to still hand the
    // tool a script for one call.)
    for tool in &tools {
        let schema = &tool.input_schema;
        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .unwrap_or_else(|| panic!("{} tool missing required list", tool.name));
        assert_eq!(
            required,
            &[serde_json::json!("action")],
            "{} tool's only required input should be `action`",
            tool.name
        );
        let action_enum = schema
            .get("properties")
            .and_then(|p| p.get("action"))
            .and_then(|a| a.get("enum"))
            .and_then(|e| e.as_array())
            .unwrap_or_else(|| panic!("{} tool missing action enum", tool.name));
        assert!(
            action_enum.len() > 1,
            "{} tool's action enum should offer more than one action",
            tool.name
        );
    }
}

#[test]
fn flat_mode_preserves_configured_names_for_duplicate_kinds() {
    let config = YarrConfig {
        services: vec![
            ServiceConfig {
                name: "sonarr-east".into(),
                kind: ServiceKind::Sonarr,
                base_url: "http://localhost:8989".into(),
                ..ServiceConfig::default()
            },
            ServiceConfig {
                name: "sonarr-west".into(),
                kind: ServiceKind::Sonarr,
                base_url: "http://localhost:8990".into(),
                ..ServiceConfig::default()
            },
        ],
    };
    let client = YarrClient::new(&config).expect("client builds");
    let state = AppState {
        config: McpConfig {
            tool_mode: ToolMode::Flat,
            ..McpConfig::default()
        },
        auth_policy: AuthPolicy::LoopbackDev,
        service: YarrService::new(client, config),
    };

    let tools = rmcp_tool_definitions_for_service(&state).expect("tool definitions");
    let names = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["sonarr-east", "sonarr-west"]);
}
