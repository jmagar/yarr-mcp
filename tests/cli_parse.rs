use rustarr::cli::{parse_args_from, Command, SetupCommand};
use serde_json::json;

#[test]
fn integrations_parsed() {
    assert_eq!(
        parse_args_from(["integrations"]).unwrap(),
        Some(Command::Integrations)
    );
}

#[test]
fn status_service_parsed() {
    assert_eq!(
        parse_args_from(["status", "--service", "sonarr"]).unwrap(),
        Some(Command::Status {
            service: "sonarr".into()
        })
    );
}

#[test]
fn get_parsed() {
    assert_eq!(
        parse_args_from([
            "get",
            "--service",
            "radarr",
            "--path",
            "/api/v3/system/status"
        ])
        .unwrap(),
        Some(Command::Get {
            service: "radarr".into(),
            path: "/api/v3/system/status".into()
        })
    );
}

#[test]
fn post_parsed() {
    assert_eq!(
        parse_args_from([
            "post",
            "--service",
            "overseerr",
            "--path",
            "/api/v1/request",
            "--body",
            "{\"mediaId\":1}"
        ])
        .unwrap(),
        Some(Command::Post {
            service: "overseerr".into(),
            path: "/api/v1/request".into(),
            body: json!({"mediaId": 1})
        })
    );
}

#[test]
fn setup_plugin_hook_no_repair_parsed() {
    assert_eq!(
        parse_args_from(["setup", "plugin-hook", "--no-repair"]).unwrap(),
        Some(Command::Setup(SetupCommand::PluginHook { no_repair: true }))
    );
}
