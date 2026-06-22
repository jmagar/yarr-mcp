use serde_json::json;

use super::{ActionCase, expect_path_contains, matrix};

pub(super) fn cases(service: &matrix::ServiceCase, action: &str) -> Option<Vec<ActionCase>> {
    match action {
        "integrations" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_path_contains("supported", &service.name)],
        )]),
        "help" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![expect_path_contains("help", "api_get")],
        )]),
        "service_status" => Some(vec![ActionCase::ok(
            action,
            json!({ "action": action }),
            vec![service.status.clone()],
        )]),
        "api_get" => Some(
            service
                .get
                .iter()
                .map(|get| {
                    ActionCase::ok(
                        format!("api_get {}", get.path),
                        json!({ "action": "api_get", "path": get.path }),
                        vec![get.expectation.clone()],
                    )
                })
                .collect(),
        ),
        "api_post" | "api_put" => {
            let mut args = json!({
                "action": action,
                "path": service.post_expected_error.path,
                "confirm": false,
            });
            args["body"] = service.post_expected_error.body.clone();
            let mut tokens = service.post_expected_error.error_contains_any.clone();
            tokens.extend(["execution_error".to_string(), action.to_string()]);
            Some(vec![ActionCase::expected_error_tokens(
                action, args, tokens,
            )])
        }
        "api_delete" => Some(vec![ActionCase::expected_error(
            action,
            json!({
                "action": action,
                "path": service.post_expected_error.path,
                "confirm": false,
            }),
            &["confirm=true", "confirm", "execution_error", action],
        )]),
        _ => None,
    }
}
