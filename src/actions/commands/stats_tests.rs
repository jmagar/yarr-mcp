use super::STATS_COMMANDS;
use crate::actions::model::READ_SCOPE;
use crate::actions::{RustarrAction, curated_command, required_scope_for_action};
use crate::capability::Capability;
use serde_json::json;

/// Every Stats command (all READ).
const ALL_COMMANDS: &[&str] = &[
    "stats_activity",
    "stats_history",
    "stats_users",
    "stats_libraries",
];

#[test]
fn registers_all_stats_commands() {
    let names: Vec<&str> = STATS_COMMANDS.iter().map(|c| c.name).collect();
    for expected in ALL_COMMANDS {
        assert!(names.contains(expected), "missing stats command {expected}");
    }
    assert_eq!(STATS_COMMANDS.len(), ALL_COMMANDS.len());
}

#[test]
fn all_commands_are_stats_capability() {
    for cmd in STATS_COMMANDS {
        assert_eq!(
            cmd.capability,
            Capability::Stats,
            "{} must be Stats-scoped",
            cmd.name
        );
    }
}

#[test]
fn all_commands_are_read_scope_and_non_mutating() {
    // Tautulli is read-only stats — every command is READ, non-mutating, no confirm.
    for cmd in STATS_COMMANDS {
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
fn action_names_are_stats_prefixed_avoiding_global_collisions() {
    // `history` is owned by ArrManager; action names are globally unique, so every
    // stats action is `stats_`-prefixed.
    for cmd in STATS_COMMANDS {
        assert!(
            cmd.name.starts_with("stats_"),
            "{} must be `stats_`-prefixed for global uniqueness",
            cmd.name
        );
        assert_ne!(
            cmd.name, "history",
            "must not collide with ArrManager history"
        );
    }
}

#[test]
fn names_are_unique_within_slice() {
    let mut names: Vec<&str> = STATS_COMMANDS.iter().map(|c| c.name).collect();
    names.sort_unstable();
    let len = names.len();
    names.dedup();
    assert_eq!(names.len(), len, "stats action names must be unique");
}

#[test]
fn history_declares_optional_pagination_and_user() {
    let history = STATS_COMMANDS
        .iter()
        .find(|c| c.name == "stats_history")
        .unwrap();
    assert!(history.required_params.contains(&"service"));
    assert!(history.optional_params.contains(&"start"));
    assert!(history.optional_params.contains(&"length"));
    assert!(history.optional_params.contains(&"user"));
}

// ── registry integration + MCP-dispatch parse (no live services) ─────────────────

#[test]
fn commands_are_visible_in_global_registry() {
    for name in ALL_COMMANDS {
        assert!(
            curated_command(name).is_some(),
            "{name} must be reachable through the global curated registry"
        );
    }
}

#[test]
fn registry_scopes_match_descriptors() {
    for name in ALL_COMMANDS {
        assert_eq!(required_scope_for_action(name), Some(READ_SCOPE), "{name}");
    }
}

#[test]
fn mcp_dispatch_parses_stats_activity_to_curated_variant() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "stats_activity",
        "service": "tautulli"
    }))
    .expect("stats_activity action should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated {
            name: "stats_activity",
            ..
        }
    ));
}

#[test]
fn mcp_dispatch_parses_stats_history_with_pagination() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "stats_history",
        "service": "tautulli",
        "start": 0,
        "length": 25,
        "user": "jacob"
    }))
    .expect("stats_history action should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated {
            name: "stats_history",
            ..
        }
    ));
}

#[test]
fn mcp_dispatch_parses_stats_users_and_libraries() {
    for name in ["stats_users", "stats_libraries"] {
        let action = RustarrAction::from_mcp_args(&json!({
            "action": name,
            "service": "tautulli"
        }))
        .unwrap_or_else(|_| panic!("{name} should parse"));
        assert!(
            matches!(action, RustarrAction::Curated { name: parsed, .. } if parsed == name),
            "{name} parsed to the wrong curated command"
        );
    }
}
