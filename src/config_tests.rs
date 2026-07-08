//! Unit tests for src/config.rs

use super::*;

// Use the single process-wide env lock from the testing module to serialise
// all tests that mutate `YARR_HOME`, `YARR_SERVICES`, etc.
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
    // "localhost.yarr.com" must not be treated as loopback
    assert!(
        !mcp_with_host("localhost.yarr.com").is_loopback(),
        "localhost.yarr.com should not be loopback"
    );
}

// ── env_bool helper ───────────────────────────────────────────────────────────
//
// env_bool is private, so we exercise it via a thin wrapper that sets a
// uniquely-named env var, calls the function, and unsets it again.
// Each test uses a distinct key to avoid collisions with parallel test threads.

fn call_env_bool(key: &str, raw: &str) -> anyhow::Result<bool> {
    // SAFETY: each test uses a uniquely-named key, so no other thread reads or
    // writes this env var concurrently.
    unsafe {
        std::env::set_var(key, raw);
    }
    let mut target = false;
    let result = env_bool(key, &mut target);
    unsafe {
        std::env::remove_var(key);
    }
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
    // SAFETY: each test uses a uniquely-named key, so no other thread reads or
    // writes this env var concurrently.
    unsafe {
        std::env::set_var(key, raw);
    }
    let mut target: Vec<String> = Vec::new();
    env_list(key, &mut target);
    unsafe {
        std::env::remove_var(key);
    }
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
    // SAFETY: this test owns the uniquely-named TEST_ENV_LIST_EMPTY key.
    unsafe {
        std::env::set_var("TEST_ENV_LIST_EMPTY", "");
    }
    let mut target = vec!["existing".to_string()];
    env_list("TEST_ENV_LIST_EMPTY", &mut target);
    unsafe {
        std::env::remove_var("TEST_ENV_LIST_EMPTY");
    }
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
fn load_reads_dotenv_from_yarr_home_without_overriding_process_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "YARR_SERVICES=sonarr\nYARR_SONARR_URL=https://sonarr.local\nYARR_SONARR_API_KEY=from-file\nYARR_MCP_TOKEN=from-file\n",
    )
    .unwrap();

    let old_home = std::env::var_os("YARR_HOME");
    let old_services = std::env::var_os("YARR_SERVICES");
    let old_url = std::env::var_os("YARR_SONARR_URL");
    let old_key = std::env::var_os("YARR_SONARR_API_KEY");
    let old_token = std::env::var_os("YARR_MCP_TOKEN");
    // SAFETY: `_guard` holds the process-wide ENV_LOCK, so no other test mutates
    // or reads these shared keys concurrently.
    unsafe {
        std::env::set_var("YARR_HOME", dir.path());
        std::env::remove_var("YARR_SERVICES");
        std::env::remove_var("YARR_SONARR_URL");
        std::env::set_var("YARR_SONARR_API_KEY", "from-env");
        std::env::remove_var("YARR_MCP_TOKEN");
    }

    let loaded = Config::load().unwrap();

    restore_env("YARR_HOME", old_home);
    restore_env("YARR_SERVICES", old_services);
    restore_env("YARR_SONARR_URL", old_url);
    restore_env("YARR_SONARR_API_KEY", old_key);
    restore_env("YARR_MCP_TOKEN", old_token);

    assert_eq!(loaded.yarr.services.len(), 1);
    assert_eq!(loaded.yarr.services[0].base_url, "https://sonarr.local");
    assert_eq!(loaded.yarr.services[0].api_key.as_deref(), Some("from-env"));
    assert_eq!(loaded.mcp.api_token.as_deref(), Some("from-file"));
}

