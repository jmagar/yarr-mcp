use super::{reporter::PatternReporter, util::read_file};

const ACTION_TEST_COVERAGE_EXCEPTIONS: &[&str] = &[
    // Requires a live MCP Peer<RoleServer>; covered by parser/schema/help checks instead.
    "elicit_name",
];

pub(super) fn action_surfaces(reporter: &mut PatternReporter) {
    let actions_text = read_file("src/actions.rs");
    let action_specs = action_specs_body(&actions_text).unwrap_or(&actions_text);
    let action_names = extract_action_names(action_specs);
    let mcp_only = extract_mcp_only_actions(action_specs);

    if action_names.is_empty() {
        reporter.fail(
            "actions",
            "could not parse ACTION_SPECS from src/actions.rs",
        );
        return;
    }

    let schema = read_file("src/mcp/schemas.rs");
    let tools = read_file("src/mcp/tools.rs");
    let tests = read_file("tests/tool_dispatch.rs");
    let cli = read_file("src/cli.rs");

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
    let missing_help = action_names
        .iter()
        .filter(|action| {
            !tools.contains(&format!("### {action}")) && !tools.contains(&format!("`{action}`"))
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
                "mcp/tools.rs HELP_TEXT missing action(s): {}. Hint: add `### <action>` docs to HELP_TEXT.",
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
        name: "greet",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::Any,
    },
    ActionSpec {
        name: "elicit_name",
        required_scope: Some(READ_SCOPE),
        transport: ActionTransport::McpOnly,
    },
];

pub fn rest_help() {
    let rustarr = "Alice";
}
"#;

    #[test]
    fn action_specs_body_limits_parsing_to_metadata_block() {
        let body = action_specs_body(ACTIONS).expect("ACTION_SPECS body should parse");
        assert!(body.contains("greet"));
        assert!(!body.contains("Alice"));
    }

    #[test]
    fn action_name_parser_ignores_non_metadata_names() {
        let body = action_specs_body(ACTIONS).unwrap();
        assert_eq!(extract_action_names(body), vec!["greet", "elicit_name"]);
    }

    #[test]
    fn mcp_only_parser_detects_transport_restriction() {
        let body = action_specs_body(ACTIONS).unwrap();
        assert_eq!(extract_mcp_only_actions(body), vec!["elicit_name"]);
    }

    #[test]
    fn variant_name_matches_cli_enum_style() {
        assert_eq!(variant_name("elicit_name"), "ElicitName");
    }

    #[test]
    fn cli_tokens_support_action_specific_command_names() {
        assert!(cli_tokens_for_action("service_status").contains(&"Status".to_string()));
        assert!(cli_tokens_for_action("api_get").contains(&"Get".to_string()));
        assert!(cli_tokens_for_action("api_post").contains(&"Post".to_string()));
    }
}
