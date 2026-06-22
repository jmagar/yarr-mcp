use rustarr::{Command, SetupCommand, parse_args_from};
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

#[test]
fn codemode_code_flag_parsed() {
    assert_eq!(
        parse_args_from(["codemode", "--code", "async () => 1"]).unwrap(),
        Some(Command::CodeMode {
            code: "async () => 1".to_string()
        })
    );
}

#[test]
fn codemode_requires_code_or_file() {
    assert!(
        parse_args_from(["codemode"])
            .unwrap_err()
            .to_string()
            .contains("--code")
    );
}

#[test]
fn codemode_rejects_both_code_and_file() {
    let err = parse_args_from(["codemode", "--code", "x", "--file", "y"])
        .unwrap_err()
        .to_string();
    assert!(err.contains("only one"), "got: {err}");
}

#[test]
fn snippet_list_parsed() {
    assert_eq!(
        parse_args_from(["snippet", "list"]).unwrap(),
        Some(Command::SnippetList)
    );
}

#[test]
fn snippet_save_parsed() {
    assert_eq!(
        parse_args_from(["snippet", "save", "--name", "s", "--code", "async () => 1"]).unwrap(),
        Some(Command::SnippetSave {
            name: "s".to_string(),
            code: "async () => 1".to_string(),
            description: None,
        })
    );
}

#[test]
fn snippet_run_with_input_parsed() {
    assert_eq!(
        parse_args_from(["snippet", "run", "--name", "s", "--input", r#"{"a":1}"#]).unwrap(),
        Some(Command::SnippetRun {
            name: "s".to_string(),
            input: serde_json::json!({ "a": 1 }),
        })
    );
}

#[test]
fn snippet_delete_parsed() {
    assert_eq!(
        parse_args_from(["snippet", "delete", "--name", "s"]).unwrap(),
        Some(Command::SnippetDelete {
            name: "s".to_string()
        })
    );
}

#[test]
fn snippet_save_requires_name() {
    assert!(
        parse_args_from(["snippet", "save", "--code", "x"])
            .unwrap_err()
            .to_string()
            .contains("--name")
    );
}

#[test]
fn snippet_run_input_must_be_json() {
    assert!(parse_args_from(["snippet", "run", "--name", "s", "--input", "not json"]).is_err());
}
