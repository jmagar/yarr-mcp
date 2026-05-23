//! Stub transport client for the Example service.
//!
//! **Template note**: This is a placeholder for whatever HTTP/GraphQL/gRPC client
//! your real server needs. Replace the methods with actual network calls.
//!
//! The pattern:
//!   - `ExampleClient::new()` builds the transport (HTTP client, connection pool, etc.)
//!   - Each method corresponds to one remote operation and returns `Result<Value>`
//!   - `ExampleService` in `app.rs` wraps this and adds any business logic
//!   - MCP tools in `mcp/tools.rs` call `ExampleService`, never `ExampleClient` directly

use anyhow::Result;
use serde_json::{json, Value};

use crate::config::ExampleConfig;

// Unit tests live in a sidecar file — see src/example_tests.rs for the pattern.
// TEMPLATE: Copy this block into every module that needs unit tests.
#[cfg(test)]
#[path = "example_tests.rs"]
mod tests;

/// HTTP (or other transport) client for the example remote service.
///
/// In a real server this would hold a `reqwest::Client`, connection pool, base URL, etc.
// These fields are intentionally kept as stubs — a real implementation would use them.
#[allow(dead_code)]
#[derive(Clone)]
pub struct ExampleClient {
    /// Base URL of the remote service (from `EXAMPLE_API_URL`).
    api_url: String,
    /// API key or bearer token (from `EXAMPLE_API_KEY`).
    api_key: String,
    // In a real server you'd also have:
    //   client: reqwest::Client,
}

impl ExampleClient {
    /// Construct a new client from configuration.
    ///
    /// Returns an error if required config values are missing — this is intentional
    /// so startup fails loudly rather than silently falling back to empty strings.
    ///
    /// **Template**: replace this with real validation / client construction.
    pub fn new(cfg: &ExampleConfig) -> Result<Self> {
        // In a template, we allow empty values so the stub works without real creds.
        // A real server would `bail!` here:
        //   if cfg.api_url.is_empty() { anyhow::bail!("EXAMPLE_API_URL is not set"); }
        //   if cfg.api_key.is_empty() { anyhow::bail!("EXAMPLE_API_KEY is not set"); }

        // Build reqwest client if you need one:
        //   let client = reqwest::ClientBuilder::new()
        //       .timeout(std::time::Duration::from_secs(30))
        //       .build()
        //       .context("failed to build HTTP client")?;

        Ok(Self {
            api_url: cfg.api_url.clone(),
            api_key: cfg.api_key.clone(),
        })
    }

    // ── placeholder operations ────────────────────────────────────────────────
    // Each of these would be a real HTTP/GraphQL/gRPC call in a production server.
    // They return serde_json::Value so the service layer can transform them freely.

    /// Say hello to `name`, or "World" if not provided.
    pub async fn greet(&self, name: Option<&str>) -> Result<Value> {
        let target = name.unwrap_or("World");
        // Real implementation would call:
        //   self.client.get(format!("{}/greet", self.api_url))
        //       .bearer_auth(&self.api_key)
        //       .query(&[("name", target)])
        //       .send().await?
        //       .json().await?
        Ok(json!({
            "greeting": format!("Hello, {target}!"),
            "target": target,
            "server": self.api_url,
        }))
    }

    /// Echo a message back unchanged.
    pub async fn echo(&self, message: &str) -> Result<Value> {
        Ok(json!({ "echo": message }))
    }

    /// Return a status snapshot of the remote service.
    ///
    /// Note: this value is returned by the unauthenticated `/status` endpoint,
    /// so it must not include secrets or sensitive topology (e.g. `api_url`).
    /// TEMPLATE: Add non-sensitive runtime metrics (uptime, version, etc.).
    pub async fn status(&self) -> Result<Value> {
        Ok(json!({
            "status": "ok",
            // api_url intentionally omitted — topology leak on unauthenticated endpoint.
            "note": "stub — replace with real health endpoint",
        }))
    }
}
