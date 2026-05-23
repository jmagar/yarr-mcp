use super::{parse_args_from, usage, Command, SetupCommand};

#[test]
fn empty_args_returns_none() {
    let result = parse_args_from::<_, String>([]).unwrap();
    assert!(result.is_none());
}

#[test]
fn unknown_subcommand_returns_none() {
    let result = parse_args_from(["unknown-command"]).unwrap();
    assert!(result.is_none());
}

#[test]
fn greet_no_name() {
    let cmd = parse_args_from(["greet"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Greet { name: None });
}

#[test]
fn greet_with_name_flag() {
    let cmd = parse_args_from(["greet", "--name", "Alice"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Greet {
            name: Some("Alice".into())
        }
    );
}

#[test]
fn echo_with_message_flag() {
    let cmd = parse_args_from(["echo", "--message", "hello"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Echo {
            message: "hello".into()
        }
    );
}

#[test]
fn echo_missing_message_is_error() {
    let err = parse_args_from(["echo"]).unwrap_err();
    assert!(err.to_string().contains("--message"));
}

#[test]
fn status_subcommand() {
    let cmd = parse_args_from(["status"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Status);
}

#[test]
fn help_subcommand() {
    let cmd = parse_args_from(["help"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Help);
}

#[test]
fn doctor_no_flags() {
    let cmd = parse_args_from(["doctor"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Doctor { json: false });
}

#[test]
fn doctor_json_flag() {
    let cmd = parse_args_from(["doctor", "--json"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Doctor { json: true });
}

#[test]
fn watch_defaults() {
    let cmd = parse_args_from(["watch"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Watch {
            url: None,
            interval: 10
        }
    );
}

#[test]
fn watch_with_url_and_interval() {
    let cmd = parse_args_from([
        "watch",
        "--url",
        "http://localhost:40060",
        "--interval",
        "5",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        cmd,
        Command::Watch {
            url: Some("http://localhost:40060".into()),
            interval: 5
        }
    );
}

#[test]
fn setup_check() {
    let cmd = parse_args_from(["setup", "check"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Setup(SetupCommand::Check));
}

#[test]
fn setup_repair() {
    let cmd = parse_args_from(["setup", "repair"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Setup(SetupCommand::Repair));
}

#[test]
fn setup_plugin_hook() {
    let cmd = parse_args_from(["setup", "plugin-hook"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Setup(SetupCommand::PluginHook { no_repair: false })
    );
}

#[test]
fn setup_plugin_hook_no_repair_flag() {
    let cmd = parse_args_from(["setup", "plugin-hook", "--no-repair"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Setup(SetupCommand::PluginHook { no_repair: true })
    );
}

#[test]
fn usage_mentions_current_cli_commands_and_loopback_default() {
    let text = usage();
    for expected in [
        "example help",
        "example doctor",
        "example setup plugin-hook",
        "example watch",
        "default 127.0.0.1",
    ] {
        assert!(text.contains(expected), "usage missing {expected}");
    }
}

#[test]
fn parser_rejects_unknown_and_malformed_flags() {
    for args in [
        &["status", "--bogus"][..],
        &["help", "--bogus"],
        &["greet", "--bogus"],
        &["greet", "--name"],
        &["greet", "--name", "--bogus"],
        &["greet", "--name", "Alice", "extra"],
        &["doctor", "--bogus"],
        &["doctor", "--json", "--json"],
        &["watch", "--url", "http://localhost:40060", "--bogus"],
        &["watch", "--interval", "0"],
        &["setup", "check", "--no-repair"],
        &["setup", "plugin-hook", "--no-reapir"],
    ] {
        assert!(
            parse_args_from(args.iter().copied()).is_err(),
            "{args:?} should be rejected"
        );
    }
}

#[test]
fn parser_reports_duplicate_value_flags() {
    let err = parse_args_from(["greet", "--name", "Alice", "--name", "Bob"]).unwrap_err();
    assert!(err.to_string().contains("duplicate --name"));
}
