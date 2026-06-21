use super::REQUEST_COMMANDS;
use crate::actions::model::{READ_SCOPE, WRITE_SCOPE};
use crate::actions::{RustarrAction, curated_command, required_scope_for_action};
use crate::capability::Capability;
use serde_json::json;

/// READ verbs.
const READ_COMMANDS: &[&str] = &["requests", "request_search"];
/// WRITE verbs (mutating + confirm-gated).
const WRITE_COMMANDS: &[&str] = &["request_create", "request_approve", "request_decline"];

#[test]
fn registers_all_request_commands() {
    let names: Vec<&str> = REQUEST_COMMANDS.iter().map(|c| c.name).collect();
    for expected in READ_COMMANDS.iter().chain(WRITE_COMMANDS) {
        assert!(
            names.contains(expected),
            "missing request command {expected}"
        );
    }
    assert_eq!(
        REQUEST_COMMANDS.len(),
        READ_COMMANDS.len() + WRITE_COMMANDS.len()
    );
}

#[test]
fn all_commands_are_requests_capability() {
    for cmd in REQUEST_COMMANDS {
        assert_eq!(
            cmd.capability,
            Capability::Requests,
            "{} must be Requests-scoped",
            cmd.name
        );
    }
}

#[test]
fn action_names_avoid_global_collisions() {
    // Registry action names are globally unique; ArrManager already owns the bare
    // `search`/`add` names, so requests verbs use distinct names.
    for cmd in REQUEST_COMMANDS {
        assert!(
            cmd.name == "requests" || cmd.name.starts_with("request_"),
            "{} must be `requests` or `request_`-prefixed for global uniqueness",
            cmd.name
        );
        assert_ne!(
            cmd.name, "search",
            "must not collide with ArrManager search"
        );
        assert_ne!(cmd.name, "add", "must not collide with ArrManager add");
        assert_ne!(cmd.name, "request", "bare `request` is reserved/ambiguous");
    }
}

#[test]
fn read_commands_are_read_scope_and_non_mutating() {
    for cmd in REQUEST_COMMANDS
        .iter()
        .filter(|c| READ_COMMANDS.contains(&c.name))
    {
        assert_eq!(cmd.required_scope, READ_SCOPE, "{} scope", cmd.name);
        assert!(!cmd.mutates, "{} must not mutate", cmd.name);
        assert!(!cmd.destructive, "{} must not require confirm", cmd.name);
    }
}

#[test]
fn write_commands_are_write_scope_mutate_and_ungated() {
    // create/approve/decline mutate and use WRITE scope, but none are DESTRUCTIVE
    // (none delete media), so none are confirm-gated — they run immediately.
    for cmd in REQUEST_COMMANDS
        .iter()
        .filter(|c| WRITE_COMMANDS.contains(&c.name))
    {
        assert_eq!(cmd.required_scope, WRITE_SCOPE, "{} scope", cmd.name);
        assert!(cmd.mutates, "{} must mutate", cmd.name);
        assert!(
            !cmd.destructive,
            "{} is non-destructive and must not be confirm-gated",
            cmd.name
        );
    }
}

#[test]
fn create_declares_media_type_id_required_and_seasons_optional() {
    let create = REQUEST_COMMANDS
        .iter()
        .find(|c| c.name == "request_create")
        .unwrap();
    assert!(create.required_params.contains(&"service"));
    assert!(create.required_params.contains(&"media_type"));
    assert!(create.required_params.contains(&"media_id"));
    assert!(create.optional_params.contains(&"seasons"));
    // create is non-destructive now, so it no longer carries a confirm param.
    assert!(!create.optional_params.contains(&"confirm"));
}

#[test]
fn approve_and_decline_require_id() {
    for name in ["request_approve", "request_decline"] {
        let cmd = REQUEST_COMMANDS.iter().find(|c| c.name == name).unwrap();
        assert!(cmd.required_params.contains(&"id"), "{name} requires id");
    }
}

#[test]
fn approve_description_documents_manage_requests() {
    let cmd = REQUEST_COMMANDS
        .iter()
        .find(|c| c.name == "request_approve")
        .unwrap();
    assert!(
        cmd.description.contains("MANAGE_REQUESTS"),
        "approve help must document the admin-key requirement"
    );
}

// ── registry integration + MCP-dispatch parse (no live services) ─────────────────

#[test]
fn commands_are_visible_in_global_registry() {
    for name in READ_COMMANDS.iter().chain(WRITE_COMMANDS) {
        assert!(
            curated_command(name).is_some(),
            "{name} must be reachable through the global curated registry"
        );
    }
}

#[test]
fn registry_scopes_match_descriptors() {
    for name in READ_COMMANDS {
        assert_eq!(required_scope_for_action(name), Some(READ_SCOPE), "{name}");
    }
    for name in WRITE_COMMANDS {
        assert_eq!(required_scope_for_action(name), Some(WRITE_SCOPE), "{name}");
    }
}

#[test]
fn mcp_dispatch_parses_requests_to_curated_variant() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "requests",
        "service": "overseerr",
        "filter": "pending"
    }))
    .expect("requests action should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated {
            name: "requests",
            ..
        }
    ));
}

#[test]
fn mcp_dispatch_parses_request_create() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "request_create",
        "service": "overseerr",
        "media_type": "movie",
        "media_id": 27205,
        "confirm": true
    }))
    .expect("request_create action should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated {
            name: "request_create",
            ..
        }
    ));
}

#[test]
fn mcp_dispatch_parses_request_approve() {
    let action = RustarrAction::from_mcp_args(&json!({
        "action": "request_approve",
        "service": "overseerr",
        "id": 5,
        "confirm": true
    }))
    .expect("request_approve action should parse");
    assert!(matches!(
        action,
        RustarrAction::Curated {
            name: "request_approve",
            ..
        }
    ));
}