#[test]
fn load_falls_back_to_legacy_rustarr_config_when_yarr_config_is_absent() {
    let _guard = ENV_LOCK.lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    let legacy_dir = home.path().join(".rustarr");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    std::fs::write(
        legacy_dir.join("config.toml"),
        "[mcp]\nport = 40123\nserver_name = \"legacy-yarr\"\n\n[[rustarr.services]]\nname = \"sonarr\"\nkind = \"sonarr\"\nbase_url = \"https://sonarr.legacy\"\napi_key = \"legacy-key\"\n",
    )
    .unwrap();

    let keys = ["HOME", "YARR_HOME", "YARR_CONFIG", "YARR_MCP_PORT"];
    let old = keys
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect::<Vec<_>>();
    unsafe {
        std::env::set_var("HOME", home.path());
        std::env::remove_var("YARR_HOME");
        std::env::remove_var("YARR_CONFIG");
        std::env::remove_var("YARR_MCP_PORT");
    }

    let loaded = Config::load().unwrap();

    for (key, value) in old {
        restore_env(key, value);
    }

    assert_eq!(loaded.mcp.port, 40123);
    assert_eq!(loaded.mcp.server_name, "legacy-yarr");
    assert_eq!(loaded.yarr.services.len(), 1);
    assert_eq!(loaded.yarr.services[0].name, "sonarr");
    assert_eq!(loaded.yarr.services[0].base_url, "https://sonarr.legacy");
    assert_eq!(
        loaded.yarr.services[0].api_key.as_deref(),
        Some("legacy-key")
    );
}

#[test]
fn load_rejects_conflicting_legacy_rustarr_and_yarr_config_sections() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        "[rustarr]\nservices = []\n\n[yarr]\nservices = []\n",
    )
    .unwrap();

    let keys = ["YARR_CONFIG", "YARR_HOME"];
    let old = keys
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect::<Vec<_>>();
    unsafe {
        std::env::set_var("YARR_CONFIG", &config_path);
        std::env::remove_var("YARR_HOME");
    }

    let result = Config::load();

    for (key, value) in old {
        restore_env(key, value);
    }

    assert!(
        result.is_err(),
        "mixed [rustarr] and [yarr] config sections must not silently merge"
    );
}

#[test]
fn load_accepts_yarr_service_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let keys = [
        "YARR_HOME",
        "YARR_SERVICES",
        "YARR_SONARR_KIND",
        "YARR_SONARR_URL",
        "YARR_SONARR_API_KEY",
    ];
    let old = keys
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect::<Vec<_>>();
    unsafe {
        std::env::set_var("YARR_HOME", dir.path());
        std::env::set_var("YARR_SERVICES", "sonarr");
        std::env::set_var("YARR_SONARR_KIND", "sonarr");
        std::env::set_var("YARR_SONARR_URL", "https://api.yarr.test");
        std::env::set_var("YARR_SONARR_API_KEY", "secret");
    }

    let loaded = Config::load().unwrap();

    for (key, value) in old {
        restore_env(key, value);
    }

    assert_eq!(loaded.yarr.services.len(), 1);
    assert_eq!(loaded.yarr.services[0].name, "sonarr");
    assert_eq!(loaded.yarr.services[0].base_url, "https://api.yarr.test");
    assert_eq!(loaded.yarr.services[0].api_key.as_deref(), Some("secret"));
}

#[test]
fn load_migrates_legacy_rustarr_env_namespace_when_new_keys_are_absent() {
    let _guard = ENV_LOCK.lock().unwrap();
    let keys = [
        "YARR_HOME",
        "RUSTARR_SERVICES",
        "RUSTARR_SONARR_KIND",
        "RUSTARR_SONARR_URL",
        "RUSTARR_SONARR_API_KEY",
        "YARR_SERVICES",
        "YARR_SONARR_KIND",
        "YARR_SONARR_URL",
        "YARR_SONARR_API_KEY",
    ];
    let old = keys
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect::<Vec<_>>();
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("YARR_HOME", dir.path());
        std::env::set_var("RUSTARR_SERVICES", "sonarr");
        std::env::set_var("RUSTARR_SONARR_KIND", "sonarr");
        std::env::set_var("RUSTARR_SONARR_URL", "https://legacy.yarr.test");
        std::env::set_var("RUSTARR_SONARR_API_KEY", "legacy-secret");
        std::env::remove_var("YARR_SERVICES");
        std::env::remove_var("YARR_SONARR_KIND");
        std::env::remove_var("YARR_SONARR_URL");
        std::env::remove_var("YARR_SONARR_API_KEY");
    }

    let loaded = Config::load().unwrap();

    for (key, value) in old {
        restore_env(key, value);
    }

    assert_eq!(loaded.yarr.services.len(), 1);
    assert_eq!(loaded.yarr.services[0].base_url, "https://legacy.yarr.test");
    assert_eq!(
        loaded.yarr.services[0].api_key.as_deref(),
        Some("legacy-secret")
    );
}

