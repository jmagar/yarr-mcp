use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::{build_url, query_get, slim, RustarrClient};
use serde_json::json;

use super::{LIBRARY_FIELDS, SEARCH_FIELDS, SESSION_FIELDS};

fn plex_config(base_url: &str) -> ServiceConfig {
    ServiceConfig {
        name: "plex".into(),
        kind: ServiceKind::Plex,
        base_url: base_url.into(),
        token: Some("plex-secret".into()),
        ..ServiceConfig::default()
    }
}

fn plex_service(config: ServiceConfig) -> RustarrService {
    let cfg = RustarrConfig {
        services: vec![config],
    };
    let client = RustarrClient::new(&cfg).unwrap();
    RustarrService::new(client, cfg)
}

/// Single-request TCP stub: capture the request header lines, return `body` as
/// JSON, and hand the headers back over a channel. Used to assert the transport
/// actually puts `Accept: application/json` on the wire for Plex.
fn stub_once(body: &'static str) -> (String, std::sync::mpsc::Receiver<Vec<String>>) {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel::<Vec<String>>();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut header_lines = Vec::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }
            if line == "\r\n" || line == "\n" {
                break;
            }
            header_lines.push(line.trim_end().to_string());
        }
        let mut sink = [0_u8; 256];
        let _ = reader
            .get_mut()
            .set_read_timeout(Some(std::time::Duration::from_millis(20)));
        let _ = reader.get_mut().read(&mut sink);
        tx.send(header_lines).unwrap();
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
fn sessions_path_is_status_sessions_with_token() {
    let url = build_url(&plex_config("http://localhost:32400"), "/status/sessions").expect("url");
    assert_eq!(url.path(), "/status/sessions");
    // PlexToken auth is injected as a query param, never a header.
    assert!(
        url.query().unwrap().contains("X-Plex-Token=plex-secret"),
        "X-Plex-Token query param injected: {url}"
    );
}

#[test]
fn libraries_path_is_library_sections() {
    let url = build_url(&plex_config("http://localhost:32400"), "/library/sections").expect("url");
    assert_eq!(url.path(), "/library/sections");
}

#[test]
fn search_uses_library_search_with_percent_encoded_query() {
    // Reserved chars in the query must be percent-encoded by query_get, never
    // format!'d into the path (S6: no second-parameter injection).
    let url = query_get(
        &plex_config("http://localhost:32400"),
        "/library/search",
        &[("query", "dune&type=movie")],
    )
    .expect("url");
    assert_eq!(url.path(), "/library/search");
    let query = url.query().expect("query present");
    assert!(query.contains("query=dune"), "query param present: {query}");
    assert!(
        query.contains("%26"),
        "reserved chars percent-encoded: {query}"
    );
    // Exactly one `query=` pair — no injection of a second parameter.
    assert_eq!(query.matches("query=").count(), 1, "single query pair");
}

#[test]
fn scan_path_targets_section_refresh() {
    let url = build_url(
        &plex_config("http://localhost:32400"),
        "/library/sections/3/refresh",
    )
    .expect("url");
    assert_eq!(url.path(), "/library/sections/3/refresh");
}

#[tokio::test]
async fn sessions_requests_accept_application_json() {
    // FACT (bead, HIGH): Plex returns XML unless Accept: application/json is sent
    // on EVERY call. Assert the header is actually on the wire.
    let (base, rx) = stub_once("{\"MediaContainer\":{\"Metadata\":[]}}");
    let svc = plex_service(plex_config(&base));
    let _ = svc.media_sessions("plex").await;
    let headers = rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("stub server should record a request within 2s");
    assert!(
        headers
            .iter()
            .any(|h| h.eq_ignore_ascii_case("accept: application/json")),
        "Plex sessions must send `Accept: application/json`, headers: {headers:?}"
    );
}

#[tokio::test]
async fn search_requests_accept_application_json() {
    let (base, rx) = stub_once("{\"MediaContainer\":{\"Metadata\":[]}}");
    let svc = plex_service(plex_config(&base));
    let _ = svc.media_search("plex", "dune").await;
    let headers = rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("stub server should record a request within 2s");
    assert!(
        headers
            .iter()
            .any(|h| h.eq_ignore_ascii_case("accept: application/json")),
        "Plex search must send `Accept: application/json`, headers: {headers:?}"
    );
}

#[test]
fn session_slim_keeps_expected_fields() {
    let raw = json!({"MediaContainer":{"Metadata":[{
        "title": "Dune",
        "type": "movie",
        "User": {"title": "jacob"},
        "Player": {"title": "Living Room"},
        "Session": {"id": "abc"},
        "viewOffset": 12345,
        "bulky": "drop me"
    }]}});
    let metadata = super::unwrap_container(&raw, "Metadata");
    let slimmed = slim(metadata, SESSION_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["title"], "Dune");
    assert_eq!(row["User"]["title"], "jacob");
    assert!(row.get("bulky").is_none(), "bulky fields dropped");
}

#[test]
fn library_slim_keeps_key_title_type() {
    let raw = json!({"MediaContainer":{"Directory":[{
        "key": "3", "title": "Movies", "type": "movie", "agent": "drop"
    }]}});
    let dirs = super::unwrap_container(&raw, "Directory");
    let slimmed = slim(dirs, LIBRARY_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["key"], "3");
    assert_eq!(row["title"], "Movies");
    assert!(row.get("agent").is_none());
}

#[test]
fn search_slim_keeps_expected_fields() {
    let raw = json!({"MediaContainer":{"Metadata":[{
        "ratingKey": "999", "title": "Dune", "type": "movie",
        "year": 2021, "librarySectionTitle": "Movies", "summary": "drop"
    }]}});
    let hits = super::unwrap_container(&raw, "Metadata");
    let slimmed = slim(hits, SEARCH_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["ratingKey"], "999");
    assert_eq!(row["year"], 2021);
    assert!(row.get("summary").is_none());
}
