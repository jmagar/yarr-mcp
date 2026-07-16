use std::{fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::tempdir;

use super::common::{assert_private_file, yarr_bin};

fn oauth_setup_command(data_dir: &Path) -> Command {
    let mut cmd = Command::new(yarr_bin());
    cmd.env_clear()
        .env("HOME", data_dir)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("YARR_HOME", data_dir)
        .env("YARR_SERVICES", "sonarr")
        .env("YARR_SONARR_KIND", "sonarr")
        .env("YARR_SONARR_URL", "https://api.yarr.test")
        .env("YARR_SONARR_API_KEY", "yarr-secret")
        .env("YARR_MCP_PORT", "0")
        .env("YARR_MCP_AUTH_MODE", "oauth")
        .env("YARR_MCP_PUBLIC_URL", "https://mcp.yarr.test")
        .env("YARR_MCP_GOOGLE_CLIENT_ID", "test-client-id")
        .env("YARR_MCP_GOOGLE_CLIENT_SECRET", "test-client-secret")
        .env("YARR_MCP_AUTH_ADMIN_EMAIL", "admin@yarr.test");
    cmd
}

fn blocking_failure_codes(output: &std::process::Output) -> Vec<String> {
    let json: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout not JSON: {error}\nstdout: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    json["blocking_failures"]
        .as_array()
        .expect("blocking_failures should be an array")
        .iter()
        .map(|failure| failure["code"].as_str().unwrap_or("").to_owned())
        .collect()
}

fn assert_missing_oauth_field(env_var: &str, expected_code: &str) {
    let dir = tempdir().unwrap();
    let mut command = oauth_setup_command(dir.path());
    command.env_remove(env_var);
    let output = command
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected blocking failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let codes = blocking_failure_codes(&output);
    assert!(
        codes.iter().any(|code| code == expected_code),
        "expected {expected_code}, got: {codes:?}"
    );
}

#[test]
fn oauth_missing_public_url_produces_blocking_failure() {
    assert_missing_oauth_field("YARR_MCP_PUBLIC_URL", "missing_oauth_public_url");
}

#[test]
fn oauth_missing_client_id_produces_blocking_failure() {
    assert_missing_oauth_field("YARR_MCP_GOOGLE_CLIENT_ID", "missing_oauth_client_id");
}

#[test]
fn oauth_missing_client_secret_produces_blocking_failure() {
    assert_missing_oauth_field(
        "YARR_MCP_GOOGLE_CLIENT_SECRET",
        "missing_oauth_client_secret",
    );
}

#[test]
fn oauth_missing_admin_email_produces_blocking_failure() {
    assert_missing_oauth_field("YARR_MCP_AUTH_ADMIN_EMAIL", "missing_oauth_admin_email");
}

#[test]
fn setup_repair_oauth_writes_oauth_env_lines() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("appdata");
    let output = oauth_setup_command(&data_dir)
        .args(["setup", "repair"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["exit_policy"], "success");
    assert_eq!(json["ran_repair"], true);

    let env_file = fs::read_to_string(data_dir.join(".env")).unwrap();
    for expected in [
        "YARR_MCP_AUTH_MODE=oauth",
        "YARR_MCP_PUBLIC_URL=https://mcp.yarr.test",
        "YARR_MCP_GOOGLE_CLIENT_ID=test-client-id",
        "YARR_MCP_GOOGLE_CLIENT_SECRET=test-client-secret",
        "YARR_MCP_AUTH_ADMIN_EMAIL=admin@yarr.test",
    ] {
        assert!(
            env_file.contains(expected),
            ".env should contain {expected}"
        );
    }
    assert_private_file(&data_dir.join(".env"));
}
