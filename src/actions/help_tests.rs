use super::*;
use crate::actions::all_action_names;

#[test]
fn rest_help_lists_actions_and_examples() {
    let help = rest_help();
    let actions = help.get("actions").and_then(|v| v.as_array()).unwrap();
    let names: Vec<&str> = actions.iter().filter_map(|v| v.as_str()).collect();
    assert!(names.contains(&"service_status"));
    assert!(names.contains(&"api_get"));
    assert!(names.contains(&"help"));
    assert!(help.get("examples").unwrap().get("api_delete").is_some());
}

#[test]
fn help_text_lists_every_action() {
    let text = help_text();
    for action in all_action_names() {
        assert!(
            text.contains(&format!("`{action}`")),
            "help missing action {action}"
        );
    }
}

#[test]
fn help_text_marks_write_actions() {
    let text = help_text();
    // api_post requires rustarr:write — generated help must flag it.
    let line = text
        .lines()
        .find(|l| l.contains("`api_post`"))
        .expect("api_post line present");
    assert!(
        line.contains("rustarr:write"),
        "api_post not flagged as write"
    );
}

#[test]
fn help_text_has_header_and_credentials_note() {
    let text = help_text();
    assert!(text.contains("# rustarr MCP Tool"));
    assert!(text.contains("RUSTARR_SERVICES"));
}

#[test]
fn help_text_shows_capability_digest_and_curated_commands() {
    // C1: the help renders the capability digest line and lists each curated arr
    // read command with its description and required `service` param.
    let text = help_text();
    assert!(
        text.contains("Capabilities: "),
        "help missing capability digest"
    );
    assert!(
        text.contains("arr("),
        "digest should name the arr capability"
    );
    for cmd in [
        "quality_profiles",
        "list",
        "wanted",
        "queue",
        "history",
        "rootfolders",
        "health",
    ] {
        assert!(
            text.contains(&format!("`{cmd}`")),
            "help missing curated `{cmd}`"
        );
    }
}

#[test]
fn help_text_c2_write_commands_present() {
    let text = help_text();
    for cmd in [
        "set_quality",
        "search",
        "refresh",
        "monitor",
        "unmonitor",
        "add",
        "delete",
    ] {
        assert!(text.contains(&format!("`{cmd}`")), "help missing {cmd}");
    }
    let sq = text.lines().find(|l| l.contains("`set_quality`")).unwrap();
    assert!(
        sq.contains("rustarr:write"),
        "set_quality must mark write scope: {sq}"
    );
    let enum_names = crate::actions::all_action_names();
    for cmd in ["set_quality", "delete"] {
        assert!(enum_names.contains(&cmd), "schema enum missing {cmd}");
    }
}
