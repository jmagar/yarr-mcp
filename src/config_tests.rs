//! Unit tests for src/config.rs

use super::*;

// Use the single process-wide env lock from the testing module to serialise
// all tests that mutate `RUSTARR_HOME`, `RUSTARR_SERVICES`, etc.
use crate::testing::ENV_LOCK;

// ── McpConfig::is_loopback edge cases ─────────────────────────────────────────

fn mcp_with_host(host: &str) -> McpConfig {
    McpConfig {
        host: host.to_string(),
        ..McpConfig::default()
    }
}

#[test]
fn is_loopback_ipv6_bare() {
    // "::1" without brackets — parsed as IpAddr, is_loopback() returns true
    assert!(mcp_with_host("::1").is_loopback(), "::1 should be loopback");
}

#[test]
fn is_loopback_ipv6_bracketed() {
    // "[::1]" bracket-quoted IPv6 — brackets are stripped before parse
    assert!(
        mcp_with_host("[::1]").is_loopback(),
        "[::1] should be loopback"
    );
}

#[test]
fn is_loopback_127_0_0_2() {
    // Any 127.x.x.x address is in the loopback range
    assert!(
        mcp_with_host("127.0.0.2").is_loopback(),
        "127.0.0.2 should be loopback"
    );
}

#[test]
fn is_loopback_0_0_0_0_is_false() {
    // 0.0.0.0 is unspecified, not loopback
    assert!(
        !mcp_with_host("0.0.0.0").is_loopback(),
        "0.0.0.0 should not be loopback"
    );
}

#[test]
fn is_loopback_uppercase_localhost_is_false() {
    // is_loopback only matches the literal "localhost" (case-sensitive)
    assert!(
        !mcp_with_host("LOCALHOST").is_loopback(),
        "LOCALHOST (uppercase) should not be loopback — check is case-sensitive"
    );
}

#[test]
fn is_loopback_subdomain_is_false() {
    // "localhost.rustarr.com" must not be treated as loopback
    assert!(
        !mcp_with_host("localhost.rustarr.com").is_loopback(),
        "localhost.rustarr.com should not be loopback"
    );
}

// ── env_bool helper ───────────────────────────────────────────────────────────
//
// env_bool is private, so we exercise it via a thin wrapper that sets a
// uniquely-named env var, calls the function, and unsets it again.
// Each test uses a distinct key to avoid collisions with parallel test threads.

fn call_env_bool(key: &str, raw: &str) -> anyhow::Result<bool> {
    std::env::set_var(key, raw);
    let mut target = false;
    let result = env_bool(key, &mut target);
    std::env::remove_var(key);
    result.map(|_| target)
}

#[test]
fn env_bool_accepts_1() {
    assert!(call_env_bool("TEST_ENV_BOOL_1", "1").unwrap());
}

#[test]
fn env_bool_accepts_true() {
    assert!(call_env_bool("TEST_ENV_BOOL_TRUE", "true").unwrap());
}

#[test]
fn env_bool_accepts_yes() {
    assert!(call_env_bool("TEST_ENV_BOOL_YES", "yes").unwrap());
}

#[test]
fn env_bool_accepts_0() {
    assert!(!call_env_bool("TEST_ENV_BOOL_0", "0").unwrap());
}

#[test]
fn env_bool_accepts_false() {
    assert!(!call_env_bool("TEST_ENV_BOOL_FALSE", "false").unwrap());
}

#[test]
fn env_bool_accepts_no() {
    assert!(!call_env_bool("TEST_ENV_BOOL_NO", "no").unwrap());
}

#[test]
fn env_bool_rejects_invalid() {
    let result = call_env_bool("TEST_ENV_BOOL_INVALID", "maybe");
    assert!(result.is_err(), "invalid bool string should return Err");
}

// ── env_list helper ───────────────────────────────────────────────────────────

