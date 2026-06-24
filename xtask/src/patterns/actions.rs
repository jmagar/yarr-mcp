use super::{reporter::PatternReporter, util::read_file};

const ACTION_TEST_COVERAGE_EXCEPTIONS: &[&str] = &[];

pub(super) fn action_surfaces(reporter: &mut PatternReporter) {
    // ACTION_SPECS holds the generic/infra actions. After the descriptor-table
    // refactor the registry lives in `src/actions/registry.rs` (the old
    // monolithic `src/actions.rs` is now a thin facade). Curated commands are
    // validated separately by `tests/parity.rs`.
    let actions_text = read_file("src/actions/registry.rs");
    let action_specs = action_specs_body(&actions_text).unwrap_or(&actions_text);
    let action_names = extract_action_names(action_specs);
    let mcp_only = extract_mcp_only_actions(action_specs);

    if action_names.is_empty() {
        reporter.fail(
            "actions",
            "could not parse ACTION_SPECS from src/actions/registry.rs",
        );
        return;
    }

    let schema = read_file("src/mcp/schemas.rs");
    // Help is generated from the registry in `src/actions/help.rs` (service-layer,
    // shared by MCP); `tools.rs` is a thin dispatch shim with no special cases.
    // CLI parsing moved from `cli.rs` into the `cli/router.rs` + `cli/commands/`
    // tree. Read the current homes.
    let tools = read_file("src/actions/help.rs");
    let tests = read_file("tests/tool_dispatch.rs");
    let cli = format!(
        "{}\n{}",
        read_file("src/cli.rs"),
        read_file("src/cli/router.rs")
    );

    let schema_uses_metadata = schema.contains("action_names()");
    let missing_schema = if schema_uses_metadata {
        Vec::new()
    } else {
        action_names
            .iter()
            .filter(|action| !schema.contains(&format!("\"{action}\"")))
            .cloned()
            .collect::<Vec<_>>()
    };
    // help.rs generates the help text from the registry; the generic actions
    // appear by name in its `generic_description` match, so plain containment
    // is the right signal (the old `### <action>` markup no longer applies).
    // MCP-only actions (codemode/op/snippet_*) are not part of the REST help
    // surface, so they are excluded here just as they are for the CLI check.
    let missing_help = action_names
        .iter()
        .filter(|action| !mcp_only.contains(action))
        .filter(|action| {
            !tools.contains(&format!("\"{action}\"")) && !tools.contains(&format!("`{action}`"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let missing_tests = action_names
        .iter()
        .filter(|action| {
            action.as_str() != "help"
                && !ACTION_TEST_COVERAGE_EXCEPTIONS.contains(&action.as_str())
                && !tests.contains(action.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let missing_cli = action_names
        .iter()
        .filter(|action| action.as_str() != "help" && !mcp_only.contains(action))
        .filter(|action| {
            !cli_tokens_for_action(action)
                .iter()
                .any(|token| cli.contains(token))
        })
        .cloned()
        .collect::<Vec<_>>();

    if !missing_schema.is_empty() {
        reporter.fail(
            "actions",
            format!(
                "schemas.rs missing action(s): {}",
                missing_schema.join(", ")
            ),
        );
    }
    if !missing_help.is_empty() {
        reporter.fail(
            "actions",
            format!(
                "actions/help.rs generated help missing action(s): {}. Hint: add a generic_description arm or registry entry.",
                missing_help.join(", ")
            ),
        );
    }
    if !missing_tests.is_empty() {
        reporter.warn(
            "actions",
            format!(
                "tests/tool_dispatch.rs may be missing action coverage: {}. Hint: add a direct dispatch/service test or an explicit exception.",
                missing_tests.join(", ")
            ),
        );
    }
    if !missing_cli.is_empty() {
        reporter.warn(
            "cli-mcp-parity",
            format!(
                "CLI may be missing non-MCP-only action(s): {}. Hint: add a Command variant, parse arm, and dispatch arm.",
                missing_cli.join(", ")
            ),
        );
    }
    if missing_schema.is_empty()
        && missing_help.is_empty()
        && missing_tests.is_empty()
        && missing_cli.is_empty()
    {
        reporter.ok(
            "actions",
            format!(
                "{} actions appear in schema/help/tests/CLI surfaces",
                action_names.len()
            ),
        );
    }
}

fn action_specs_body(text: &str) -> Option<&str> {
    let start = text.find("ACTION_SPECS")?;
    let after_start = &text[start..];
    let end = after_start.find("];")?;
    Some(&after_start[..end])
}

fn extract_action_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let (_, after_name) = line.split_once("name:")?;
            let start = after_name.find('"')? + 1;
            let rest = &after_name[start..];
            let end = rest.find('"')?;
            Some(rest[..end].to_string())
        })
        .collect()
}

fn extract_mcp_only_actions(text: &str) -> Vec<String> {
    let mut actions = Vec::new();
    for block in text.split("ActionSpec").skip(1) {
        let Some(end) = block.find('}') else {
            continue;
        };
        let block = &block[..end];
        if !block.contains("ActionTransport::McpOnly") {
            continue;
        }
        if let Some(name) = extract_action_names(block).into_iter().next() {
            actions.push(name);
        }
    }
    actions
}

fn variant_name(action: &str) -> String {
    action
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<String>()
}

fn cli_tokens_for_action(action: &str) -> Vec<String> {
    let cli_name = match action {
        "service_status" => "status",
        "api_get" => "get",
        "api_post" => "post",
        "api_put" => "put",
        "api_delete" => "delete",
        other => other,
    };
    vec![format!("\"{cli_name}\""), variant_name(cli_name)]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTIONS: &str = r#"
pub const ACTION_SPECS: &[ActionSpec] = &[
    ActionSpec {
        name: "api_get",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "help",
        required_scope: None,
        transport: ActionTransport::Any,
    },
];

pub fn rest_help() {
    let rustarr = "Alice";
}
"#;

    const MCP_ONLY_ACTIONS: &str = r#"
pub const ACTION_SPECS: &[ActionSpec] = &[
    ActionSpec {
        name: "internal_prompt",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::McpOnly,
    },
];
"#;

    #[test]
    fn action_specs_body_limits_parsing_to_metadata_block() {
        let body = action_specs_body(ACTIONS).expect("ACTION_SPECS body should parse");
        assert!(body.contains("api_get"));
        assert!(!body.contains("Alice"));
    }

    #[test]
    fn action_name_parser_ignores_non_metadata_names() {
        let body = action_specs_body(ACTIONS).unwrap();
        assert_eq!(extract_action_names(body), vec!["api_get", "help"]);
    }

    #[test]
    fn mcp_only_parser_detects_transport_restriction() {
        let body = action_specs_body(MCP_ONLY_ACTIONS).unwrap();
        assert_eq!(extract_mcp_only_actions(body), vec!["internal_prompt"]);
    }

    #[test]
    fn variant_name_matches_cli_enum_style() {
        assert_eq!(variant_name("api_get"), "ApiGet");
    }

    #[test]
    fn cli_tokens_support_action_specific_command_names() {
        assert!(cli_tokens_for_action("service_status").contains(&"Status".to_string()));
        assert!(cli_tokens_for_action("api_get").contains(&"Get".to_string()));
        assert!(cli_tokens_for_action("api_post").contains(&"Post".to_string()));
    }
}
