//! Unit tests for ExampleClient — sidecar file for src/example.rs
//!
//! # Sidecar test pattern
//!
//! Tests live in a separate `*_tests.rs` file (this file) rather than inline in
//! `example.rs`. The parent module declares them with:
//!
//! ```rust
//! #[cfg(test)]
//! #[path = "example_tests.rs"]
//! mod tests;
//! ```
//!
//! Benefits of the sidecar pattern:
//!   - `example.rs` stays focused on production code — no test boilerplate
//!   - Tests can be found quickly (always `<module>_tests.rs`)
//!   - Large test suites don't inflate the source file line count
//!   - IDE navigation: open `example.rs`, jump to `mod tests`, find the file
//!
//! **Template**: Copy this pattern for every module that needs unit tests.
//!   1. Create `src/<module>_tests.rs`
//!   2. Add `#[cfg(test)] #[path = "<module>_tests.rs"] mod tests;` to `src/<module>.rs`
//!   3. Write tests here — they can access `pub(crate)` items via `super::*`

use super::*;
use crate::config::ExampleConfig;

/// Helper: build a stub ExampleConfig pointing at a non-existent local server.
/// Tests do not make real network calls — the stub client methods return mock data.
fn stub_config() -> ExampleConfig {
    ExampleConfig {
        // Points at a port nothing listens on — safe for offline tests.
        // TEMPLATE: Replace with your service's config struct fields.
        api_url: "http://localhost:1/stub".to_string(),
        api_key: "test-key".to_string(),
    }
}

#[tokio::test]
async fn test_greet_returns_greeting_field() {
    // TEMPLATE: Replace greet() with your client's first operation.
    //           Test that the response has the expected JSON shape.
    let client = ExampleClient::new(&stub_config()).expect("stub client should build");
    let result = client.greet(None).await.expect("greet should succeed");

    assert!(
        result.get("greeting").is_some(),
        "greet response should have 'greeting' field, got: {result}"
    );
}

#[tokio::test]
async fn test_greet_with_name_includes_name_in_response() {
    // TEMPLATE: Test that input parameters are reflected in the output.
    //           This is a semantic test — not just "did it return JSON" but
    //           "did it return the RIGHT JSON for this input".
    let client = ExampleClient::new(&stub_config()).expect("stub client should build");
    let result = client
        .greet(Some("Alice"))
        .await
        .expect("greet Alice should succeed");

    let greeting = result
        .get("greeting")
        .and_then(|v| v.as_str())
        .expect("greeting field should be a string");

    assert!(
        greeting.contains("Alice"),
        "greeting should include the provided name 'Alice', got: {greeting}"
    );
}

#[tokio::test]
async fn test_greet_default_name_is_world() {
    // TEMPLATE: Test default/fallback behavior explicitly.
    let client = ExampleClient::new(&stub_config()).expect("stub client should build");
    let result = client.greet(None).await.expect("greet should succeed");

    let target = result
        .get("target")
        .and_then(|v| v.as_str())
        .expect("target field should be present");

    assert_eq!(target, "World", "default greeting target should be 'World'");
}

#[tokio::test]
async fn test_echo_returns_exact_message() {
    // TEMPLATE: For operations that pass data through, verify the round-trip exactly.
    //           "is it JSON?" is not a good test. "does it contain the right value?" is.
    let client = ExampleClient::new(&stub_config()).expect("stub client should build");
    let message = "hello from the test suite";
    let result = client.echo(message).await.expect("echo should succeed");

    let echo = result
        .get("echo")
        .and_then(|v| v.as_str())
        .expect("echo field should be present");

    assert_eq!(echo, message, "echo should return the exact input message");
}

#[tokio::test]
async fn test_status_returns_ok() {
    // TEMPLATE: Status/health operations should always return a known good value.
    let client = ExampleClient::new(&stub_config()).expect("stub client should build");
    let result = client.status().await.expect("status should succeed");

    let status = result
        .get("status")
        .and_then(|v| v.as_str())
        .expect("status field should be present");

    assert_eq!(status, "ok");
}

#[test]
fn test_client_builds_with_empty_config() {
    // TEMPLATE: Verify that the client can be constructed even with empty credentials.
    //           In the template, this is intentional (the stub allows it).
    //           A real server would error here — update this test to expect an Err.
    let config = ExampleConfig {
        api_url: String::new(),
        api_key: String::new(),
    };
    let result = ExampleClient::new(&config);
    // TEMPLATE: Change to assert!(result.is_err()) once you add real validation
    assert!(
        result.is_ok(),
        "stub client should build even with empty config (real server should validate)"
    );
}
