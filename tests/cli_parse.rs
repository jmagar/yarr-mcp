use rustarr::cli::{Command, SetupCommand, parse_args_from};
use serde_json::json;

#[test]
fn integrations_parsed() {
    assert_eq!(
        parse_args_from(["integrations"]).unwrap(),
        Some(Command::Integrations)
    );
}

#[test]
fn service_status_parsed() {
    assert_eq!(
        parse_args_from(["sonarr", "status"]).unwrap(),
        Some(Command::Status {
            service: "sonarr".into()
        })
    );
}

#[test]
fn service_get_parsed() {
    assert_eq!(
        parse_args_from(["radarr", "get", "--path", "/api/v3/system/status"]).unwrap(),
        Some(Command::Get {
            service: "radarr".into(),
            path: "/api/v3/system/status".into()
        })
    );
}

#[test]
fn service_post_parsed() {
    assert_eq!(
        parse_args_from([
            "overseerr",
            "post",
            "--path",
            "/api/v1/request",
            "--body",
            "{\"mediaId\":1}",
            "--confirm"
        ])
        .unwrap(),
        Some(Command::Post {
            service: "overseerr".into(),
            path: "/api/v1/request".into(),
            body: json!({"mediaId": 1}),
            confirm: true
        })
    );
}

#[test]
fn unknown_service_errors() {
    let err = parse_args_from(["bogus", "status"]).unwrap_err();
    assert!(err.to_string().contains("unknown command"));
}

#[test]
fn setup_plugin_hook_no_repair_parsed() {
    assert_eq!(
        parse_args_from(["setup", "plugin-hook", "--no-repair"]).unwrap(),
        Some(Command::Setup(SetupCommand::PluginHook { no_repair: true }))
    );
}
