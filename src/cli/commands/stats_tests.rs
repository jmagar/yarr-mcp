use crate::cli::command::Command;
use crate::cli::parse_args_from;
use serde_json::json;

#[test]
fn tautulli_activity_maps_kebab_to_stats_activity() {
    let cmd = parse_args_from(["tautulli", "activity"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "stats_activity",
            params: json!({ "service": "tautulli" }),
        }
    );
}

#[test]
fn tautulli_users_and_libraries_map_to_stats_actions() {
    let users = parse_args_from(["tautulli", "users"]).unwrap().unwrap();
    assert_eq!(
        users,
        Command::Curated {
            action: "stats_users",
            params: json!({ "service": "tautulli" }),
        }
    );
    let libs = parse_args_from(["tautulli", "libraries"]).unwrap().unwrap();
    assert_eq!(
        libs,
        Command::Curated {
            action: "stats_libraries",
            params: json!({ "service": "tautulli" }),
        }
    );
}

#[test]
fn tautulli_history_parses_start_and_length() {
    let cmd = parse_args_from(["tautulli", "history", "--start", "0", "--length", "25"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "stats_history",
            params: json!({ "service": "tautulli", "start": "0", "length": "25" }),
        }
    );
}

#[test]
fn tautulli_history_parses_user_filter() {
    let cmd = parse_args_from(["tautulli", "history", "--user", "jacob"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "stats_history",
            params: json!({ "service": "tautulli", "user": "jacob" }),
        }
    );
}

#[test]
fn tautulli_write_verbs_run_immediately() {
    for (verb, action) in [
        ("refresh-libraries", "stats_refresh_libraries"),
        ("refresh-users", "stats_refresh_users"),
        ("delete-image-cache", "stats_delete_image_cache"),
    ] {
        let cmd = parse_args_from(["tautulli", verb]).unwrap().unwrap();
        assert_eq!(
            cmd,
            Command::Curated {
                action,
                params: json!({ "service": "tautulli" }),
            }
        );
    }
}

#[test]
fn activity_rejects_unexpected_flags() {
    let err = parse_args_from(["tautulli", "activity", "--start", "0"]).unwrap_err();
    assert!(err.to_string().contains("activity"));
}

#[test]
fn history_rejects_unknown_flag() {
    let err = parse_args_from(["tautulli", "history", "--bogus", "x"]).unwrap_err();
    assert!(err.to_string().contains("history"));
}

#[test]
fn write_verb_rejects_unknown_flag() {
    let err = parse_args_from(["tautulli", "refresh-users", "--bogus"]).unwrap_err();
    assert!(err.to_string().contains("refresh-users"));
}

#[test]
fn sonarr_activity_rejected_wrong_kind() {
    // `activity` is a Stats verb; sonarr is ArrManager, so its Stats parse module is
    // never consulted and the generic passthrough rejects the unknown verb.
    let err = parse_args_from(["sonarr", "activity"]).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown command") && msg.contains("sonarr"),
        "sonarr activity should be rejected, got: {msg}"
    );
}
