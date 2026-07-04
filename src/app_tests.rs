use super::*;
use crate::capability::Capability;
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
fn service_of_capability_matches_and_rejects() {
    let svc = service();
    // Sonarr is an ArrManager — resolving for that capability succeeds.
    assert!(
        svc.service_of_capability("sonarr", Capability::ArrManager)
            .is_ok()
    );
    // Resolving the same service for a mismatched capability fails closed.
    let err = svc
        .service_of_capability("sonarr", Capability::MediaServer)
        .unwrap_err();
    assert!(err.to_string().contains("does not provide"));
}

#[tokio::test]
async fn unknown_service_is_actionable() {
    let error = service()
        .api_get("missing", "/api/v3/system/status")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("unknown yarr service"));
}
