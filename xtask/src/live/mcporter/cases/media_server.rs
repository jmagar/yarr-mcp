use serde_json::json;

use super::{ActionCase, expect_type, matrix};

pub(super) fn cases(service: &matrix::ServiceCase, action: &str) -> Option<Vec<ActionCase>> {
    match action {
        "media_sessions" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_type("array_or_object")],
        )]),
        "media_libraries" => {
            let assertions = match service.name.as_str() {
                "plex" | "jellyfin" => vec![matrix::Expectation {
                    json_path: Some("libraries".into()),
                    equals: None,
                    equals_any: None,
                    value_type: Some("array".into()),
                    contains: Some("Rustarr Live Movies".into()),
                    xml_root: None,
                }],
                _ => vec![expect_type("array_or_object")],
            };
            Some(vec![ActionCase::ok(
                action,
                json!({ "action": action }),
                assertions,
            )])
        }
        "media_search" => {
            let (query, assertions) = match service.name.as_str() {
                "plex" | "jellyfin" => (
                    "Rustarr",
                    vec![matrix::Expectation {
                        json_path: Some("results".into()),
                        equals: None,
                        equals_any: None,
                        value_type: Some("array".into()),
                        contains: Some("Rustarr Fixture Movie".into()),
                        xml_root: None,
                    }],
                ),
                _ => ("star", vec![expect_type("array_or_object")]),
            };
            Some(vec![ActionCase::ok(
                action,
                json!({ "action": action, "query": query }),
                assertions,
            )])
        }
        "media_scan" => {
            if service.name == "plex" {
                Some(vec![ActionCase::expected_error(
                    action,
                    json!({ "action": action, "confirm": false }),
                    &["library", "execution_error", action],
                )])
            } else {
                Some(vec![ActionCase::ok(
                    action,
                    json!({ "action": action, "confirm": false }),
                    vec![expect_type("array_or_object")],
                )])
            }
        }
        _ => None,
    }
}
