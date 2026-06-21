use super::MEDIA_COMMANDS;
use crate::actions::model::{READ_SCOPE, WRITE_SCOPE};
use crate::capability::Capability;

/// READ verbs.
const READ_COMMANDS: &[&str] = &["media_sessions", "media_libraries", "media_search"];
/// WRITE verbs (mutating + confirm-gated).
const WRITE_COMMANDS: &[&str] = &["media_scan"];

#[test]
fn registers_all_media_commands() {
    let names: Vec<&str> = MEDIA_COMMANDS.iter().map(|c| c.name).collect();
    for expected in READ_COMMANDS.iter().chain(WRITE_COMMANDS) {
        assert!(names.contains(expected), "missing media command {expected}");
    }
    assert_eq!(
        MEDIA_COMMANDS.len(),
        READ_COMMANDS.len() + WRITE_COMMANDS.len()
    );
}

#[test]
fn all_commands_are_media_server_capability() {
    for cmd in MEDIA_COMMANDS {
        assert_eq!(
            cmd.capability,
            Capability::MediaServer,
            "{} must be MediaServer-scoped",
            cmd.name
        );
    }
}

#[test]
fn action_names_are_capability_prefixed_for_global_uniqueness() {
    // Registry action names are globally unique; ArrManager already owns `search`.
    // Media verbs must use the `media_` prefix so they cannot collide.
    for cmd in MEDIA_COMMANDS {
        assert!(
            cmd.name.starts_with("media_"),
            "{} must be media_-prefixed for global action-name uniqueness",
            cmd.name
        );
    }
}

#[test]
fn read_commands_are_read_scope_and_non_mutating() {
    for cmd in MEDIA_COMMANDS
        .iter()
        .filter(|c| READ_COMMANDS.contains(&c.name))
    {
        assert_eq!(cmd.required_scope, READ_SCOPE, "{} scope", cmd.name);
        assert!(!cmd.mutates, "{} must not mutate", cmd.name);
        assert!(
            !cmd.confirm_required,
            "{} must not require confirm",
            cmd.name
        );
    }
}

#[test]
fn scan_is_write_scope_mutate_and_ungated() {
    let cmd = MEDIA_COMMANDS
        .iter()
        .find(|c| c.name == "media_scan")
        .expect("media_scan registered");
    assert_eq!(cmd.required_scope, WRITE_SCOPE);
    assert!(cmd.mutates, "scan must mutate");
    assert!(
        !cmd.confirm_required,
        "scan is non-destructive and must not be confirm-gated"
    );
}

#[test]
fn search_declares_query_required_and_scan_library_optional() {
    let search = MEDIA_COMMANDS
        .iter()
        .find(|c| c.name == "media_search")
        .unwrap();
    assert!(search.required_params.contains(&"service"));
    assert!(search.required_params.contains(&"query"));

    let scan = MEDIA_COMMANDS
        .iter()
        .find(|c| c.name == "media_scan")
        .unwrap();
    assert!(scan.optional_params.contains(&"library"));
    // scan is non-destructive now, so it no longer carries a confirm param.
    assert!(!scan.optional_params.contains(&"confirm"));
}