#[test]
fn load_rejects_conflicting_legacy_and_yarr_env_values() {
    let _guard = ENV_LOCK.lock().unwrap();
    let keys = ["RUSTARR_SERVICES", "YARR_SERVICES"];
    let old = keys
        .iter()
        .map(|key| (*key, std::env::var_os(key)))
        .collect::<Vec<_>>();
    unsafe {
        std::env::set_var("RUSTARR_SERVICES", "sonarr");
        std::env::set_var("YARR_SERVICES", "radarr");
    }

    let result = Config::load();

    for (key, value) in old {
        restore_env(key, value);
    }

    let error = result.expect_err("conflicting legacy and new env vars must fail");
    assert!(
        error.to_string().contains("conflicting legacy env"),
        "unexpected error: {error:#}"
    );
}

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    // SAFETY: callers invoke this only while holding the process-wide ENV_LOCK,
    // so there is no concurrent env access to these shared keys.
    unsafe {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
}

// ── .env key-injection allowlist (security hardening) ─────────────────────────

#[test]
fn injectable_env_key_allows_yarr_namespace_and_rust_log() {
    assert!(is_injectable_env_key("YARR_SERVICES"));
    assert!(is_injectable_env_key("YARR_SONARR_API_KEY"));
    assert!(is_injectable_env_key("RUSTARR_SERVICES"));
    assert!(is_injectable_env_key("RUST_LOG"));
}

#[test]
fn injectable_env_key_rejects_dangerous_process_vars() {
    for key in [
        "PATH",
        "LD_PRELOAD",
        "SSL_CERT_FILE",
        "HOME",
        "RUST_BACKTRACE",
    ] {
        assert!(
            !is_injectable_env_key(key),
            "{key} must not be injectable from .env"
        );
    }
}

#[test]
fn load_dotenv_skips_non_yarr_keys_but_injects_allowed_ones() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "ZZINJECT_REVIEW_TEST=danger\nYARR_REVIEW_INJECT_OK=safe\n",
    )
    .unwrap();

    let old_home = std::env::var_os("YARR_HOME");
    let old_danger = std::env::var_os("ZZINJECT_REVIEW_TEST");
    let old_ok = std::env::var_os("YARR_REVIEW_INJECT_OK");
    // SAFETY: `_guard` holds the process-wide ENV_LOCK, so no other test mutates
    // or reads these keys concurrently. Clear the two test keys first so the
    // `var_os` already-set guard does not mask the allowlist behaviour.
    unsafe {
        std::env::set_var("YARR_HOME", dir.path());
        std::env::remove_var("ZZINJECT_REVIEW_TEST");
        std::env::remove_var("YARR_REVIEW_INJECT_OK");
    }

    Config::load().unwrap();

    let danger_after = std::env::var_os("ZZINJECT_REVIEW_TEST");
    let ok_after = std::env::var("YARR_REVIEW_INJECT_OK").ok();

    restore_env("YARR_HOME", old_home);
    restore_env("ZZINJECT_REVIEW_TEST", old_danger);
    restore_env("YARR_REVIEW_INJECT_OK", old_ok);

    assert!(
        danger_after.is_none(),
        "non-RUSTARR key must not be injected from .env"
    );
    assert_eq!(
        ok_after.as_deref(),
        Some("safe"),
        "YARR_* key should still be injected"
    );
}

