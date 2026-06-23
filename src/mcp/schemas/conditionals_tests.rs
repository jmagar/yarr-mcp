use super::conditionals;
use crate::actions::required_params_for_action;
use crate::config::ServiceKind;

#[test]
fn emits_required_params_fragment_for_each_generic_action() {
    let fragments = conditionals(ServiceKind::Sonarr);
    for action in ["api_get", "api_post", "api_put", "api_delete"] {
        let required: Vec<_> = required_params_for_action(action)
            .into_iter()
            .filter(|param| *param != "service")
            .collect();
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
fn service_status_has_no_required_fragment_because_service_is_implied() {
    let fragments = conditionals(ServiceKind::Sonarr);
    assert!(
        !fragments
            .iter()
            .any(|f| f["if"]["properties"]["action"]["const"] == "service_status")
    );
}

#[test]
fn curated_required_params_still_emit_without_service() {
    let fragments = conditionals(ServiceKind::Qbittorrent);
    let frag = fragments
        .iter()
        .find(|f| f["if"]["properties"]["action"]["const"] == "download_add")
        .expect("download_add should require the `url` param");
    let required = frag["then"]["required"].as_array().unwrap();
    assert!(required.iter().any(|field| field == "url"));
    assert!(!required.iter().any(|field| field == "service"));
}
