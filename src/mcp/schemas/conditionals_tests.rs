use super::conditionals;
use crate::actions::required_params_for_action;

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
    // emitted (would be noise). With no curated commands (F4 state) there should
    // be zero allowed-kind fragments at all.
    let fragments = conditionals();
    let kind_fragments = fragments
        .iter()
        .filter(|f| f["then"]["properties"]["service"]["description"].is_string())
        .count();
    assert_eq!(
        kind_fragments, 0,
        "no curated commands registered → no allowed-kind constraints"
    );
}