#[test]
fn load_dotenv_ignores_legacy_and_dangerous_keys() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "YARR_SERVICES=sonarr\nRUSTARR_NOAUTH=true\nPATH=/tmp/evil\nLD_PRELOAD=/tmp/evil.so\nRUST_LOG=debug\n",
    )
    .unwrap();

    let old_home = std::env::var_os("YARR_HOME");
    let old_services = std::env::var_os("YARR_SERVICES");
    let old_legacy = std::env::var_os("RUSTARR_NOAUTH");
    let old_yarr_noauth = std::env::var_os("YARR_NOAUTH");
    let old_ld_preload = std::env::var_os("LD_PRELOAD");
    let old_rust_log = std::env::var_os("RUST_LOG");
    unsafe {
        std::env::set_var("YARR_HOME", dir.path());
        std::env::remove_var("YARR_SERVICES");
        std::env::remove_var("RUSTARR_NOAUTH");
        std::env::remove_var("YARR_NOAUTH");
        std::env::remove_var("LD_PRELOAD");
        std::env::remove_var("RUST_LOG");
    }

    let result = load_dotenv_defaults();

    let services = std::env::var("YARR_SERVICES").ok();
    let legacy = std::env::var_os("RUSTARR_NOAUTH");
    let yarr_noauth = std::env::var("YARR_NOAUTH").ok();
    let ld_preload = std::env::var_os("LD_PRELOAD");
    let rust_log = std::env::var("RUST_LOG").ok();
    restore_env("YARR_HOME", old_home);
    restore_env("YARR_SERVICES", old_services);
    restore_env("RUSTARR_NOAUTH", old_legacy);
    restore_env("YARR_NOAUTH", old_yarr_noauth);
    restore_env("LD_PRELOAD", old_ld_preload);
    restore_env("RUST_LOG", old_rust_log);

    result.unwrap();
    assert_eq!(services.as_deref(), Some("sonarr"));
    assert!(legacy.is_none());
    assert_eq!(yarr_noauth.as_deref(), Some("true"));
    assert!(ld_preload.is_none());
    assert_eq!(rust_log.as_deref(), Some("debug"));
}

#[test]
fn load_dotenv_rejects_conflicting_legacy_and_yarr_keys() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".env"),
        "YARR_NOAUTH=false\nRUSTARR_NOAUTH=true\n",
    )
    .unwrap();

    let old_home = std::env::var_os("YARR_HOME");
    let old_noauth = std::env::var_os("YARR_NOAUTH");
    unsafe {
        std::env::set_var("YARR_HOME", dir.path());
        std::env::remove_var("YARR_NOAUTH");
    }

    let result = load_dotenv_defaults();

    restore_env("YARR_HOME", old_home);
    restore_env("YARR_NOAUTH", old_noauth);

    let error = result.expect_err("mixed legacy and yarr .env keys must conflict");
    assert!(
        error
            .to_string()
            .contains("conflicting values for YARR_NOAUTH"),
        "unexpected error: {error:#}"
    );
}

#[cfg(unix)]
#[test]
fn legacy_dotenv_migration_writes_private_env_file() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = ENV_LOCK.lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    let data_dir = home.path().join(".yarr");
    let legacy_dir = home.path().join(".rustarr");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    std::fs::write(
        legacy_dir.join(".env"),
        "RUSTARR_SERVICES=sonarr\nRUSTARR_SONARR_API_KEY=secret\n",
    )
    .unwrap();

    let old_home = std::env::var_os("HOME");
    unsafe {
        std::env::set_var("HOME", home.path());
    }

    migrate_legacy_dotenv(&data_dir).unwrap();

    restore_env("HOME", old_home);

    let metadata = std::fs::metadata(data_dir.join(".env")).unwrap();
    assert_eq!(
        metadata.permissions().mode() & 0o777,
        0o600,
        "migrated .env must not expose service credentials"
    );
    let migrated = std::fs::read_to_string(data_dir.join(".env")).unwrap();
    assert!(migrated.contains("YARR_SERVICES=sonarr"));
    assert!(migrated.contains("YARR_SONARR_API_KEY=secret"));
}

#[test]
fn load_dotenv_rejects_null_byte_in_value() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".env"),
        b"YARR_REVIEW_NULL_TEST=ab\0cd\n".as_slice(),
    )
    .unwrap();

    let old_home = std::env::var_os("YARR_HOME");
    let old_val = std::env::var_os("YARR_REVIEW_NULL_TEST");
    // SAFETY: `_guard` holds the process-wide ENV_LOCK.
    unsafe {
        std::env::set_var("YARR_HOME", dir.path());
        std::env::remove_var("YARR_REVIEW_NULL_TEST");
    }

    let result = Config::load();

    restore_env("YARR_HOME", old_home);
    restore_env("YARR_REVIEW_NULL_TEST", old_val);

    assert!(
        result.is_err(),
        "a null byte in a .env value must be rejected, not passed to set_var"
    );
}