fn call_env_list(key: &str, raw: &str) -> Vec<String> {
    std::env::set_var(key, raw);
    let mut target: Vec<String> = Vec::new();
    env_list(key, &mut target);
    std::env::remove_var(key);
    target
}

#[test]
fn env_list_splits_comma_separated() {
    let result = call_env_list("TEST_ENV_LIST_CSV", "a,b,c");
    assert_eq!(result, vec!["a", "b", "c"]);
}

#[test]
fn env_list_trims_spaces_around_commas() {
    let result = call_env_list("TEST_ENV_LIST_SPACES", "foo , bar , baz");
    assert_eq!(result, vec!["foo", "bar", "baz"]);
}

#[test]
fn env_list_empty_string_leaves_target_unchanged() {
    // An empty env var should not overwrite an existing target
    std::env::set_var("TEST_ENV_LIST_EMPTY", "");
    let mut target = vec!["existing".to_string()];
    env_list("TEST_ENV_LIST_EMPTY", &mut target);
    std::env::remove_var("TEST_ENV_LIST_EMPTY");
    assert_eq!(
        target,
        vec!["existing"],
        "empty env var should not clear target"
    );
}

// ── AuthMode serde parsing ────────────────────────────────────────────────────
//
// AuthMode parsing in Config::load() is an inline match on the env var string,
// not a standalone function. We test the serde Deserialize path instead, which
// exercises the #[serde(rename_all = "lowercase")] attribute.

#[test]
fn auth_mode_deserializes_oauth() {
    let mode: AuthMode = serde_json::from_str("\"oauth\"").expect("oauth should deserialize");
    assert_eq!(mode, AuthMode::OAuth);
}

#[test]
fn auth_mode_deserializes_bearer() {
    let mode: AuthMode = serde_json::from_str("\"bearer\"").expect("bearer should deserialize");
    assert_eq!(mode, AuthMode::Bearer);
}

#[test]
fn auth_mode_rejects_bad_value() {
    let result = serde_json::from_str::<AuthMode>("\"bad\"");
    assert!(
        result.is_err(),
        "unknown auth mode should fail to deserialize"
    );
}

#[test]
fn load_reads_dotenv_from_rustarr_home_without_overriding_process_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "RUSTARR_SERVICES=sonarr\nRUSTARR_SONARR_URL=https://sonarr.local\nRUSTARR_SONARR_API_KEY=from-file\nRUSTARR_MCP_TOKEN=from-file\n",
    )
    .unwrap();

    let old_home = std::env::var_os("RUSTARR_HOME");
    let old_services = std::env::var_os("RUSTARR_SERVICES");
    let old_url = std::env::var_os("RUSTARR_SONARR_URL");
    let old_key = std::env::var_os("RUSTARR_SONARR_API_KEY");
    let old_token = std::env::var_os("RUSTARR_MCP_TOKEN");
    std::env::set_var("RUSTARR_HOME", dir.path());
    std::env::remove_var("RUSTARR_SERVICES");
    std::env::remove_var("RUSTARR_SONARR_URL");
    std::env::set_var("RUSTARR_SONARR_API_KEY", "from-env");
    std::env::remove_var("RUSTARR_MCP_TOKEN");

    let loaded = Config::load().unwrap();

    restore_env("RUSTARR_HOME", old_home);
    restore_env("RUSTARR_SERVICES", old_services);
    restore_env("RUSTARR_SONARR_URL", old_url);
    restore_env("RUSTARR_SONARR_API_KEY", old_key);
    restore_env("RUSTARR_MCP_TOKEN", old_token);

    assert_eq!(loaded.rustarr.services.len(), 1);
    assert_eq!(loaded.rustarr.services[0].base_url, "https://sonarr.local");
    assert_eq!(
        loaded.rustarr.services[0].api_key.as_deref(),
        Some("from-env")
    );
    assert_eq!(loaded.mcp.api_token.as_deref(), Some("from-file"));
}

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}
