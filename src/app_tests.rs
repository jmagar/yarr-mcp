use super::*;
use crate::capability::Capability;
use crate::config::{ServiceConfig, ServiceKind, YarrConfig};

fn service() -> YarrService {
    let config = YarrConfig {
        services: vec![ServiceConfig {
            name: "sonarr".into(),
            kind: ServiceKind::Sonarr,
            base_url: "http://localhost:1".into(),
            api_key: Some("secret".into()),
            ..ServiceConfig::default()
        }],
    };
    let client = YarrClient::new(&config).unwrap();
    YarrService::new(client, config)
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

fn multi_instance_service() -> YarrService {
    let config = YarrConfig {
        services: vec![
            ServiceConfig {
                name: "sonarr-east".into(),
                kind: ServiceKind::Sonarr,
                base_url: "http://localhost:1".into(),
                ..ServiceConfig::default()
            },
            ServiceConfig {
                name: "sonarr-west".into(),
                kind: ServiceKind::Sonarr,
                base_url: "http://localhost:2".into(),
                ..ServiceConfig::default()
            },
            ServiceConfig {
                name: "movies".into(),
                kind: ServiceKind::Radarr,
                base_url: "http://localhost:3".into(),
                ..ServiceConfig::default()
            },
        ],
    };
    let client = YarrClient::new(&config).unwrap();
    YarrService::new(client, config)
}

#[test]
fn configured_name_wins_and_unique_kind_falls_back() {
    let svc = multi_instance_service();
    assert_eq!(svc.service("sonarr-east").unwrap().name, "sonarr-east");
    assert_eq!(svc.service("movies").unwrap().name, "movies");
    assert_eq!(svc.service("radarr").unwrap().name, "movies");
}

#[test]
fn ambiguous_kind_requires_configured_name() {
    let error = multi_instance_service()
        .service("sonarr")
        .expect_err("two Sonarr instances make kind fallback ambiguous");
    let message = error.to_string();
    assert!(message.contains("ambiguous"), "got: {message}");
    assert!(message.contains("sonarr-east"), "got: {message}");
    assert!(message.contains("sonarr-west"), "got: {message}");
}
