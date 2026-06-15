//! Unit tests for src/cli/doctor/checks.rs
//!
//! Declared in checks.rs as:
//! ```rust
//! #[cfg(test)]
//! #[path = "checks_tests.rs"]
//! mod tests;
//! ```
//!
//! Tests cover the pure and near-pure check functions. Async network checks
//! (`check_upstream`) and filesystem-heavy checks are covered with minimal
//! scaffolding.

use super::*;
use crate::{
    app::RustarrService,
    config::{McpConfig, RustarrConfig, ServiceConfig, ServiceKind},
    rustarr::RustarrClient,
};

// ── check_required_var ────────────────────────────────────────────────────────

#[test]
fn required_var_passes_when_set() {
    let check = check_required_var("MY_VAR", "some-value");
    assert!(check.ok, "non-empty value should pass");
    assert_eq!(check.category, "credentials");
    let value = check.value.expect("pass should have a value");
    assert!(value.contains("(set)"), "pass value should mention (set)");
    assert!(
        !value.contains("some-value"),
        "actual secret must be redacted"
    );
}

#[test]
fn required_var_fails_when_empty() {
    let check = check_required_var("MY_VAR", "");
    assert!(!check.ok, "empty value should fail");
    let hint = check.hint.expect("fail should have a hint");
    assert!(hint.contains("MY_VAR"), "hint should name the missing var");
}

#[test]
fn required_var_redacts_short_secrets() {
    let check = check_required_var("KEY", "abc");
    let value = check.value.unwrap();
    assert!(!value.contains("abc"), "short secret must be fully masked");
}

#[test]
fn required_var_redacts_long_secrets() {
    let check = check_required_var("KEY", "supersecrettoken");
    let value = check.value.unwrap();
    assert!(
        !value.contains("supersecrettoken"),
        "long secret must not appear in full"
    );
    assert!(
        value.contains("****"),
        "long secret should show mask suffix"
    );
}

// ── check_service_url ────────────────────────────────────────────────────────

#[test]
fn service_url_passes_when_set() {
    let check = check_service_url("sonarr", "https://sonarr.local");
    assert!(check.ok, "non-empty base URL should pass");
    assert_eq!(check.category, "credentials");
    // Only the origin is shown.
    assert_eq!(check.value.as_deref(), Some("https://sonarr.local"));
}

#[test]
fn service_url_shows_only_origin_not_credentials() {
    // Userinfo / query secrets in the configured URL must NOT appear in the
    // doctor detail (it is printed verbatim, including under --json).
    let check = check_service_url(
        "sonarr",
        "http://admin:hunter2@sonarr.local:8989/?apikey=sekret",
    );
    assert!(check.ok);
    let shown = check.value.as_deref().unwrap_or("");
    assert_eq!(shown, "http://sonarr.local:8989");
    assert!(!shown.contains("hunter2"), "password leaked: {shown}");
    assert!(!shown.contains("sekret"), "apikey leaked: {shown}");
}

#[test]
fn service_url_fails_when_empty() {
    let check = check_service_url("sonarr", "");
    assert!(!check.ok, "empty base URL should fail");
    let hint = check.hint.expect("fail should have a hint");
    assert!(
        hint.contains("RUSTARR_SONARR_URL"),
        "hint should name the uppercased env var"
    );
    assert!(hint.contains("sonarr"), "hint should name the service");
}

// ── check_binary_in_path ─────────────────────────────────────────────────────

#[test]
fn binary_in_path_passes_for_sh() {
    // /bin/sh or /usr/bin/sh is on PATH in any POSIX system.
    let check = check_binary_in_path("sh");
    assert!(check.ok, "sh should be found in PATH");
    assert_eq!(check.category, "config");
}

#[test]
fn binary_in_path_fails_for_nonexistent() {
    let check = check_binary_in_path("this-binary-definitely-does-not-exist-rmcp");
    assert!(!check.ok, "unknown binary should fail");
    let hint = check.hint.unwrap();
    assert!(hint.contains("PATH"), "hint should mention PATH");
}

// ── check_port_available ─────────────────────────────────────────────────────

