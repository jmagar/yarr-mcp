use crate::cli::command::Command;
use crate::cli::parse_args_from;
use serde_json::json;

#[test]
fn sonarr_quality_profiles_parses_to_curated() {
    let cmd = parse_args_from(["sonarr", "quality-profiles"])
        .unwrap()
        .unwrap();
    // Kebab CLI verb maps to the snake_case registry/MCP action name.
    assert_eq!(
        cmd,
        Command::Curated {
            action: "quality_profiles",
            params: json!({ "service": "sonarr" }),
        }
    );
}

#[test]
fn radarr_list_parses_to_curated() {
    let cmd = parse_args_from(["radarr", "list"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "list",
            params: json!({ "service": "radarr" }),
        }
    );
}

#[test]
fn every_arr_read_verb_parses() {
    for (verb, action) in [
        ("quality-profiles", "quality_profiles"),
        ("list", "list"),
        ("wanted", "wanted"),
        ("queue", "queue"),
        ("history", "history"),
        ("rootfolders", "rootfolders"),
        ("health", "health"),
    ] {
        let cmd = parse_args_from(["sonarr", verb]).unwrap().unwrap();
        assert_eq!(
            cmd,
            Command::Curated {
                action,
                params: json!({ "service": "sonarr" }),
            },
            "verb {verb} should map to action {action}"
        );
    }
}

#[test]
fn arr_read_verbs_reject_extra_flags() {
    // C1 read verbs take only the positional service.
    let err = parse_args_from(["sonarr", "list", "--title", "X"]).unwrap_err();
    assert!(err.to_string().contains("--title") || err.to_string().contains("does not accept"));
}

#[test]
fn curated_arr_verb_rejected_on_non_arr_kind() {
    // `list` is an ArrManager verb; plex (MediaServer) must not parse it — the
    // router falls through to its "unknown command for service" error.
    let err = parse_args_from(["plex", "list"]).unwrap_err();
    assert!(
        err.to_string().contains("unknown command"),
        "plex list should be rejected at parse: {err}"
    );
}

#[test]
fn generic_verbs_still_work_alongside_curated() {
    // The curated hook returns Ok(None) for generic verbs, which fall through.
    let cmd = parse_args_from(["sonarr", "status"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Status {
            service: "sonarr".into()
        }
    );
}
