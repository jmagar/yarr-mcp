use serde_json::json;

use super::{ActionCase, expect_type};

pub(super) fn cases(action: &str) -> Option<Vec<ActionCase>> {
    read_cases(action).or_else(|| write_cases(action))
}

fn read_cases(action: &str) -> Option<Vec<ActionCase>> {
    match action {
        "quality_profiles" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_type("array")],
        )]),
        "list" | "rootfolders" | "health" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_type("array_or_object")],
        )]),
        "wanted" | "queue" | "history" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_type("object")],
        )]),
        _ => None,
    }
}

fn write_cases(action: &str) -> Option<Vec<ActionCase>> {
    match action {
        "set_quality" => Some(vec![ActionCase::expected_error(
            action,
            json!({
                "action": action,
                "to": "__rustarr_live_missing_profile__",
                "confirm": false
            }),
            &[
                "quality profile",
                "available profiles",
                "confirm_required",
                "confirm",
                "execution_error",
                action,
            ],
        )]),
        "search" | "refresh" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action, "confirm": false }),
            vec![expect_type("array_or_object")],
        )]),
        "monitor" | "unmonitor" => Some(vec![ActionCase::expected_error(
            action,
            json!({ "action": action, "confirm": false }),
            &[
                "id",
                "ids",
                "title",
                "selection",
                "monitored",
                "execution_error",
                action,
            ],
        )]),
        "delete" => Some(vec![ActionCase::expected_error(
            action,
            json!({
                "action": action,
                "id": 999999999_i64,
                "delete_files": false,
                "confirm": false
            }),
            &["confirm=true", "confirm", "execution_error", action],
        )]),
        "add" => Some(vec![ActionCase::expected_error(
            action,
            json!({
                "action": action,
                "term": "__rustarr_live_missing_title__",
                "quality_profile": "__rustarr_live_missing_profile__",
                "root_folder": "/__rustarr_live_missing_root__",
                "confirm": false
            }),
            &[
                "quality profile",
                "lookup",
                "confirm_required",
                "confirm",
                "execution_error",
                action,
            ],
        )]),
        _ => None,
    }
}
