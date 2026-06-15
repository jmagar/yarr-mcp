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
    for cmd in curated_commands() {
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
fn curated_registry_populated_with_arr_commands() {
    // C1 registers the ArrManager read commands; an unknown name still misses.
    assert!(!curated_commands().is_empty());
    assert!(curated_command("anything").is_none());
    assert!(curated_command("quality_profiles").is_some());
    assert!(curated_command("list").is_some());
}

#[test]
fn all_action_names_unions_generic_and_curated() {
    // Generic action names come first, then every curated command name.
    let names = all_action_names();
    for generic in action_names() {
        assert!(names.contains(&generic), "missing generic action {generic}");
    }
    for curated in curated_command_names() {
        assert!(names.contains(&curated), "missing curated action {curated}");
    }
    assert!(!curated_command_names().is_empty());
    // The curated param union is first-seen-ordered and de-duplicated. C1 read
    // commands contribute only `service`; C2 write commands add the selectors and
    // safety flags. Assert the union contains each declared param without pinning
    // exact ordering (so later beads can extend it freely).
    let params = curated_param_names();
    for expected in [
        "service",
        "to",
        "from",
        "title",
        "ids",
        "confirm",
        "bulk",
        "term",
        "quality_profile",
        "root_folder",
        "id",
        "delete_files",
    ] {
        assert!(
            params.contains(&expected),
            "curated param union missing {expected}: {params:?}"
        );
    }
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
fn capability_digest_lists_arr_commands() {
    // C1 state: the digest renders the arr capability with its read commands and
    // the kinds that share the ArrManager capability.
    let digest = capability_digest().expect("digest should exist once arr commands registered");
    assert!(
        digest.contains("arr("),
        "digest should label arr capability: {digest}"
    );
    assert!(
        digest.contains("sonarr"),
        "arr kinds should include sonarr: {digest}"
    );
    assert!(
        digest.contains("radarr"),
        "arr kinds should include radarr: {digest}"
    );
    assert!(
        digest.contains("quality_profiles"),
        "digest should list commands: {digest}"
    );
}

#[test]
fn arr_commands_valid_only_for_arr_kinds() {
    // Teaching guard: a curated arr command is allowed for sonarr/radarr but
    // rejected for a non-arr kind like plex.
    assert!(action_allowed_for_kind("list", ServiceKind::Sonarr));
    assert!(action_allowed_for_kind("list", ServiceKind::Radarr));
    assert!(!action_allowed_for_kind("list", ServiceKind::Plex));
    // And it appears in the valid-action list only for arr kinds.
    assert!(valid_actions_for_kind(ServiceKind::Sonarr).contains(&"list"));
    assert!(!valid_actions_for_kind(ServiceKind::Plex).contains(&"list"));
    // allowed-kind names for an arr command are a strict subset (4 arr kinds).
    let kinds = allowed_kind_names_for_action("list");
    assert!(kinds.contains(&"sonarr") && kinds.contains(&"radarr"));
    assert!(!kinds.contains(&"plex"));
    assert!(kinds.len() < ServiceKind::ALL.len());
}
