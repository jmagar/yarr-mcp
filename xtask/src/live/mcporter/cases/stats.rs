use serde_json::json;

use super::{ActionCase, expect_type, matrix};

pub(super) fn cases(service: &matrix::ServiceCase, action: &str) -> Option<Vec<ActionCase>> {
    match action {
        "stats_activity" | "stats_users" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_type("array_or_object")],
        )]),
        "stats_libraries" => {
            let assertions = if service.name == "tautulli" {
                vec![matrix::Expectation {
                    json_path: None,
                    equals: None,
                    equals_any: None,
                    value_type: Some("array".into()),
                    contains: Some("Rustarr Live Movies".into()),
                    xml_root: None,
                }]
            } else {
                vec![expect_type("array_or_object")]
            };
            Some(vec![ActionCase::ok(
                action,
                json!({ "action": action }),
                assertions,
            )])
        }
        "stats_history" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action, "start": 0, "length": 1 }),
            vec![expect_type("array_or_object")],
        )]),
        "stats_refresh_libraries" | "stats_refresh_users" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action, "confirm": false }),
            vec![expect_type("array_or_object")],
        )]),
        "stats_delete_image_cache" => Some(vec![ActionCase::expected_error(
            action,
            json!({ "action": action, "confirm": false }),
            &["confirm=true", "confirm", "execution_error", action],
        )]),
        _ => None,
    }
}
