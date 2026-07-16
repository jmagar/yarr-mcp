//! Tests for [`AuthConfig`] defaults and the [`AuthMode`] default.

use super::*;

#[test]
fn auth_mode_defaults_to_bearer() {
    assert_eq!(AuthMode::default(), AuthMode::Bearer);
}

#[test]
fn auth_config_defaults() {
    let cfg = AuthConfig::default();
    assert_eq!(cfg.mode, AuthMode::Bearer);
    assert!(cfg.public_url.is_none());
    assert!(cfg.google_client_id.is_none());
    assert!(cfg.admin_email.is_empty());
    assert!(cfg.allowed_emails.is_empty());
    assert_eq!(cfg.sqlite_path, "/data/auth.db");
    assert_eq!(cfg.key_path, "/data/auth-jwt.pem");
    assert_eq!(cfg.access_token_ttl_secs, 3600);
    assert_eq!(cfg.refresh_token_ttl_secs, 86400 * 30);
    assert_eq!(cfg.auth_code_ttl_secs, 300);
    assert_eq!(cfg.register_rpm, 10);
    assert_eq!(cfg.authorize_rpm, 60);
    assert!(!cfg.disable_static_token_with_oauth);
}

#[test]
fn documented_oauth_static_token_control_parses() {
    let cfg: crate::config::Config = toml::from_str(
        r#"
        [mcp.auth]
        mode = "oauth"
        public_url = "https://yarr.example.com"
        disable_static_token_with_oauth = true
        "#,
    )
    .expect("documented auth block should parse");

    assert_eq!(cfg.mcp.auth.mode, AuthMode::OAuth);
    assert!(cfg.mcp.auth.disable_static_token_with_oauth);
}

#[test]
fn unknown_auth_keys_are_rejected() {
    let error = toml::from_str::<crate::config::Config>(
        r#"
        [mcp.auth]
        mode = "oauth"
        disable_static_token_with_oath = true
        "#,
    )
    .expect_err("misspelled auth controls must fail closed");

    assert!(
        error.to_string().contains("unknown field"),
        "unexpected parse error: {error}"
    );
}

#[test]
fn oauth_local_state_allows_only_one_replica() {
    let dir = tempfile::tempdir().unwrap();
    let sqlite = dir.path().join("auth.db");
    let first = acquire_oauth_instance_lock(&sqlite).unwrap();
    let error = acquire_oauth_instance_lock(&sqlite)
        .expect_err("a second local OAuth replica must fail closed");
    assert!(error.to_string().contains("exactly one replica"), "{error}");
    drop(first);
    acquire_oauth_instance_lock(&sqlite).expect("lock must be released on clean shutdown");
}
