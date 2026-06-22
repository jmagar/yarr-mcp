use serde_json::json;

use super::{ActionCase, expect_type};

pub(super) fn cases(action: &str) -> Option<Vec<ActionCase>> {
    match action {
        "download_queue" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_type("array_or_object")],
        )]),
        "download_pause" | "download_resume" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action, "confirm": false }),
            vec![expect_type("array_or_object")],
        )]),
        "download_remove" => Some(vec![ActionCase::expected_error(
            action,
            json!({
                "action": action,
                "id": "__rustarr_live_missing_delete_target__",
                "delete_files": false,
                "confirm": false
            }),
            &["confirm=true", "confirm", "execution_error", action],
        )]),
        "download_add" => Some(vec![ActionCase::expected_error(
            action,
            json!({
                "action": action,
                "url": "",
                "confirm": false
            }),
            &["url", "empty", "invalid", "execution_error", action],
        )]),
        _ => None,
    }
}
