use super::INDEXER_COMMANDS;
use crate::actions::model::{READ_SCOPE, WRITE_SCOPE};
use crate::capability::Capability;

/// READ verbs.
const READ_COMMANDS: &[&str] = &["indexers", "indexer_search", "indexer_stats"];
/// WRITE verbs.
const WRITE_COMMANDS: &[&str] = &["indexer_test"];

#[test]
fn registers_all_indexer_commands() {
    let names: Vec<&str> = INDEXER_COMMANDS.iter().map(|c| c.name).collect();
    for expected in READ_COMMANDS.iter().chain(WRITE_COMMANDS) {
        assert!(
            names.contains(expected),
            "missing indexer command {expected}"
        );
    }
    assert_eq!(
        INDEXER_COMMANDS.len(),
        READ_COMMANDS.len() + WRITE_COMMANDS.len()
    );
}

#[test]
fn all_commands_are_indexer_capability() {
    for cmd in INDEXER_COMMANDS {
        assert_eq!(
            cmd.capability,
            Capability::Indexer,
            "{} must be Indexer-scoped",
            cmd.name
        );
    }
}

#[test]
fn read_commands_are_read_scope_and_non_mutating() {
    for cmd in INDEXER_COMMANDS
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
fn test_command_is_write_scope_mutates_and_confirm_gated() {
    // indexer test TRIGGERS a health-check command → write + mutates + confirm.
    let cmd = INDEXER_COMMANDS
        .iter()
        .find(|c| c.name == "indexer_test")
        .expect("indexer_test must be registered");
    assert_eq!(cmd.required_scope, WRITE_SCOPE);
    assert!(cmd.mutates, "indexer_test must mutate");
    assert!(cmd.confirm_required, "indexer_test must require confirm");
    assert!(
        cmd.description
            .contains("triggers an indexer health check (write)"),
        "description must teach the agent to expect the gate, got: {}",
        cmd.description
    );
}

#[test]
fn search_requires_query_param() {
    let cmd = INDEXER_COMMANDS
        .iter()
        .find(|c| c.name == "indexer_search")
        .expect("indexer_search must be registered");
    assert!(cmd.required_params.contains(&"service"));
    assert!(cmd.required_params.contains(&"query"));
}
