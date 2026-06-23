//! Discovery catalog tests.

use super::*;
use crate::config::ServiceKind;

fn services() -> Vec<(String, ServiceKind)> {
    vec![
        ("sonarr".to_string(), ServiceKind::Sonarr),
        ("radarr".to_string(), ServiceKind::Radarr),
        ("plex".to_string(), ServiceKind::Plex),
    ]
}

#[test]
fn catalog_paths_are_fully_qualified_per_service() {
    let cat = build_catalog(&services());
    let paths: Vec<&str> = cat.iter().map(|e| e.path.as_str()).collect();
    // Every configured service gets a status callable and its curated commands,
    // each prefixed with the service name. The service is baked into the path.
    assert!(paths.contains(&"sonarr.service_status"));
    assert!(paths.contains(&"radarr.service_status"));
    assert!(paths.contains(&"sonarr.list"));
    assert!(paths.contains(&"radarr.list"));
    assert!(paths.contains(&"plex.media_sessions"));
    // No bare action names leak in — discovery only offers callable paths.
    assert!(!paths.contains(&"list"));
    assert!(!paths.contains(&"integrations"));
    assert!(!paths.contains(&"codemode"));
}

#[test]
fn each_entry_carries_its_service() {
    let cat = build_catalog(&services());
    let sonarr_list = cat.iter().find(|e| e.path == "sonarr.list").unwrap();
    assert_eq!(sonarr_list.service.as_deref(), Some("sonarr"));
    assert_eq!(sonarr_list.method, "list");
    assert_eq!(sonarr_list.kind, "curated");
    assert_ne!(sonarr_list.capability, "infra");
    assert!(!sonarr_list.description.is_empty());
    // `service` is baked in, never a param the script passes.
    assert!(!sonarr_list.required_params.contains(&"service"));
}

#[test]
fn catalog_paths_are_unique() {
    let cat = build_catalog(&services());
    let mut paths: Vec<&str> = cat.iter().map(|e| e.path.as_str()).collect();
    paths.sort_unstable();
    let mut deduped = paths.clone();
    deduped.dedup();
    assert_eq!(paths, deduped, "catalog callable paths must be unique");
}

#[test]
fn raw_api_client_is_documented_service_agnostically() {
    let cat = build_catalog(&services());
    let get = cat.iter().find(|e| e.path == "api.<service>.get").unwrap();
    assert_eq!(get.scope, "write"); // api_get requires write scope
    assert!(!get.destructive);
    assert!(get.service.is_none(), "raw-api docs are service-agnostic");

    let del = cat
        .iter()
        .find(|e| e.path == "api.<service>.delete")
        .unwrap();
    assert!(del.destructive, "api_delete is destructive");
}

#[test]
fn catalog_json_is_valid_json_array() {
    let json = catalog_json(&services());
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_array());
    assert_eq!(
        parsed.as_array().unwrap().len(),
        build_catalog(&services()).len()
    );
}

#[test]
fn empty_services_yields_only_raw_api_docs() {
    let cat = build_catalog(&[]);
    // No services configured → only the four service-agnostic raw-API entries.
    assert_eq!(cat.len(), 4);
    assert!(cat.iter().all(|e| e.service.is_none()));
}
