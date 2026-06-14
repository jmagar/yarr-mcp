use super::{parse_args_from, usage, Command, SetupCommand};
use serde_json::json;

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
fn integrations_subcommand() {
    let cmd = parse_args_from(["integrations"]).unwrap().unwrap();
    assert_eq!(cmd, Command::Integrations);
}

#[test]
fn status_subcommand_requires_service() {
    let err = parse_args_from(["status"]).unwrap_err();
    assert!(err.to_string().contains("--service"));
    let cmd = parse_args_from(["status", "--service", "sonarr"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Status {
            service: "sonarr".into()
        }
    );
}

#[test]
fn get_and_post_subcommands() {
    let get = parse_args_from([
        "get",
        "--service",
        "radarr",
        "--path",
        "/api/v3/system/status",
    ])
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
        "post",
        "--service",
        "overseerr",
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
            confirm: true
        }
    );
}

#[test]
fn put_subcommand() {
    let put = parse_args_from([
        "put",
        "--service",
        "sonarr",
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
            confirm: true
        }
    );
}

#[test]
fn delete_subcommand_allows_missing_body() {
    let delete = parse_args_from([
        "delete",
        "--service",
        "sonarr",
        "--path",
        "/api/v3/series/9?deleteFiles=false",
        "--confirm",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        delete,
        Command::Delete {
            service: "sonarr".into(),
            path: "/api/v3/series/9?deleteFiles=false".into(),
            body: None,
            confirm: true
        }
    );
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
fn usage_mentions_current_cli_commands_and_loopback_default() {
    let text = usage();
    for expected in [
        "rustarr integrations",
        "rustarr status --service NAME",
        "rustarr get --service NAME --path PATH",
        "rustarr doctor",
        "default 127.0.0.1",
    ] {
        assert!(text.contains(expected), "usage missing {expected}");
    }
}
