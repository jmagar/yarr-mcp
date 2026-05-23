//! Unit tests for src/mcp/transport.rs
//!
//! Declared in transport.rs as:
//! ```rust
//! #[cfg(test)]
//! #[path = "transport_tests.rs"]
//! mod tests;
//! ```
//!
//! Tests cover the deterministic host/origin computation logic. All tests are
//! synchronous and need no network access.

use super::*;
use crate::config::{AuthConfig, McpConfig};

fn config(host: &str, port: u16) -> McpConfig {
    McpConfig {
        host: host.to_string(),
        port,
        ..Default::default()
    }
}

// ── allowed_hosts ─────────────────────────────────────────────────────────────

#[test]
fn allowed_hosts_always_includes_loopback() {
    let hosts = allowed_hosts(&config("0.0.0.0", 3000));
    assert!(hosts.contains(&"localhost".to_string()));
    assert!(hosts.contains(&"127.0.0.1".to_string()));
}

#[test]
fn allowed_hosts_includes_bound_host_and_port_variant() {
    let hosts = allowed_hosts(&config("myhost.rustarr.com", 8080));
    assert!(hosts.contains(&"myhost.rustarr.com".to_string()));
    assert!(hosts.contains(&"myhost.rustarr.com:8080".to_string()));
}

#[test]
fn allowed_hosts_deduplicates() {
    let hosts = allowed_hosts(&config("localhost", 3000));
    let localhost_count = hosts.iter().filter(|h| h.as_str() == "localhost").count();
    assert_eq!(localhost_count, 1, "localhost should appear exactly once");
}

#[test]
fn allowed_hosts_with_extra_allowed_hosts() {
    let cfg = McpConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        allowed_hosts: vec!["proxy.internal".to_string()],
        ..Default::default()
    };
    let hosts = allowed_hosts(&cfg);
    assert!(hosts.contains(&"proxy.internal".to_string()));
    assert!(hosts.contains(&"proxy.internal:3000".to_string()));
}

#[test]
fn allowed_hosts_ipv6_loopback_bracketed() {
    let hosts = allowed_hosts(&config("0.0.0.0", 3000));
    assert!(
        hosts.iter().any(|h| h.contains("::1")),
        "IPv6 loopback should be present"
    );
}

// ── allowed_origins ───────────────────────────────────────────────────────────

#[test]
fn allowed_origins_includes_loopback_with_port() {
    let origins = allowed_origins(&config("0.0.0.0", 4000));
    assert!(origins.contains(&"http://localhost:4000".to_string()));
    assert!(origins.contains(&"http://127.0.0.1:4000".to_string()));
}

#[test]
fn allowed_origins_deduplicates() {
    let origins = allowed_origins(&config("localhost", 4000));
    let count = origins
        .iter()
        .filter(|o| o.as_str() == "http://localhost:4000")
        .count();
    assert_eq!(count, 1);
}

#[test]
fn allowed_origins_includes_extra_allowed_origins() {
    let cfg = McpConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        allowed_origins: vec!["https://app.rustarr.com".to_string()],
        ..Default::default()
    };
    let origins = allowed_origins(&cfg);
    assert!(origins.contains(&"https://app.rustarr.com".to_string()));
}

#[test]
fn allowed_origins_normalizes_extra_allowed_origins() {
    let cfg = McpConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        allowed_origins: vec!["https://app.rustarr.com/some/path?ignored=true".to_string()],
        ..Default::default()
    };
    let origins = allowed_origins(&cfg);
    assert!(origins.contains(&"https://app.rustarr.com".to_string()));
    assert!(!origins.contains(&"https://app.rustarr.com/some/path?ignored=true".to_string()));
}

#[test]
fn allowed_origins_skips_invalid_and_wildcard_origins() {
    let cfg = McpConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        allowed_origins: vec!["not-a-url".to_string(), "https://*.rustarr.com".to_string()],
        ..Default::default()
    };
    let origins = allowed_origins(&cfg);
    assert!(!origins.contains(&"not-a-url".to_string()));
    assert!(!origins.contains(&"https://*.rustarr.com".to_string()));
}

#[test]
fn allowed_origins_preserves_non_http_configured_origins() {
    let cfg = McpConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        allowed_origins: vec!["vscode-webview://extension.rustarr".to_string()],
        ..Default::default()
    };
    let origins = allowed_origins(&cfg);
    assert!(origins.contains(&"vscode-webview://extension.rustarr".to_string()));
}

#[test]
fn allowed_origins_brackets_ipv6_literals() {
    let cfg = McpConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        allowed_origins: vec!["http://[::1]:3000/path?ignored=true".to_string()],
        ..Default::default()
    };
    let origins = allowed_origins(&cfg);
    assert!(origins.contains(&"http://[::1]:3000".to_string()));
    assert!(!origins.contains(&"http://::1:3000".to_string()));
}

#[test]
fn allowed_origins_includes_public_url_origin() {
    let cfg = McpConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        auth: AuthConfig {
            public_url: Some("https://mcp.rustarr.com".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let origins = allowed_origins(&cfg);
    assert!(origins.contains(&"https://mcp.rustarr.com".to_string()));
}

// ── has_port (private, tested via allowed_hosts behaviour) ───────────────────

#[test]
fn host_with_port_not_doubled() {
    // "myhost:8080" already has a port — push_host_variants must not append another.
    let cfg = McpConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        allowed_hosts: vec!["proxy:9000".to_string()],
        ..Default::default()
    };
    let hosts = allowed_hosts(&cfg);
    assert!(hosts.contains(&"proxy:9000".to_string()));
    assert!(
        !hosts.contains(&"proxy:9000:3000".to_string()),
        "port must not be appended twice"
    );
}
