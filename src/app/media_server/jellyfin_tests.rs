use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::{build_url, query_get, slim, RustarrClient};
use serde_json::json;

use super::{LIBRARY_FIELDS, SEARCH_FIELDS, SEARCH_ITEM_TYPES, SESSION_FIELDS};

fn jellyfin_config() -> ServiceConfig {
    ServiceConfig {
        name: "jellyfin".into(),
        kind: ServiceKind::Jellyfin,
        base_url: "http://localhost:8096".into(),
        token: Some("jelly-secret".into()),
        ..ServiceConfig::default()
    }
}

fn jellyfin_config_at(base_url: &str) -> ServiceConfig {
    ServiceConfig {
        base_url: base_url.into(),
        ..jellyfin_config()
    }
}

fn jellyfin_service(config: ServiceConfig) -> RustarrService {
    let cfg = RustarrConfig {
        services: vec![config],
    };
    let client = RustarrClient::new(&cfg).unwrap();
    RustarrService::new(client, cfg)
}

/// Single-request TCP stub returning a JSON body; hands the request line back so a
/// test can assert what the REAL method put on the wire.
fn stub_get(body: &'static str) -> (String, std::sync::mpsc::Receiver<String>) {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" || line == "\n" {
                break;
            }
        }
        tx.send(request_line.trim_end().to_string()).unwrap();
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
fn sessions_path_is_sessions() {
    let url = build_url(&jellyfin_config(), "/Sessions").expect("url");
    assert_eq!(url.path(), "/Sessions");
    // Jellyfin uses header auth, so the token must NOT appear in the query string.
    assert!(
        url.query().is_none() || !url.query().unwrap().contains("jelly-secret"),
        "Jellyfin token must not leak into the query string: {url}"
    );
}

#[test]
fn libraries_path_is_virtual_folders() {
    let url = build_url(&jellyfin_config(), "/Library/VirtualFolders").expect("url");
    assert_eq!(url.path(), "/Library/VirtualFolders");
}

#[test]
fn search_always_includes_item_types_and_recursive() {
    // FACT (bead, HIGH): everything in Jellyfin is a BaseItemDto, so an item query
    // without includeItemTypes returns folders/noise. It must ALWAYS be present.
    let url = query_get(
        &jellyfin_config(),
        "/Items",
        &[
            ("searchTerm", "dune"),
            ("includeItemTypes", SEARCH_ITEM_TYPES),
            ("recursive", "true"),
        ],
    )
    .expect("url");
    assert_eq!(url.path(), "/Items");
    let query = url.query().expect("query present");
    assert!(
        query.contains("includeItemTypes=Movie%2CSeries%2CEpisode")
            || query.contains("includeItemTypes=Movie,Series,Episode"),
        "includeItemTypes must always be sent: {query}"
    );
    assert!(query.contains("recursive=true"), "recursive=true: {query}");
    assert!(query.contains("searchTerm=dune"), "searchTerm: {query}");
}

#[tokio::test]
async fn search_includes_item_types_on_the_wire() {
    // Call the REAL search() via media_search and assert includeItemTypes (and
    // recursive) are on the wire — without it Jellyfin returns folders/noise.
    let (base, rx) = stub_get("{\"Items\":[]}");
    let svc = jellyfin_service(jellyfin_config_at(&base));
    let _ = svc.media_search("jellyfin", "dune").await.unwrap();
    let line = rx.recv().unwrap();
    assert!(
        line.starts_with("GET /Items?"),
        "search hits /Items: {line}"
    );
    assert!(
        line.contains("includeItemTypes=Movie%2CSeries%2CEpisode")
            || line.contains("includeItemTypes=Movie,Series,Episode"),
        "includeItemTypes on the wire: {line}"
    );
    assert!(line.contains("recursive=true"), "{line}");
    assert!(line.contains("searchTerm=dune"), "{line}");
}

#[test]
fn search_percent_encodes_user_query() {
    // A query with reserved chars must be percent-encoded by query_get (S6).
    let url = query_get(
        &jellyfin_config(),
        "/Items",
        &[
            ("searchTerm", "dune&IsFavorite=true"),
            ("includeItemTypes", SEARCH_ITEM_TYPES),
            ("recursive", "true"),
        ],
    )
    .expect("url");
    let query = url.query().expect("query present");
    assert!(query.contains("%26"), "ampersand percent-encoded: {query}");
    // Exactly one searchTerm pair — no injection of a second parameter.
    assert_eq!(
        query.matches("searchTerm=").count(),
        1,
        "single searchTerm pair: {query}"
    );
}

#[test]
fn scan_targets_library_refresh() {
    let url = build_url(&jellyfin_config(), "/Library/Refresh").expect("url");
    assert_eq!(url.path(), "/Library/Refresh");
}

#[test]
fn session_slim_keeps_expected_fields() {
    let raw = json!([{
        "UserName": "jacob",
        "NowPlayingItem": {"Name": "Dune"},
        "DeviceName": "Living Room",
        "Client": "Jellyfin Web",
        "PlayState": {"PositionTicks": 123},
        "bulky": "drop me"
    }]);
    let slimmed = slim(raw, SESSION_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["UserName"], "jacob");
    assert_eq!(row["NowPlayingItem"]["Name"], "Dune");
    assert!(row.get("bulky").is_none(), "bulky fields dropped");
}

#[test]
fn library_slim_keeps_expected_fields() {
    let raw = json!([{
        "ItemId": "uuid-1", "Name": "Movies", "CollectionType": "movies", "Locations": ["/x"]
    }]);
    let slimmed = slim(raw, LIBRARY_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["ItemId"], "uuid-1");
    assert_eq!(row["CollectionType"], "movies");
    assert!(row.get("Locations").is_none());
}

#[test]
fn search_slim_keeps_expected_fields() {
    // Ids are UUID strings (bead FACT).
    let raw = json!([{
        "Id": "a1b2c3d4-0000", "Name": "Dune", "Type": "Movie",
        "ProductionYear": 2021, "SeriesName": null, "Overview": "drop"
    }]);
    let slimmed = slim(raw, SEARCH_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["Id"], "a1b2c3d4-0000");
    assert_eq!(row["ProductionYear"], 2021);
    assert!(row.get("Overview").is_none());
}
