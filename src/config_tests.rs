//! Unit tests for configuration types and loading.

use super::*;
use crate::testing::TestEnv;

fn mcp_with_host(host: &str) -> McpConfig {
    McpConfig {
        host: host.to_owned(),
        ..McpConfig::default()
    }
}

#[test]
fn test_env_guard_restores_values_when_dropped() {
    const KEY: &str = "YARR_TEST_ENV_GUARD_RESTORE";
    let original = std::env::var_os(KEY);
    {
        let mut env = TestEnv::new();
        env.set(KEY, "changed");
        assert_eq!(std::env::var(KEY).as_deref(), Ok("changed"));
    }
    assert_eq!(std::env::var_os(KEY), original);
}

#[test]
fn loopback_host_detection_handles_ip_and_hostname_edges() {
    for host in ["::1", "[::1]", "127.0.0.2"] {
        assert!(
            mcp_with_host(host).is_loopback(),
            "{host} should be loopback"
        );
    }
    for host in ["0.0.0.0", "LOCALHOST", "localhost.yarr.com"] {
        assert!(
            !mcp_with_host(host).is_loopback(),
            "{host} must not be loopback"
        );
    }
}

#[test]
fn auth_mode_serde_accepts_documented_values_and_rejects_unknown_values() {
    assert_eq!(
        serde_json::from_str::<AuthMode>("\"oauth\"").unwrap(),
        AuthMode::OAuth
    );
    assert_eq!(
        serde_json::from_str::<AuthMode>("\"bearer\"").unwrap(),
        AuthMode::Bearer
    );
    assert!(serde_json::from_str::<AuthMode>("\"bad\"").is_err());
}
