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

fn overseerr_at(base_url: &str) -> RustarrService {
    let config = RustarrConfig {
        services: vec![ServiceConfig {
            name: "overseerr".into(),
            kind: ServiceKind::Overseerr,
            base_url: base_url.into(),
            api_key: Some("test".into()),
            ..ServiceConfig::default()
        }],
    };
    let client = RustarrClient::new(&config).expect("client builds");
    RustarrService::new(client, config)
}

/// Captured request line + body from a one-shot stub.
struct Captured {
    request_line: String,
    body: String,
}

/// Single-request TCP stub returning a small JSON body; captures the request line
/// (`POST path HTTP/1.1`) and the request body so a test can assert the REAL
/// method's on-the-wire payload.
fn stub_capture(body: &'static str) -> (String, std::sync::mpsc::Receiver<Captured>) {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel::<Captured>();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        let mut content_length = 0_usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = rest.trim().parse().unwrap_or(0);
            }
        }
        let mut buf = vec![0_u8; content_length];
        if content_length > 0 {
            reader.read_exact(&mut buf).unwrap();
        }
        tx.send(Captured {
            request_line: request_line.trim_end().to_string(),
            body: String::from_utf8_lossy(&buf).to_string(),
        })
        .unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let _ = stream.flush();
    });
    (format!("http://{addr}"), rx)
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

#[tokio::test]
async fn req_create_body_carries_media_type_id_and_seasons_on_the_wire() {
    // Call the REAL req_create() (with confirm) against a stub and assert the POST
    // body actually carries {mediaType, mediaId, seasons}.
    let (base, rx) = stub_capture("{\"id\":1}");
    let svc = overseerr_at(&base);
    let _ = svc
        .req_create("overseerr", "tv", 1399, &[1, 2], true)
        .await
        .unwrap();
    let req = rx.recv().unwrap();
    assert!(
        req.request_line.starts_with("POST /api/v1/request "),
        "req_create POSTs /api/v1/request: {}",
        req.request_line
    );
    let body: serde_json::Value = serde_json::from_str(&req.body).expect("json body");
    assert_eq!(body["mediaType"], "tv");
    assert_eq!(body["mediaId"], 1399);
    assert_eq!(body["seasons"], json!([1, 2]));
}

#[tokio::test]
async fn req_create_movie_omits_seasons_on_the_wire() {
    let (base, rx) = stub_capture("{\"id\":1}");
    let svc = overseerr_at(&base);
    let _ = svc
        .req_create("overseerr", "movie", 27205, &[], true)
        .await
        .unwrap();
    let req = rx.recv().unwrap();
    let body: serde_json::Value = serde_json::from_str(&req.body).expect("json body");
    assert_eq!(body["mediaType"], "movie");
    assert_eq!(body["mediaId"], 27205);
    assert!(
        body.get("seasons").is_none(),
        "movie omits seasons: {}",
        req.body
    );
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
