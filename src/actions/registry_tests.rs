use super::*;
use crate::config::ServiceKind;

#[test]
fn action_metadata_matches_rustarr_surface() {
    assert_eq!(
        action_names(),
        vec![
            "integrations",
            "service_status",
            "api_get",
            "api_post",
            "api_put",
            "api_delete",
            "help"
        ]
    );
    assert_eq!(required_scope_for_action("api_get"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_post"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_put"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_delete"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("help"), None);
    assert_eq!(
        rest_action_names(),
        vec![
            "integrations",
            "service_status",
            "api_get",
            "api_post",
            "api_put",
            "api_delete",
            "help"
        ]
    );
    assert_eq!(mcp_only_action_names(), Vec::<&str>::new());
}

#[test]
fn unknown_action_denies() {
    assert_eq!(required_scope_for_action("nope"), Some(DENY_SCOPE));
    assert!(!is_known_action("nope"));
}

#[test]
fn no_read_only_curated_command_carries_write_scope() {
    // Security S2: a non-mutating curated command must NEVER require write scope,
    // so read-only tokens work for dashboards.
    for cmd in CURATED_COMMANDS {
        if !cmd.mutates {
            assert_ne!(
                cmd.required_scope, WRITE_SCOPE,
                "read-only command {} must not require write scope",
                cmd.name
            );
        }
    }
}

#[test]
fn action_allowed_for_kind_allows_infra_for_all_kinds() {
    for kind in ServiceKind::ALL {
        for action in action_names() {
            assert!(
                action_allowed_for_kind(action, kind),
                "infra action {action} should be allowed for {}",
                kind.as_str()
            );
        }
        // Unknown action fails closed for every kind.
        assert!(!action_allowed_for_kind("totally_unknown", kind));
    }
}

#[test]
fn valid_actions_for_kind_includes_infra() {
    let valid = valid_actions_for_kind(ServiceKind::Plex);
    assert!(valid.contains(&"integrations"));
    assert!(valid.contains(&"service_status"));
}

#[test]
fn curated_registry_empty_for_f1() {
    assert!(CURATED_COMMANDS.is_empty());
    assert!(curated_command("anything").is_none());
}

#[test]
fn all_action_names_unions_generic_and_curated() {
    // With no curated commands, this equals the generic action names.
    assert_eq!(all_action_names(), action_names());
    assert!(curated_command_names().is_empty());
    assert!(curated_param_names().is_empty());
}

#[test]
fn required_params_mirror_parser_contract() {
    assert_eq!(
        required_params_for_action("integrations"),
        Vec::<&str>::new()
    );
    assert_eq!(
        required_params_for_action("service_status"),
        vec!["service"]
    );
    assert_eq!(
        required_params_for_action("api_get"),
        vec!["service", "path"]
    );
    assert_eq!(
        required_params_for_action("api_post"),
        vec!["service", "path", "confirm"]
    );
}

#[test]
fn allowed_kind_names_covers_all_kinds_for_infra() {
    let all = allowed_kind_names_for_action("api_get");
    assert_eq!(all.len(), ServiceKind::ALL.len());
    // Unknown action → no allowed kinds (fail closed).
    assert!(allowed_kind_names_for_action("unknown").is_empty());
}

#[test]
fn capability_digest_is_none_without_curated_commands() {
    // F4 state: no curated commands → no digest section.
    assert!(capability_digest().is_none());
}
