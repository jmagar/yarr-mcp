use super::*;

#[test]
fn kind_queries_include_infra_and_only_matching_curated_actions() {
    let sonarr = valid_actions_for_kind(ServiceKind::Sonarr);
    assert!(sonarr.contains(&"help"));
    assert!(sonarr.contains(&"op"));
    assert!(!sonarr.contains(&"download_queue"));
    assert!(action_allowed_for_kind("help", ServiceKind::Sonarr));
    assert!(!action_allowed_for_kind(
        "download_queue",
        ServiceKind::Sonarr
    ));
}

#[test]
fn curated_parameter_queries_are_consistent() {
    assert!(curated_param_names().contains(&"service"));
    for action in actions_for_curated_param("service") {
        assert!(
            required_params_for_action(action).contains(&"service")
                || curated_command(action)
                    .is_some_and(|command| command.optional_params.contains(&"service"))
        );
    }
}
