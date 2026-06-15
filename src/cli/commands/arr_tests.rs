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

// ── C2 write verbs ───────────────────────────────────────────────────────────────

#[test]
fn set_quality_parses_from_to() {
    let cmd = parse_args_from([
        "sonarr",
        "set-quality",
        "--from",
        "Ultra-HD",
        "--to",
        "HD-1080p",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "set_quality",
            params: json!({ "service": "sonarr", "from": "Ultra-HD", "to": "HD-1080p" }),
        }
    );
}

#[test]
fn set_quality_confirm_and_yes_alias_set_confirm() {
    for flag in ["--confirm", "--yes"] {
        let cmd = parse_args_from(["radarr", "set-quality", "--to", "HD-1080p", flag])
            .unwrap()
            .unwrap();
        let Command::Curated { params, .. } = cmd else {
            panic!("expected curated");
        };
        assert_eq!(params["confirm"], json!(true), "{flag} should set confirm");
    }
}

#[test]
fn set_quality_repeatable_title_selectors_collect_into_array() {
    let cmd = parse_args_from([
        "sonarr",
        "set-quality",
        "--to",
        "HD-1080p",
        "--title",
        "Alpha",
        "--title",
        "Beta",
    ])
    .unwrap()
    .unwrap();
    let Command::Curated { params, .. } = cmd else {
        panic!("expected curated");
    };
    assert_eq!(params["title"], json!(["Alpha", "Beta"]));
}

#[test]
fn set_quality_bulk_flag_sets_override() {
    let cmd = parse_args_from(["sonarr", "set-quality", "--to", "HD-1080p", "--bulk"])
        .unwrap()
        .unwrap();
    let Command::Curated { params, .. } = cmd else {
        panic!("expected curated");
    };
    assert_eq!(params["bulk"], json!(true));
}

#[test]
fn delete_parses_id_and_delete_files_opt_in() {
    let cmd = parse_args_from([
        "radarr",
        "delete",
        "--id",
        "9",
        "--delete-files",
        "--confirm",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "delete",
            params: json!({
                "service": "radarr",
                "ids": ["9"],
                "delete_files": true,
                "confirm": true,
            }),
        }
    );
}

#[test]
fn search_parses_with_no_selector() {
    let cmd = parse_args_from(["sonarr", "search", "--confirm"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "search",
            params: json!({ "service": "sonarr", "confirm": true }),
        }
    );
}

#[test]
fn write_verb_rejected_on_non_arr_kind() {
    // set-quality is an ArrManager verb; plex must not parse it.
    let err = parse_args_from(["plex", "set-quality", "--to", "HD-1080p"]).unwrap_err();
    assert!(
        err.to_string().contains("unknown command"),
        "plex set-quality should be rejected: {err}"
    );
}

#[test]
fn write_verb_rejects_unknown_flag() {
    let err = parse_args_from(["sonarr", "set-quality", "--to", "X", "--bogus"]).unwrap_err();
    assert!(err.to_string().contains("--bogus"), "{err}");
}

#[test]
fn set_quality_rejects_duplicate_to() {
    let err = parse_args_from(["sonarr", "set-quality", "--to", "X", "--to", "Y"]).unwrap_err();
    assert!(err.to_string().contains("duplicate"), "{err}");
}
