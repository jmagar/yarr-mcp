use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::{slim, RustarrClient};
use serde_json::json;

use super::{INDEXER_FIELDS, STATS_FIELDS};

fn service_with(kinds: &[(&str, ServiceKind)]) -> RustarrService {
    let services = kinds
        .iter()
        .map(|(name, kind)| ServiceConfig {
            name: (*name).into(),
            kind: *kind,
            base_url: "http://localhost:1".into(),
            api_key: Some("test".into()),
            ..ServiceConfig::default()
        })
        .collect();
    let config = RustarrConfig { services };
    let client = RustarrClient::new(&config).expect("client builds");
    RustarrService::new(client, config)
}

#[test]
fn indexer_path_uses_v1_prefix() {
    // Descriptor-driven: prowlarr is /api/v1, no hardcoded version.
    let config = ServiceConfig {
        name: "prowlarr".into(),
        kind: ServiceKind::Prowlarr,
        base_url: "http://localhost:1".into(),
        api_key: Some("test".into()),
        ..ServiceConfig::default()
    };
    assert_eq!(
        RustarrService::indexer_path(&config, "indexer"),
        "/api/v1/indexer"
    );
    assert_eq!(
        RustarrService::indexer_path(&config, "indexerstats"),
        "/api/v1/indexerstats"
    );
    // Fix 5: single-id test is GET indexer/{id} then POST indexer/test (there is
    // NO indexer/{id}/test route); test-all is POST indexer/testall.
    assert_eq!(
        RustarrService::indexer_path(&config, "indexer/3"),
        "/api/v1/indexer/3"
    );
    assert_eq!(
        RustarrService::indexer_path(&config, "indexer/test"),
        "/api/v1/indexer/test"
    );
    assert_eq!(
        RustarrService::indexer_path(&config, "indexer/testall"),
        "/api/v1/indexer/testall"
    );
}

#[test]
fn indexers_slim_keeps_expected_fields() {
    let raw = json!([{
        "id": 1,
        "name": "NZBgeek",
        "enable": true,
        "protocol": "usenet",
        "priority": 25,
        "definitionName": "nzbgeek",
        "fields": [{"name": "apiKey", "value": "secret"}]
    }]);
    let slimmed = slim(raw, INDEXER_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["id"], 1);
    assert_eq!(row["name"], "NZBgeek");
    assert_eq!(row["enable"], true);
    assert_eq!(row["protocol"], "usenet");
    assert_eq!(row["priority"], 25);
    // Sensitive / bulky fields dropped.
    assert!(row.get("definitionName").is_none());
    assert!(row.get("fields").is_none());
}

#[test]
fn stats_slim_keeps_counter_fields() {
    let raw = json!({
        "indexerId": 2,
        "indexerName": "DrunkenSlug",
        "numberOfQueries": 100,
        "numberOfGrabs": 5,
        "numberOfFailedQueries": 1,
        "averageResponseTime": 250,
        "internalField": "drop me"
    });
    let slimmed = slim(raw, STATS_FIELDS);
    assert_eq!(slimmed["indexerId"], 2);
    assert_eq!(slimmed["indexerName"], "DrunkenSlug");
    assert_eq!(slimmed["numberOfQueries"], 100);
    assert_eq!(slimmed["numberOfGrabs"], 5);
    assert!(slimmed.get("internalField").is_none());
}

#[tokio::test]
async fn indexer_list_rejects_non_indexer_kind() {
    // sonarr is ArrManager, not Indexer — the capability check must reject it
    // before any request is built (wrong-capability reject).
    let svc = service_with(&[("sonarr", ServiceKind::Sonarr)]);
    let err = svc
        .indexer_list("sonarr")
        .await
        .expect_err("indexer_list on sonarr must be rejected");
    assert!(
        err.to_string().contains("Indexer"),
        "error should mention the Indexer capability, got: {err}"
    );
}

#[tokio::test]
async fn indexer_search_rejects_non_indexer_kind() {
    let svc = service_with(&[("radarr", ServiceKind::Radarr)]);
    let err = svc
        .indexer_search("radarr", "ubuntu", &[])
        .await
        .expect_err("indexer_search on radarr must be rejected");
    assert!(err.to_string().contains("Indexer"));
}

#[tokio::test]
async fn indexer_test_requires_confirm() {
    // The write guard runs before the capability/transport, so an unconfigured
    // prowlarr still surfaces the confirm error.
    let svc = service_with(&[("prowlarr", ServiceKind::Prowlarr)]);
    let err = svc
        .indexer_test("prowlarr", None, false)
        .await
        .expect_err("indexer_test without confirm must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("confirm"),
        "expected confirm error, got: {msg}"
    );
    assert!(
        msg.contains("health check"),
        "error should teach that test triggers a health check, got: {msg}"
    );
}
