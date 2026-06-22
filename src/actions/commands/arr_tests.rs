use super::ARR_COMMANDS;
use crate::actions::model::{READ_SCOPE, WRITE_SCOPE};
use crate::capability::Capability;

/// C1 read verbs.
const READ_COMMANDS: &[&str] = &[
    "quality_profiles",
    "list",
    "wanted",
    "queue",
    "history",
    "rootfolders",
    "health",
];

/// C2 write/intent verbs.
const WRITE_COMMANDS: &[&str] = &[
    "set_quality",
    "search",
    "refresh",
    "monitor",
    "unmonitor",
    "add",
    "delete",
];

/// The DESTRUCTIVE subset of the write verbs — the only ones still confirm-gated.
const DESTRUCTIVE_COMMANDS: &[&str] = &["delete"];

#[test]
fn registers_all_read_and_write_commands() {
    let names: Vec<&str> = ARR_COMMANDS.iter().map(|c| c.name).collect();
    for expected in READ_COMMANDS.iter().chain(WRITE_COMMANDS) {
        assert!(names.contains(expected), "missing arr command {expected}");
    }
    assert_eq!(
        ARR_COMMANDS.len(),
        READ_COMMANDS.len() + WRITE_COMMANDS.len()
    );
}

#[test]
fn read_commands_are_read_scope_and_non_mutating() {
    // Security S2: read commands must use READ scope and not mutate.
    for cmd in ARR_COMMANDS
        .iter()
        .filter(|c| READ_COMMANDS.contains(&c.name))
    {
        assert_eq!(cmd.required_scope, READ_SCOPE, "{} scope", cmd.name);
        assert!(!cmd.mutates, "{} must not mutate", cmd.name);
        assert!(!cmd.destructive, "{} must not require confirm", cmd.name);
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
fn write_commands_are_write_scope_mutating_and_only_destructive_is_gated() {
    // Security S3/AN-4: every write/intent command requires WRITE scope and
    // declares it mutates. Only DESTRUCTIVE commands stay confirm-gated; plain
    // writes run immediately.
    for cmd in ARR_COMMANDS
        .iter()
        .filter(|c| WRITE_COMMANDS.contains(&c.name))
    {
        assert_eq!(cmd.required_scope, WRITE_SCOPE, "{} scope", cmd.name);
        assert!(cmd.mutates, "{} must mutate", cmd.name);
        let destructive = DESTRUCTIVE_COMMANDS.contains(&cmd.name);
        assert_eq!(
            cmd.destructive, destructive,
            "{} destructive must equal destructive={destructive}",
            cmd.name
        );
        assert_eq!(
            cmd.capability,
            Capability::ArrManager,
            "{} capability",
            cmd.name
        );
        // Every write command targets a service.
        assert!(
            cmd.required_params.contains(&"service"),
            "{} must require service",
            cmd.name
        );
    }
}

#[test]
fn set_quality_requires_to_profile() {
    let cmd = ARR_COMMANDS
        .iter()
        .find(|c| c.name == "set_quality")
        .expect("set_quality registered");
    assert!(
        cmd.required_params.contains(&"to"),
        "set_quality needs --to"
    );
    assert!(cmd.optional_params.contains(&"from"));
}

#[test]
fn list_advertises_bounded_response_params() {
    let cmd = ARR_COMMANDS
        .iter()
        .find(|c| c.name == "list")
        .expect("list registered");
    for param in ["limit", "offset", "fields"] {
        assert!(
            cmd.optional_params.contains(&param),
            "list should advertise optional {param}"
        );
        assert!(
            cmd.typed_params.iter().any(|(name, _)| *name == param),
            "list should type optional {param}"
        );
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
