//! Unit tests for ExampleService — sidecar file for src/app.rs
//!
//! Declared in app.rs as:
//! ```rust
//! #[cfg(test)]
//! #[path = "app_tests.rs"]
//! mod tests;
//! ```
//!
//! The service layer tests verify that ExampleService correctly delegates to
//! ExampleClient and that any transformations or caching are correct.
//!
//! **Template**: These tests use real ExampleClient instances (pointing at a
//! stub URL), which means they test the full delegation chain without mocking.
//! For services with complex business logic, consider adding mock clients.

use super::*;
use crate::{config::ExampleConfig, example::ExampleClient};

/// Build a stub ExampleService for testing without real credentials.
fn stub_service() -> ExampleService {
    let client = ExampleClient::new(&ExampleConfig {
        api_url: "http://localhost:1/stub".to_string(),
        api_key: "test-key".to_string(),
    })
    .expect("stub client should always build");
    ExampleService::new(client)
}

#[tokio::test]
async fn test_service_greet_delegates_to_client() {
    // TEMPLATE: Test that the service correctly passes parameters through to the client.
    //           If your service adds transformation logic, test that here.
    let service = stub_service();
    let result = service.greet(None).await.expect("greet should succeed");

    assert!(
        result.get("greeting").is_some(),
        "service greet should return greeting field"
    );
}

#[tokio::test]
async fn test_service_greet_with_name_passes_name_through() {
    // TEMPLATE: Verify the service passes parameters through unchanged.
    //           If your service transforms inputs, test the transformation here.
    let service = stub_service();
    let result = service
        .greet(Some("Bob"))
        .await
        .expect("greet Bob should succeed");

    let greeting = result
        .get("greeting")
        .and_then(|v| v.as_str())
        .expect("greeting field should be present");

    assert!(
        greeting.contains("Bob"),
        "service should pass name through to client; got: {greeting}"
    );
}

#[tokio::test]
async fn test_service_echo_returns_exact_message() {
    // TEMPLATE: Test round-trip fidelity at the service layer.
    let service = stub_service();
    let msg = "service layer echo test";
    let result = service.echo(msg).await.expect("echo should succeed");

    let echo = result
        .get("echo")
        .and_then(|v| v.as_str())
        .expect("echo field should be present");

    assert_eq!(
        echo, msg,
        "service echo should return the input message unchanged"
    );
}

#[tokio::test]
async fn test_service_status_returns_ok() {
    // TEMPLATE: Status checks at the service layer should pass through correctly.
    let service = stub_service();
    let result = service.status().await.expect("status should succeed");

    assert_eq!(
        result.get("status").and_then(|v| v.as_str()),
        Some("ok"),
        "service status should return ok"
    );
}

#[test]
fn test_scaffold_intent_transformation_lives_in_service() {
    let service = stub_service();
    let result = service
        .scaffold_intent(ScaffoldIntent {
            display_name: "Lab Gateway".into(),
            crate_name: "lab-gateway-mcp".into(),
            binary_name: "lab-gateway".into(),
            server_category: "application platform".into(),
            env_prefix: "lab".into(),
            auth_kind: "api key".into(),
            host: "".into(),
            port: 3100,
            mcp_transport: "streamable-http".into(),
            mcp_primitives: "tools, resources, tools".into(),
            deployment: "containers".into(),
            plugins: "claude, gemini, none".into(),
            publish_mcp: true,
            crawl_urls: "https://docs.example.test, https://api.example.test".into(),
            crawl_repos: "".into(),
            crawl_search_topics: "Lab API".into(),
        })
        .expect("valid scaffold intent should build");

    assert_scaffold_contract_shape(&result);
    assert_eq!(result["server_category"], "application-platform");
    assert_eq!(result["project"]["service_name"], "lab_gateway");
    assert_eq!(result["project"]["env_prefix"], "LAB");
    assert_eq!(result["upstream"]["base_url_env"], "LAB_API_URL");
    assert_eq!(result["upstream"]["auth_kind"], "api-key");
    assert_eq!(result["runtime"]["host"], "127.0.0.1");
    assert_eq!(result["runtime"]["mcp_transport"], "http");
    assert_eq!(result["deployment"], "docker");
    assert_eq!(result["plugins"], serde_json::json!(["claude", "gemini"]));
    assert_eq!(
        result["mcp_primitives"],
        serde_json::json!(["tools", "resources"])
    );
}

#[test]
fn test_elicited_name_greeting_transformation_lives_in_service() {
    let service = stub_service();
    let result = service.elicited_name_greeting(ElicitedNameOutcome::Accepted("  Ada  "));

    assert_eq!(result["name"], "Ada");
    assert!(result["greeting"]
        .as_str()
        .expect("greeting should be a string")
        .contains("Ada"));
}