#[tokio::test]
async fn port_available_passes_for_free_port() {
    use std::net::TcpListener;
    // Bind to port 0 to get an OS-assigned ephemeral port, then drop the
    // listener so the port is free before calling check_port_available.
    let listener = TcpListener::bind("127.0.0.1:0").expect("should bind to an ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener); // release the port before the check

    let check = check_port_available("127.0.0.1", port).await;
    assert_eq!(check.category, "server");
    assert!(check.ok, "a free port should pass the availability check");
}

#[tokio::test]
async fn port_available_fails_when_already_bound_by_non_rustarr_process() {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("should bind to an ephemeral port");
    let port = listener.local_addr().unwrap().port();

    let check = check_port_available("127.0.0.1", port).await;
    assert!(!check.ok, "port in use should fail");
    assert!(
        check.hint.unwrap().contains(&port.to_string()),
        "hint should name the port"
    );
}

#[tokio::test]
async fn port_available_passes_when_running_server_is_healthy() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("should bind to an ephemeral port");
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("should accept one request");
        let mut buffer = [0_u8; 1024];
        let _ = stream.read(&mut buffer);
        let body = br#"{"status":"ok"}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
    });

    let check = check_port_available("127.0.0.1", port).await;
    handle.join().unwrap();
    assert!(check.ok, "healthy running server should pass doctor");
    assert!(check.value.unwrap().contains("/health"));
}

// ── check_config_file ────────────────────────────────────────────────────────

#[test]
fn config_file_passes_when_present() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, b"[mcp]\nport = 3000\n").unwrap();

    let check = check_config_file(dir.path());
    assert!(check.ok);
    assert!(check.value.unwrap().contains("config.toml"));
}

#[test]
fn config_file_passes_gracefully_when_absent() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let check = check_config_file(dir.path());
    // Missing config.toml is a soft pass (env vars cover it).
    assert!(check.ok, "missing config.toml should not hard-fail");
    assert!(
        check.value.unwrap().contains("not found"),
        "value should note the file is missing"
    );
}

// ── check_dir_writable ───────────────────────────────────────────────────────

#[test]
fn dir_writable_passes_for_writable_dir() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let check = check_dir_writable("Test dir", dir.path());
    assert!(check.ok);
    assert!(check.value.unwrap().contains("writable"));
}

#[cfg(unix)]
#[test]
fn dir_writable_does_not_recurse_into_symlinked_children() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    std::os::unix::fs::symlink(dir.path(), dir.path().join("loop")).unwrap();

    let check = check_dir_writable("Test dir", dir.path());
    assert!(
        check.ok,
        "writability check should not traverse symlinked children"
    );
}

#[tokio::test]
async fn upstream_passes_for_local_service_status_endpoint() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("should bind test server");
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("should accept one request");
        let mut buffer = [0_u8; 1024];
        let _ = stream.read(&mut buffer);
        let body = br#"{"version":"1.0.0"}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
    });

    let config = RustarrConfig {
        services: vec![ServiceConfig {
            name: "sonarr".into(),
            kind: ServiceKind::Sonarr,
            base_url: format!("http://{addr}"),
            api_key: Some("key".into()),
            ..ServiceConfig::default()
        }],
    };
    let client = RustarrClient::new(&config).unwrap();
    let service = RustarrService::new(client, config);
    let check = check_upstream(&service, "sonarr").await;
    handle.join().unwrap();

    assert!(check.ok, "local service_status response should pass");
    assert_eq!(check.category, "connectivity");
}

fn auth_config(host: &str) -> Config {
    Config {
        rustarr: RustarrConfig {
            services: vec![crate::config::ServiceConfig {
                name: "sonarr".into(),
                kind: crate::config::ServiceKind::Sonarr,
                base_url: "https://rustarr.test".into(),
                api_key: Some("secret".into()),
                ..crate::config::ServiceConfig::default()
            }],
        },
        mcp: McpConfig {
            host: host.into(),
            ..McpConfig::default()
        },
    }
}

#[test]
fn auth_config_passes_loopback_no_auth() {
    let mut config = auth_config("127.0.0.1");
    config.mcp.no_auth = true;

    let check = check_auth_config(&config);

    assert!(check.ok);
    assert!(check.value.unwrap().contains("loopback"));
}

#[test]
fn auth_config_passes_typed_trusted_gateway() {
    let mut config = auth_config("0.0.0.0");
    config.mcp.trusted_gateway = true;
    config.mcp.allowed_hosts = vec!["rustarr.example.com".into()];

    let check = check_auth_config(&config);

    assert!(check.ok);
    assert!(check.value.unwrap().contains("trusted gateway"));
}

#[test]
fn auth_config_rejects_non_loopback_without_auth() {
    let config = auth_config("0.0.0.0");

    let check = check_auth_config(&config);

    assert!(!check.ok);
    assert!(check.hint.unwrap().contains("RUSTARR_MCP_TOKEN"));
}
