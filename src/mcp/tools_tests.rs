use super::*;
use crate::{config::ExampleConfig, example::ExampleClient};

fn service() -> ExampleService {
    let client = ExampleClient::new(&ExampleConfig {
        api_url: "http://localhost:1/stub".to_owned(),
        api_key: "test-key".to_owned(),
    })
    .expect("stub client should build");
    ExampleService::new(client)
}

fn upstream_input() -> ScaffoldIntentInput {
    ScaffoldIntentInput {
        display_name: "Unraid MCP".to_owned(),
        crate_name: "unraid-mcp".to_owned(),
        binary_name: "unraid".to_owned(),
        server_category: "upstream-client".to_owned(),
        env_prefix: "unraid".to_owned(),
        auth_kind: "api key".to_owned(),
        host: "".to_owned(),
        port: 3100,
        mcp_transport: "dual".to_owned(),
        mcp_primitives: "tools, resources, prompts, elicitation".to_owned(),
        deployment: "none".to_owned(),
        plugins: "claude, codex, claude".to_owned(),
        publish_mcp: true,
        crawl_urls: "https://docs.unraid.net/".to_owned(),
        crawl_repos: "".to_owned(),
        crawl_search_topics: "Unraid API authentication".to_owned(),
    }
}

#[test]
fn scaffold_intent_json_matches_simplified_contract_shape() {
    let value = service()
        .scaffold_intent(upstream_input().into())
        .expect("valid scaffold intent should build");

    assert_eq!(value["kind"], "rmcp_template_scaffold_intent");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["server_category"], "upstream-client");
    assert_eq!(value["required_surfaces"], json!(["mcp", "cli"]));
    assert_eq!(value["project"]["env_prefix"], "UNRAID");
    assert_eq!(
        value["upstream"],
        json!({
            "base_url_env": "UNRAID_API_URL",
            "auth_kind": "api-key"
        })
    );
    assert_eq!(
        value["runtime"],
        json!({
            "host": "127.0.0.1",
            "port": 3100,
            "mcp_transport": "dual"
        })
    );
    assert_eq!(
        value["mcp_primitives"],
        json!(["tools", "resources", "prompts", "elicitation"])
    );
    assert_eq!(value["plugins"], json!(["claude", "codex"]));
    assert_eq!(value["publish_mcp"], true);
    assert_eq!(
        value["crawl_docs"]["urls"],
        json!(["https://docs.unraid.net/"])
    );
    assert!(value.get("actions").is_none());
    assert!(value["upstream"].get("resource_groups").is_none());
}

#[test]
fn application_platform_intent_requires_all_surfaces() {
    let mut input = upstream_input();
    input.server_category = "application platform".to_owned();
    input.auth_kind = "both".to_owned();
    input.host = "0.0.0.0".to_owned();
    input.mcp_transport = "streamable-http".to_owned();
    input.deployment = "container".to_owned();
    input.plugins = "claude, codex, gemini".to_owned();
    input.crawl_repos = "https://github.com/example/lab-sdk".to_owned();

    let value = service()
        .scaffold_intent(input.into())
        .expect("valid scaffold intent should build");

    assert_eq!(value["server_category"], "application-platform");
    assert_eq!(
        value["required_surfaces"],
        json!(["api", "cli", "mcp", "web"])
    );
    assert_eq!(value["upstream"]["auth_kind"], "both");
    assert_eq!(value["runtime"]["mcp_transport"], "http");
    assert_eq!(value["deployment"], "docker");
    assert_eq!(value["plugins"], json!(["claude", "codex", "gemini"]));
    assert_eq!(
        value["crawl_docs"]["repos"],
        json!(["https://github.com/example/lab-sdk"])
    );
}

#[test]
fn scaffold_intent_json_contains_contract_required_fields() {
    let value = service()
        .scaffold_intent(upstream_input().into())
        .expect("valid scaffold intent should build");
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/contracts/scaffold-intent.schema.json"
    ))
    .expect("contract should be valid JSON");
    let required = contract["required"]
        .as_array()
        .expect("contract should list root required fields");

    for field in required {
        let field = field.as_str().expect("required fields should be strings");
        assert!(
            value.get(field).is_some(),
            "missing contract field: {field}"
        );
    }
}

#[test]
fn primitive_defaults_to_tools_when_input_is_empty() {
    let mut input = upstream_input();
    input.mcp_primitives.clear();
    let value = service()
        .scaffold_intent(input.into())
        .expect("valid scaffold intent should build");
    assert_eq!(value["mcp_primitives"], json!(["tools"]));
}
