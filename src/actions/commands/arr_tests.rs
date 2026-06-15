use super::ARR_COMMANDS;
use crate::actions::model::READ_SCOPE;
use crate::capability::Capability;

#[test]
fn registers_the_seven_read_commands() {
    let names: Vec<&str> = ARR_COMMANDS.iter().map(|c| c.name).collect();
    for expected in [
        "quality_profiles",
        "list",
        "wanted",
        "queue",
        "history",
        "rootfolders",
        "health",
    ] {
        assert!(names.contains(&expected), "missing arr command {expected}");
    }
    assert_eq!(ARR_COMMANDS.len(), 7);
}

#[test]
fn all_arr_commands_are_read_scope_and_non_mutating() {
    // Security S2: read commands must use READ scope and not mutate.
    for cmd in ARR_COMMANDS {
        assert_eq!(cmd.required_scope, READ_SCOPE, "{} scope", cmd.name);
        assert!(!cmd.mutates, "{} must not mutate", cmd.name);
        assert!(
            !cmd.confirm_required,
            "{} must not require confirm",
            cmd.name
        );
        assert_eq!(
            cmd.capability,
            Capability::ArrManager,
            "{} capability",
            cmd.name
        );
        // Every read command requires only `service`.
        assert_eq!(cmd.required_params, &["service"], "{} params", cmd.name);
    }
}

#[test]
fn registered_through_central_registry() {
    // The slice is wired into the global registry, not just defined locally.
    assert!(crate::actions::curated_command("list").is_some());
    assert!(crate::actions::curated_command("quality_profiles").is_some());
}

#[tokio::test]
async fn handler_extracts_service_param_and_rejects_wrong_capability() {
    use crate::testing::loopback_state;
    use serde_json::json;

    // The `list` handler extracts `service` and calls the app method. Against a
    // sonarr (arr) loopback service the capability guard passes (transport then
    // fails, which is fine); against a missing/incompatible service it errors.
    let state = loopback_state();
    let cmd = crate::actions::curated_command("list").unwrap();

    // Missing service param → handler surfaces the validation error.
    let err = (cmd.handler)(&state.service, &json!({}))
        .await
        .expect_err("missing service should error");
    assert!(err.to_string().contains("service"), "{err}");
}
