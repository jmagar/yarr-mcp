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
}
