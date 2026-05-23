use std::sync::Mutex;

use super::{SetupCommand, SetupReport};
use crate::config::{Config, McpConfig, RustarrConfig};

static ENV_LOCK: Mutex<()> = Mutex::new(());

// ── SetupReport state machine ─────────────────────────────────────────────────

#[test]
fn new_report_has_no_failures() {
    let report = SetupReport::new(false);
    assert!(report.blocking_failures.is_empty());
    assert!(report.advisory_failures.is_empty());
    assert!(!report.ran_repair);
}

#[test]
fn finish_sets_success_when_no_failures() {
    let report = SetupReport::new(false).finish();
    assert_eq!(report.exit_policy, "success");
}

#[test]
fn finish_sets_blocking_failure_when_blocking_present() {
    let mut report = SetupReport::new(false);
    report.blocking_failures.push(super::SetupFailure {
        code: "test_code",
        message: "test message".into(),
    });
    let report = report.finish();
    assert_eq!(report.exit_policy, "blocking_failure");
}

#[test]
fn finish_sets_advisory_failure_when_only_advisory_present() {
    let mut report = SetupReport::new(false);
    report.advisory_failures.push(super::SetupFailure {
        code: "test_advisory",
        message: "advisory message".into(),
    });
    let report = report.finish();
    assert_eq!(report.exit_policy, "advisory_failure");
}

#[test]
fn finish_prefers_blocking_over_advisory() {
    let mut report = SetupReport::new(false);
    report.blocking_failures.push(super::SetupFailure {
        code: "b",
        message: "blocking".into(),
    });
    report.advisory_failures.push(super::SetupFailure {
        code: "a",
        message: "advisory".into(),
    });
    let report = report.finish();
    assert_eq!(report.exit_policy, "blocking_failure");
}

// ── SetupCommand enum ─────────────────────────────────────────────────────────

#[test]
fn setup_command_copy() {
    let cmd = SetupCommand::Check;
    let _copy = cmd;
    let _again = cmd;
}

#[test]
fn all_variants_are_distinct() {
    assert_ne!(SetupCommand::Check, SetupCommand::Repair);
    assert_ne!(
        SetupCommand::Check,
        SetupCommand::PluginHook { no_repair: false }
    );
    assert_ne!(
        SetupCommand::Repair,
        SetupCommand::PluginHook { no_repair: false }
    );
    assert_ne!(
        SetupCommand::PluginHook { no_repair: false },
        SetupCommand::PluginHook { no_repair: true }
    );
}

// ── setup check / repair behavior ────────────────────────────────────────────

fn valid_config() -> Config {
    Config {
        rustarr: RustarrConfig {
            services: vec![crate::config::ServiceConfig {
                name: "sonarr".into(),
                kind: crate::config::ServiceKind::Sonarr,
                base_url: "https://rustarr.test/api".into(),
                api_key: Some("secret with spaces".into()),
                ..crate::config::ServiceConfig::default()
            }],
        },
        mcp: McpConfig {
            host: "127.0.0.1".into(),
            no_auth: true,
            ..McpConfig::default()
        },
    }
}

fn with_plugin_data<T>(dir: &std::path::Path, f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.lock().unwrap();
    struct EnvRestore {
        old: Option<std::ffi::OsString>,
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            unsafe {
                match self.old.take() {
                    Some(value) => std::env::set_var("CLAUDE_PLUGIN_DATA", value),
                    None => std::env::remove_var("CLAUDE_PLUGIN_DATA"),
                }
            }
        }
    }

    let _restore = EnvRestore {
        old: std::env::var_os("CLAUDE_PLUGIN_DATA"),
    };
    unsafe {
        std::env::set_var("CLAUDE_PLUGIN_DATA", dir);
    }
    f()
}

#[test]
fn setup_check_reports_missing_env_as_advisory() {
    let dir = tempfile::tempdir().unwrap();
    let config = valid_config();

    let report = with_plugin_data(dir.path(), || super::setup_check(&config, true));

    assert!(report.blocking_failures.is_empty());
    assert_eq!(report.exit_policy, "advisory_failure");
    assert!(report
        .advisory_failures
        .iter()
        .any(|failure| failure.code == "env_file_missing"));
}

#[test]
fn setup_repair_creates_env_file() {
    let dir = tempfile::tempdir().unwrap();
    let config = valid_config();

    let report = with_plugin_data(dir.path(), || super::setup_repair(&config).unwrap());

    assert!(report.ran_repair);
    assert!(report.blocking_failures.is_empty());
    let env_path = dir.path().join(".env");
    let contents = std::fs::read_to_string(&env_path).unwrap();
    assert!(contents.contains("RUSTARR_SERVICES=sonarr"));
    assert!(contents.contains("RUSTARR_SONARR_URL=https://rustarr.test/api"));
    assert!(contents.contains("RUSTARR_SONARR_API_KEY=\"secret with spaces\""));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&env_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[test]
fn dotenv_values_quote_special_characters_and_escape_quotes() {
    assert_eq!(
        super::dotenv_assignment("RUSTARR_API_KEY", "secret # \"quoted\"").unwrap(),
        "RUSTARR_API_KEY=\"secret # \\\"quoted\\\"\""
    );
}

#[test]
fn dotenv_values_reject_newlines() {
    let error = super::dotenv_assignment("RUSTARR_API_KEY", "line\nbreak").unwrap_err();
    assert!(error.to_string().contains("newlines"));
}
