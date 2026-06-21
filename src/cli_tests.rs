use super::{Command, SetupCommand, parse_args_from, usage};
use serde_json::json;

#[test]
fn empty_args_returns_none() {
    let result = parse_args_from::<_, String>([]).unwrap();
    assert!(result.is_none());
}

#[test]
fn unknown_token1_errors_with_inventory() {
    let err = parse_args_from(["unknown-command"]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown command"));
    assert!(msg.contains("sonarr"), "should list services");
    assert!(msg.contains("integrations"), "should list infra verbs");
}

#[test]
fn integrations_subcommand() {
    let cmd = parse_args_from(["integrations"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Integrations);
}

#[test]
fn service_status_subcommand() {
    let cmd = parse_args_from(["sonarr", "status"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Status {
            service: "sonarr".into()
        }
    );
}

#[test]
fn service_without_command_errors() {
    let err = parse_args_from(["sonarr"]).unwrap_err();
    assert!(err.to_string().contains("requires a command"));
}

#[test]
fn get_and_post_subcommands() {
    let get = parse_args_from(["radarr", "get", "--path", "/api/v3/system/status"])
        .unwrap()
        .unwrap();
    assert_eq!(
        get,
        Command::Get {
            service: "radarr".into(),
            path: "/api/v3/system/status".into()
        }
    );

    let post = parse_args_from([
        "overseerr",
        "post",
        "--path",
        "/api/v1/request",
        "--body",
        "{\"mediaId\":1}",
        "--confirm",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        post,
        Command::Post {
            service: "overseerr".into(),
            path: "/api/v1/request".into(),
            body: json!({"mediaId": 1}),
        }
    );
}

#[test]
fn put_subcommand() {
    let put = parse_args_from([
        "sonarr",
        "put",
        "--path",
        "/api/v3/series/editor",
        "--body",
        "{\"seriesIds\":[1],\"qualityProfileId\":4}",
        "--confirm",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        put,
        Command::Put {
            service: "sonarr".into(),
            path: "/api/v3/series/editor".into(),
            body: json!({"seriesIds": [1], "qualityProfileId": 4}),
        }
    );
}

#[test]
fn delete_subcommand_allows_missing_body() {
    // Uses prowlarr (Indexer capability) because the generic passthrough `delete`
    // verb is shadowed by the curated arr `delete` command for ArrManager kinds
    // (C2). For a non-arr kind the generic passthrough still owns `delete`.
    let delete = parse_args_from([
        "prowlarr",
        "delete",
        "--path",
        "/api/v1/indexer/9",
        "--confirm",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        delete,
        Command::Delete {
            service: "prowlarr".into(),
            path: "/api/v1/indexer/9".into(),
            body: None,
            confirm: true
        }
    );
}

#[test]
fn yes_is_accepted_as_confirm_alias() {
    // `--yes` is the confirm alias; it is load-bearing on the destructive `delete`
    // passthrough (prowlarr: a non-arr kind, so the generic passthrough owns
    // `delete`).
    let delete = parse_args_from(["prowlarr", "delete", "--path", "/api/v1/indexer/9", "--yes"])
        .unwrap()
        .unwrap();
    assert_eq!(
        delete,
        Command::Delete {
            service: "prowlarr".into(),
            path: "/api/v1/indexer/9".into(),
            body: None,
            confirm: true
        }
    );
}

#[test]
fn unknown_command_for_service_errors() {
    let err = parse_args_from(["sonarr", "sessions"]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown command"));
    assert!(msg.contains("sonarr"));
}

#[test]
fn help_subcommand() {
    let cmd = parse_args_from(["help"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Help);
}

#[test]
fn doctor_and_setup_subcommands() {
    assert_eq!(
        parse_args_from(["doctor", "--json"]).unwrap().unwrap(),
        Command::Doctor { json: true }
    );
    assert_eq!(
        parse_args_from(["setup", "plugin-hook", "--no-repair"])
            .unwrap()
            .unwrap(),
        Command::Setup(SetupCommand::PluginHook { no_repair: true })
    );
}

#[test]
fn usage_lists_grammar_and_services() {
    let text = usage();
    for expected in [
        "rustarr integrations",
        "rustarr <service> status",
        "rustarr <service> get --path PATH",
        "rustarr doctor",
        "sonarr",
        "Services:",
    ] {
        assert!(text.contains(expected), "usage missing {expected}");
    }
}
