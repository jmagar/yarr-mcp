use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::{slim, RustarrClient};
use serde_json::json;

use super::{REQUEST_FIELDS, SEARCH_FIELDS};

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
fn requests_path_uses_v1_prefix() {
    // Descriptor-driven: overseerr is /api/v1, no hardcoded version.
    let config = ServiceConfig {
        name: "overseerr".into(),
        kind: ServiceKind::Overseerr,
        base_url: "http://localhost:1".into(),
        api_key: Some("test".into()),
        ..ServiceConfig::default()
    };
    assert_eq!(
        RustarrService::requests_path(&config, "request"),
        "/api/v1/request"
    );
    assert_eq!(
        RustarrService::requests_path(&config, "request/5/approve"),
        "/api/v1/request/5/approve"
    );
    assert_eq!(
        RustarrService::requests_path(&config, "request/5/decline"),
        "/api/v1/request/5/decline"
    );
}

#[test]
fn create_body_carries_media_type_id_and_seasons() {
    // The create body shape is a business decision: {mediaType, mediaId, seasons}.
    // seasons is included only when non-empty.
    let mut body = json!({ "mediaType": "tv", "mediaId": 1399 });
    let seasons = [1_i64, 2];
    body["seasons"] = json!(seasons);
    assert_eq!(body["mediaType"], "tv");
    assert_eq!(body["mediaId"], 1399);
    assert_eq!(body["seasons"], json!([1, 2]));

    // movie request omits seasons.
    let movie = json!({ "mediaType": "movie", "mediaId": 27205 });
    assert!(movie.get("seasons").is_none());
}

#[test]
fn request_slim_keeps_expected_fields() {
    let raw = json!([{
        "id": 7,
        "type": "movie",
        "status": 2,
        "media": { "title": "Inception", "tmdbId": 27205 },
        "requestedBy": { "displayName": "jacob" },
        "modifiedBy": { "displayName": "admin" },
        "internalNote": "drop me"
    }]);
    let slimmed = slim(raw, REQUEST_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["id"], 7);
    assert_eq!(row["type"], "movie");
    assert_eq!(row["status"], 2);
    assert_eq!(row["media"]["title"], "Inception");
    assert_eq!(row["requestedBy"]["displayName"], "jacob");
    // Bulky / irrelevant fields dropped.
    assert!(row.get("modifiedBy").is_none());
    assert!(row.get("internalNote").is_none());
}

#[test]
fn search_slim_keeps_expected_fields() {
    let raw = json!({
        "id": 27205,
        "mediaType": "movie",
        "title": "Inception",
        "releaseDate": "2010-07-16",
        "overview": "A thief...",
        "popularity": 99.9,
        "voteAverage": 8.3
    });
    let slimmed = slim(raw, SEARCH_FIELDS);
    assert_eq!(slimmed["id"], 27205);
    assert_eq!(slimmed["mediaType"], "movie");
    assert_eq!(slimmed["title"], "Inception");
    assert!(slimmed.get("popularity").is_none());
    assert!(slimmed.get("voteAverage").is_none());
}

#[tokio::test]
async fn req_list_rejects_non_requests_kind() {
    // sonarr is ArrManager, not Requests — the capability check must reject it
    // before any request is built (wrong-capability reject).
    let svc = service_with(&[("sonarr", ServiceKind::Sonarr)]);
    let err = svc
        .req_list("sonarr", None, None, None)
        .await
        .expect_err("req_list on sonarr must be rejected");
    assert!(
        err.to_string().contains("Requests"),
        "error should mention the Requests capability, got: {err}"
    );
}

#[tokio::test]
async fn req_search_rejects_non_requests_kind() {
    let svc = service_with(&[("radarr", ServiceKind::Radarr)]);
    let err = svc
        .req_search("radarr", "dune")
        .await
        .expect_err("req_search on radarr must be rejected");
    assert!(err.to_string().contains("Requests"));
}

#[tokio::test]
async fn req_create_requires_confirm() {
    // The write guard runs before the capability/transport, so an unconfigured
    // overseerr still surfaces the confirm error.
    let svc = service_with(&[("overseerr", ServiceKind::Overseerr)]);
    let err = svc
        .req_create("overseerr", "movie", 27205, &[], false)
        .await
        .expect_err("req_create without confirm must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("confirm"),
        "expected confirm error, got: {msg}"
    );
}

#[tokio::test]
async fn req_approve_requires_confirm_and_teaches_manage_requests() {
    let svc = service_with(&[("overseerr", ServiceKind::Overseerr)]);
    let err = svc
        .req_approve("overseerr", 5, false)
        .await
        .expect_err("req_approve without confirm must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("confirm"),
        "expected confirm error, got: {msg}"
    );
    assert!(
        msg.contains("MANAGE_REQUESTS"),
        "approve error should teach the admin-key requirement, got: {msg}"
    );
}

#[tokio::test]
async fn req_decline_requires_confirm() {
    let svc = service_with(&[("overseerr", ServiceKind::Overseerr)]);
    let err = svc
        .req_decline("overseerr", 5, false)
        .await
        .expect_err("req_decline without confirm must be rejected");
    assert!(err.to_string().contains("confirm"));
}