#[test]
fn test_elicited_name_fallback_outcomes_are_covered_below_live_mcporter() {
    let service = stub_service();

    assert_eq!(
        service.elicited_name_greeting(ElicitedNameOutcome::NoInput)["greeting"],
        "Hello! (you provided no name - that's okay)"
    );
    assert_eq!(
        service.elicited_name_greeting(ElicitedNameOutcome::Declined)["greeting"],
        "Hello, anonymous user!"
    );
    assert_eq!(
        service.elicited_name_greeting(ElicitedNameOutcome::Cancelled)["greeting"],
        "Hello there!"
    );
    assert_eq!(
        service.elicited_name_greeting(ElicitedNameOutcome::Unsupported)["fallback_greeting"],
        "Hello, World! (elicitation unavailable)"
    );
}

#[test]
fn test_scaffold_intent_rejects_invalid_contract_identifiers() {
    let service = stub_service();
    let result = service.scaffold_intent(ScaffoldIntent {
        display_name: "Bad Project".into(),
        crate_name: "Invalid Crate".into(),
        binary_name: "bad".into(),
        server_category: "upstream-client".into(),
        env_prefix: "bad".into(),
        auth_kind: "api-key".into(),
        host: "127.0.0.1".into(),
        port: 3100,
        mcp_transport: "dual".into(),
        mcp_primitives: "tools".into(),
        deployment: "none".into(),
        plugins: "".into(),
        publish_mcp: false,
        crawl_urls: "".into(),
        crawl_repos: "".into(),
        crawl_search_topics: "".into(),
    });

    let error = result.expect_err("invalid crate_name should be rejected");
    assert!(error.to_string().contains("crate_name"));
}

#[test]
fn test_scaffold_intent_rejects_zero_port_and_bad_urls() {
    let service = stub_service();
    let mut input = valid_scaffold_intent();
    input.port = 0;
    let error = service
        .scaffold_intent(input)
        .expect_err("zero port should be rejected");
    assert!(error.to_string().contains("port"));

    let mut input = valid_scaffold_intent();
    input.crawl_urls = "not-a-url".into();
    let error = service
        .scaffold_intent(input)
        .expect_err("bad crawl URL should be rejected");
    assert!(error.to_string().contains("crawl_urls"));
}

#[test]
fn test_scaffold_intent_deduplicates_contract_unique_arrays() {
    let service = stub_service();
    let mut input = valid_scaffold_intent();
    input.crawl_urls = "https://docs.example.test, https://docs.example.test".into();

    let result = service
        .scaffold_intent(input)
        .expect("duplicate crawl URLs should be deduplicated");

    assert_eq!(
        result["crawl_docs"]["urls"],
        serde_json::json!(["https://docs.example.test"])
    );
    assert_scaffold_contract_shape(&result);
}

fn valid_scaffold_intent() -> ScaffoldIntent {
    ScaffoldIntent {
        display_name: "Lab Gateway".into(),
        crate_name: "lab-gateway-mcp".into(),
        binary_name: "lab-gateway".into(),
        server_category: "application platform".into(),
        env_prefix: "lab".into(),
        auth_kind: "api key".into(),
        host: "".into(),
        port: 3100,
        mcp_transport: "streamable-http".into(),
        mcp_primitives: "tools, resources, tools".into(),
        deployment: "containers".into(),
        plugins: "claude, gemini, none".into(),
        publish_mcp: true,
        crawl_urls: "https://docs.example.test, https://api.example.test".into(),
        crawl_repos: "".into(),
        crawl_search_topics: "Lab API".into(),
    }
}

fn assert_scaffold_contract_shape(value: &Value) {
    assert_eq!(value["kind"], "rmcp_template_scaffold_intent");
    assert_eq!(value["schema_version"], 1);
    assert_non_empty_string(&value["project"]["display_name"]);
    assert_matches_kebab(&value["project"]["crate_name"]);
    assert_matches_kebab(&value["project"]["binary_name"]);
    assert_matches_service_name(&value["project"]["service_name"]);
    assert_matches_env_prefix(&value["project"]["env_prefix"]);
    assert!(
        value["runtime"]["port"]
            .as_u64()
            .is_some_and(|port| port > 0),
        "runtime.port must be a positive integer"
    );
}

fn assert_non_empty_string(value: &Value) {
    assert!(
        value.as_str().is_some_and(|value| !value.is_empty()),
        "expected non-empty string, got {value}"
    );
}

fn assert_matches_kebab(value: &Value) {
    let value = value.as_str().expect("expected string");
    let mut chars = value.chars();
    assert!(chars.next().is_some_and(|ch| ch.is_ascii_lowercase()));
    assert!(chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'));
}

fn assert_matches_service_name(value: &Value) {
    let value = value.as_str().expect("expected string");
    let mut chars = value.chars();
    assert!(chars.next().is_some_and(|ch| ch.is_ascii_lowercase()));
    assert!(chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'));
}

fn assert_matches_env_prefix(value: &Value) {
    let value = value.as_str().expect("expected string");
    let mut chars = value.chars();
    assert!(chars.next().is_some_and(|ch| ch.is_ascii_uppercase()));
    assert!(chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'));
}
