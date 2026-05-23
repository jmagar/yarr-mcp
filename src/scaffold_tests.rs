use super::*;

fn valid_input() -> ScaffoldIntent {
    ScaffoldIntent {
        display_name: "Unraid MCP".into(),
        crate_name: "unraid-mcp".into(),
        binary_name: "unraid".into(),
        server_category: "upstream-client".into(),
        env_prefix: "UNRAID".into(),
        auth_kind: "api key".into(),
        host: "127.0.0.1".into(),
        port: 3100,
        mcp_transport: "dual".into(),
        mcp_primitives: "tools, resources, prompts, elicitation".into(),
        deployment: "none".into(),
        plugins: "claude, codex".into(),
        publish_mcp: true,
        crawl_urls: "https://docs.unraid.net/".into(),
        crawl_repos: "".into(),
        crawl_search_topics: "Unraid API authentication".into(),
    }
}

#[test]
fn builds_scaffold_handoff_contract() {
    let payload = build_scaffold_intent(valid_input()).unwrap();

    assert_eq!(payload["kind"], "rustarr_scaffold_intent");
    assert_eq!(payload["schema_version"], 1);
    assert_eq!(payload["server_category"], "upstream-client");
    assert_eq!(
        payload["required_surfaces"],
        serde_json::json!(["mcp", "cli"])
    );
    assert_eq!(payload["project"]["service_name"], "unraid");
    assert_eq!(payload["upstream"]["base_url_env"], "UNRAID_API_URL");
    assert_eq!(payload["upstream"]["auth_kind"], "api-key");
    assert_eq!(payload["handoff"]["recommended_skill"], "scaffold-project");
}

#[test]
fn application_platform_uses_full_surface_policy() {
    let mut input = valid_input();
    input.server_category = "application platform".into();
    input.deployment = "docker".into();
    input.plugins = "claude, codex, gemini".into();

    let payload = build_scaffold_intent(input).unwrap();

    assert_eq!(payload["server_category"], "application-platform");
    assert_eq!(
        payload["required_surfaces"],
        serde_json::json!(["api", "cli", "mcp", "web"])
    );
    assert_eq!(payload["deployment"], "docker");
    assert_eq!(
        payload["plugins"],
        serde_json::json!(["claude", "codex", "gemini"])
    );
}

#[test]
fn rejects_invalid_scaffold_inputs() {
    let mut input = valid_input();
    input.crate_name = "Bad_Name".into();
    let error = build_scaffold_intent(input).unwrap_err();
    assert!(error.to_string().contains("crate_name"));

    let mut input = valid_input();
    input.crawl_urls = "not a url".into();
    let error = build_scaffold_intent(input).unwrap_err();
    assert!(error.to_string().contains("crawl_urls"));
}
