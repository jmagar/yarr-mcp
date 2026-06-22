use serde_json::json;

use super::{ActionCase, expect_type, matrix};

pub(super) fn cases(service: &matrix::ServiceCase, action: &str) -> Option<Vec<ActionCase>> {
    match action {
        "indexers" => {
            let assertions = if service.name == "prowlarr" {
                vec![matrix::Expectation {
                    json_path: None,
                    equals: None,
                    equals_any: None,
                    value_type: Some("array".into()),
                    contains: Some("Rustarr Live LinuxTracker".into()),
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
        "indexer_search" => {
            let (query, mut payload, assertions) = if service.name == "prowlarr" {
                (
                    "ubuntu",
                    json!({ "action": action, "query": "ubuntu", "ids": [1] }),
                    vec![matrix::Expectation {
                        json_path: None,
                        equals: None,
                        equals_any: None,
                        value_type: Some("array".into()),
                        contains: Some("Rustarr Live LinuxTracker".into()),
                        xml_root: None,
                    }],
                )
            } else {
                (
                    "star",
                    json!({ "action": action, "query": "star" }),
                    vec![expect_type("array_or_object")],
                )
            };
            payload["query"] = json!(query);
            Some(vec![ActionCase::ok(action, payload, assertions)])
        }
        "indexer_stats" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_type("object")],
        )]),
        "indexer_test" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action, "confirm": false }),
            vec![expect_type("array_or_object")],
        )]),
        _ => None,
    }
}
