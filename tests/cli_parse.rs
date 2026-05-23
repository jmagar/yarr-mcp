//! Integration tests for CLI argument parsing.
//!
//! **Template**: extend these tests when you add new CLI subcommands.

use rmcp_template::cli::{parse_args_from, Command, SetupCommand};

#[test]
fn test_greet_no_name_parsed() {
    assert_eq!(
        parse_args_from(["greet"]).unwrap(),
        Some(Command::Greet { name: None })
    );
}

#[test]
fn test_greet_with_name_parsed() {
    assert_eq!(
        parse_args_from(["greet", "--name", "Alice"]).unwrap(),
        Some(Command::Greet {
            name: Some("Alice".into())
        })
    );
}

#[test]
fn test_greet_rejects_flag_like_name_value() {
    let error = parse_args_from(["greet", "--name", "--bogus"]).unwrap_err();
    assert!(error.to_string().contains("requires a value after --name"));
}

#[test]
fn test_echo_message_parsed() {
    assert_eq!(
        parse_args_from(["echo", "--message", "Hello, World!"]).unwrap(),
        Some(Command::Echo {
            message: "Hello, World!".into()
        })
    );
}

#[test]
fn test_echo_no_message_is_rejected() {
    let error = parse_args_from(["echo"]).unwrap_err();
    assert!(error.to_string().contains("requires non-empty --message"));
}

#[test]
fn test_help_parsed() {
    assert_eq!(parse_args_from(["help"]).unwrap(), Some(Command::Help));
}

#[test]
fn test_watch_bad_interval_is_rejected() {
    let error = parse_args_from(["watch", "--interval", "nope"]).unwrap_err();
    assert!(error.to_string().contains("--interval"));
}

#[test]
fn test_setup_plugin_hook_no_repair_parsed() {
    assert_eq!(
        parse_args_from(["setup", "plugin-hook", "--no-repair"]).unwrap(),
        Some(Command::Setup(SetupCommand::PluginHook { no_repair: true }))
    );
}

#[test]
fn test_setup_check_parsed() {
    assert_eq!(
        parse_args_from(["setup", "check"]).unwrap(),
        Some(Command::Setup(SetupCommand::Check))
    );
}

#[test]
fn test_setup_repair_parsed() {
    assert_eq!(
        parse_args_from(["setup", "repair"]).unwrap(),
        Some(Command::Setup(SetupCommand::Repair))
    );
}

#[test]
fn test_setup_plugin_hook_default_parsed() {
    assert_eq!(
        parse_args_from(["setup", "plugin-hook"]).unwrap(),
        Some(Command::Setup(SetupCommand::PluginHook {
            no_repair: false
        }))
    );
}

#[test]
fn test_doctor_json_parsed() {
    assert_eq!(
        parse_args_from(["doctor", "--json"]).unwrap(),
        Some(Command::Doctor { json: true })
    );
}

#[test]
fn test_doctor_no_json_parsed() {
    assert_eq!(
        parse_args_from(["doctor"]).unwrap(),
        Some(Command::Doctor { json: false })
    );
}

#[test]
fn test_unknown_trailing_args_are_rejected() {
    for args in [
        &["status", "--bogus"][..],
        &["help", "--bogus"],
        &["greet", "--unknown"],
        &["echo", "--message", "hello", "--extra"],
        &["doctor", "--json", "--json"],
        &["watch", "--interval", "0"],
        &["setup", "plugin-hook", "--no-reapir"],
    ] {
        assert!(
            parse_args_from(args.iter().copied()).is_err(),
            "{args:?} should be rejected"
        );
    }
}
