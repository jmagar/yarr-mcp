use super::conditionals;
use crate::actions::{allowed_kind_names_for_action, required_params_for_action};

#[test]
fn emits_required_params_fragment_for_each_generic_action() {
    let fragments = conditionals();
    for action in [
        "service_status",
        "api_get",
        "api_post",
        "api_put",
        "api_delete",
    ] {
        let required = required_params_for_action(action);
        assert!(!required.is_empty());
        let found = fragments.iter().any(|f| {
            f["if"]["properties"]["action"]["const"] == action
                && f["then"]["required"]
                    .as_array()
                    .is_some_and(|r| required.iter().all(|p| r.iter().any(|v| v == p)))
        });
        assert!(found, "missing required-params fragment for {action}");
    }
}

#[test]
fn no_allowed_kinds_fragment_for_generic_actions() {
    // Generic actions are valid for all kinds, so no kind-constraint fragment is
    // emitted for them (would be noise). Find the kind fragments and assert none
    // belong to a generic action.
    let fragments = conditionals();
    for action in ["api_get", "service_status", "integrations", "help"] {
        let has_kind_fragment = fragments.iter().any(|f| {
            f["if"]["properties"]["action"]["const"] == action
                && f["then"]["properties"]["service"]["description"].is_string()
        });
        assert!(
            !has_kind_fragment,
            "generic action {action} should not get an allowed-kind fragment"
        );
    }
}

#[test]
fn emits_allowed_kinds_fragment_for_curated_arr_command() {
    // C1: a capability-scoped curated command (valid for a strict subset of
    // kinds) gets a documentation fragment recording its allowed `service` kinds.
    let fragments = conditionals();
    let allowed = allowed_kind_names_for_action("list");
    assert!(!allowed.is_empty());
    let frag = fragments
        .iter()
        .find(|f| {
            f["if"]["properties"]["action"]["const"] == "list"
                && f["then"]["properties"]["service"]["description"].is_string()
        })
        .expect("curated `list` command should emit an allowed-kind fragment");
    let desc = frag["then"]["properties"]["service"]["description"]
        .as_str()
        .unwrap();
    assert!(desc.contains("sonarr") && desc.contains("radarr"));
    assert!(!desc.contains("plex"));
}
