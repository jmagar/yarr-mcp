use anyhow::{Result, bail};
use serde_json::json;

use super::{ActionCase, matrix};

pub(super) fn action_cases(service: &matrix::ServiceCase, action: &str) -> Result<Vec<ActionCase>> {
    let mut cases = Vec::new();
    match action {
        "integrations" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_path_contains("supported", &service.name)],
            ));
        }
        "help" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_path_contains("help", "api_get")],
            ));
        }
        "service_status" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![service.status.clone()],
            ));
        }
        "api_get" => {
            for get in &service.get {
                cases.push(ActionCase::ok(
                    format!("api_get {}", get.path),
                    json!({ "action": "api_get", "path": get.path }),
                    vec![get.expectation.clone()],
                ));
            }
        }
        "api_post" | "api_put" | "api_delete" => {
            let mut args = json!({
                "action": action,
                "path": service.post_blocked.path,
                "confirm": false,
            });
            if action != "api_delete" {
                args["body"] = service.post_blocked.body.clone();
            }
            cases.push(ActionCase::expected_error(
                action,
                args,
                &["confirm=true", "confirm", "execution_error", action],
            ));
        }
        "quality_profiles" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_type("array")],
            ));
        }
        "list" | "rootfolders" | "health" | "download_queue" | "media_sessions"
        | "stats_activity" | "stats_users" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_type("array_or_object")],
            ));
        }
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
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                assertions,
            ));
        }
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
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                assertions,
            ));
        }
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
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                assertions,
            ));
        }
        "wanted" | "queue" | "history" | "indexer_stats" | "requests" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action }),
                vec![expect_type("object")],
            ));
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
            cases.push(ActionCase::ok(action, payload, assertions));
        }
        "request_search" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action, "query": "star" }),
                vec![expect_type("array_or_object")],
            ));
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
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action, "query": query }),
                assertions,
            ));
        }
        "stats_history" => {
            cases.push(ActionCase::ok(
                action,
                json!({ "action": action, "start": 0, "length": 1 }),
                vec![expect_type("array_or_object")],
            ));
        }
        "stats_refresh_libraries" | "stats_refresh_users" | "stats_delete_image_cache" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({ "action": action, "confirm": false }),
                &["confirm=true", "confirm", "execution_error", action],
            ));
        }
        "set_quality" => {
            cases.push(ActionCase::expected_error(
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
            ));
        }
        "search" | "refresh" | "monitor" | "unmonitor" | "indexer_test" | "media_scan"
        | "download_pause" | "download_resume" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({ "action": action, "confirm": false }),
                &[
                    "confirm=true",
                    "confirm_required",
                    "confirm",
                    "execution_error",
                    action,
                ],
            ));
        }
        "add" => {
            cases.push(ActionCase::expected_error(
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
            ));
        }
        "delete" | "request_approve" | "request_decline" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({ "action": action, "id": "1", "confirm": false }),
                &[
                    "confirm=true",
                    "confirm_required",
                    "confirm",
                    "execution_error",
                    action,
                ],
            ));
        }
        "download_add" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({
                    "action": action,
                    "url": "magnet:?xt=urn:btih:0000000000000000000000000000000000000000",
                    "confirm": false
                }),
                &["confirm=true", "confirm", "execution_error", action],
            ));
        }
        "download_remove" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({
                    "action": action,
                    "id": "__rustarr_live_missing_download__",
                    "delete_files": false,
                    "confirm": false
                }),
                &["confirm=true", "confirm", "execution_error", action],
            ));
        }
        "request_create" => {
            cases.push(ActionCase::expected_error(
                action,
                json!({
                    "action": action,
                    "media_type": "movie",
                    "media_id": 603,
                    "confirm": false
                }),
                &["confirm=true", "confirm", "execution_error", action],
            ));
        }
        other => bail!(
            "action {other} is advertised for {} but xtask has no stateful mcporter test case",
            service.name
        ),
    }
    Ok(cases)
}

fn expect_type(value_type: &str) -> matrix::Expectation {
    matrix::Expectation {
        json_path: None,
        equals: None,
        equals_any: None,
        value_type: Some(value_type.to_owned()),
        contains: None,
        xml_root: None,
    }
}

fn expect_path_contains(path: &str, needle: &str) -> matrix::Expectation {
    matrix::Expectation {
        json_path: Some(path.to_owned()),
        equals: None,
        equals_any: None,
        value_type: None,
        contains: Some(needle.to_owned()),
        xml_root: None,
    }
}
