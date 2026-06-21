use super::DOWNLOAD_COMMANDS;
use crate::actions::model::{READ_SCOPE, WRITE_SCOPE};
use crate::capability::Capability;

/// READ verbs.
const READ_COMMANDS: &[&str] = &["download_queue"];
/// WRITE verbs (mutating + confirm-gated).
const WRITE_COMMANDS: &[&str] = &[
    "download_add",
    "download_pause",
    "download_resume",
    "download_remove",
];

#[test]
fn registers_all_download_commands() {
    let names: Vec<&str> = DOWNLOAD_COMMANDS.iter().map(|c| c.name).collect();
    for expected in READ_COMMANDS.iter().chain(WRITE_COMMANDS) {
        assert!(
            names.contains(expected),
            "missing download command {expected}"
        );
    }
    assert_eq!(
        DOWNLOAD_COMMANDS.len(),
        READ_COMMANDS.len() + WRITE_COMMANDS.len()
    );
}

#[test]
fn all_commands_are_download_capability() {
    for cmd in DOWNLOAD_COMMANDS {
        assert_eq!(
            cmd.capability,
            Capability::DownloadClient,
            "{} must be DownloadClient-scoped",
            cmd.name
        );
    }
}

#[test]
fn action_names_are_capability_prefixed_for_global_uniqueness() {
    // Registry action names are globally unique; ArrManager already owns `queue`
    // (C1). Download verbs must use the `download_` prefix so they cannot collide.
    for cmd in DOWNLOAD_COMMANDS {
        assert!(
            cmd.name.starts_with("download_"),
            "{} must be download_-prefixed for global action-name uniqueness",
            cmd.name
        );
    }
}

#[test]
fn queue_is_read_scope_and_non_mutating() {
    let cmd = DOWNLOAD_COMMANDS
        .iter()
        .find(|c| c.name == "download_queue")
        .expect("download_queue registered");
    assert_eq!(cmd.required_scope, READ_SCOPE);
    assert!(!cmd.mutates, "queue must not mutate");
    assert!(!cmd.destructive, "queue must not require confirm");
}

#[test]
fn write_commands_are_write_scope_mutate_and_only_remove_is_gated() {
    // All writes use WRITE scope and mutate; only the DESTRUCTIVE `download_remove`
    // stays confirm-gated. add/pause/resume run immediately.
    for cmd in DOWNLOAD_COMMANDS
        .iter()
        .filter(|c| WRITE_COMMANDS.contains(&c.name))
    {
        assert_eq!(cmd.required_scope, WRITE_SCOPE, "{} scope", cmd.name);
        assert!(cmd.mutates, "{} must mutate", cmd.name);
        let destructive = cmd.name == "download_remove";
        assert_eq!(
            cmd.destructive, destructive,
            "{} destructive must equal destructive={destructive}",
            cmd.name
        );
    }
}

#[test]
fn add_and_remove_declare_required_params() {
    let add = DOWNLOAD_COMMANDS
        .iter()
        .find(|c| c.name == "download_add")
        .unwrap();
    assert!(add.required_params.contains(&"service"));
    assert!(add.required_params.contains(&"url"));

    let remove = DOWNLOAD_COMMANDS
        .iter()
        .find(|c| c.name == "download_remove")
        .unwrap();
    // `id` is OPTIONAL: a remove may target either --id or --hash, so only
    // `service` is required. The handler errors at runtime when neither is given.
    assert!(remove.required_params.contains(&"service"));
    assert!(!remove.required_params.contains(&"id"));
    assert!(remove.optional_params.contains(&"id"));
    assert!(remove.optional_params.contains(&"hash"));
    assert!(remove.optional_params.contains(&"delete_files"));
}
