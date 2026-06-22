use serde_json::json;

use super::{ActionCase, expect_type};

pub(super) fn cases(action: &str) -> Option<Vec<ActionCase>> {
    match action {
        "requests" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_type("object")],
        )]),
        "request_search" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action, "query": "star" }),
            vec![expect_type("array_or_object")],
        )]),
        "request_approve" | "request_decline" => Some(vec![ActionCase::expected_error(
            action,
            json!({ "action": action, "id": 999999999_i64, "confirm": false }),
            &["404", "not found", "request", "execution_error", action],
        )]),
        "request_create" => Some(vec![ActionCase::expected_error(
            action,
            json!({
                "action": action,
                "media_type": "__rustarr_live_invalid_media_type__",
                "media_id": -1,
                "confirm": false
            }),
            &["media", "invalid", "400", "404", "execution_error", action],
        )]),
        _ => None,
    }
}
