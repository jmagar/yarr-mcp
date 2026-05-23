use super::*;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};

fn service() -> RustarrService {
    let config = RustarrConfig {
        services: vec![ServiceConfig {
            name: "sonarr".into(),
            kind: ServiceKind::Sonarr,
            base_url: "http://localhost:1".into(),
            api_key: Some("secret".into()),
            ..ServiceConfig::default()
        }],
    };
    let client = RustarrClient::new(&config).unwrap();
    RustarrService::new(client, config)
}

#[test]
fn integrations_omits_secret_values() {
    let value = service().integrations();
    assert_eq!(value["configured"][0]["name"], "sonarr");
    assert_eq!(value["configured"][0]["api_key_configured"], true);
    assert!(!value.to_string().contains("secret"));
}

#[tokio::test]
async fn unknown_service_is_actionable() {
    let error = service()
        .api_get("missing", "/api/v3/system/status")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("unknown rustarr service"));
}
