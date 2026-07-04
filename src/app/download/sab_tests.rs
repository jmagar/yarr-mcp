use crate::app::YarrService;
use crate::config::{ServiceConfig, ServiceKind, YarrConfig};
use crate::yarr::{YarrClient, query_get, slim};
use serde_json::json;

use super::{QUEUE_FIELDS, SAB_API};

fn sab_config() -> ServiceConfig {
    ServiceConfig {
        name: "sabnzbd".into(),
        kind: ServiceKind::Sabnzbd,
        base_url: "http://localhost:8080".into(),
        api_key: Some("secret".into()),
        ..ServiceConfig::default()
    }
}

fn sab_config_at(base_url: &str) -> ServiceConfig {
    ServiceConfig {
        base_url: base_url.into(),
        ..sab_config()
    }
}

fn sab_service(config: ServiceConfig) -> YarrService {
    let cfg = YarrConfig {
        services: vec![config],
    };
    let client = YarrClient::new(&cfg).unwrap();
    YarrService::new(client, cfg)
}

/// Single-request TCP stub returning a small JSON body; hands the request line
/// (`GET path HTTP/1.1`) back so a test can assert what the REAL method put on the
/// wire (SAB is a `?mode=` GET API, so the assertion is on the query string).
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
fn queue_uses_mode_query_and_json_output() {
    // SAB is a ?mode= query API — never a /api/v2 REST path.
    let url = query_get(&sab_config(), SAB_API, &[("mode", "queue")]).expect("url builds");
    let query = url.query().expect("query present");
    assert!(
        query.contains("mode=queue"),
        "expected mode=queue, got: {query}"
    );
    assert!(
        query.contains("output=json"),
        "SAB forces output=json: {query}"
    );
    assert!(query.contains("apikey=secret"), "apikey injected: {query}");
    assert_eq!(url.path(), "/api");
}

#[test]
fn add_uses_addurl_mode_with_percent_encoded_name() {
    // A url with reserved chars must be percent-encoded by query_get, never
    // format!'d into the path (S6: no second-parameter injection).
    let url = query_get(
        &sab_config(),
        SAB_API,
        &[("mode", "addurl"), ("name", "http://x/a?b=c&d=e")],
    )
    .expect("url builds");
    let query = url.query().expect("query present");
    assert!(query.contains("mode=addurl"), "got: {query}");
    // The raw `&d=e` must be encoded so it cannot become its own query pair.
    assert!(query.contains("name=http"), "name param present: {query}");
    assert!(
        query.contains("%3F") && query.contains("%26"),
        "both reserved chars (? and &) must be percent-encoded: {query}"
    );
    // Exactly one `mode=` pair — no injection of a second mode.
    assert_eq!(
        query.matches("mode=").count(),
        1,
        "single mode pair: {query}"
    );
}

#[tokio::test]
async fn remove_with_delete_files_sends_del_files_on_the_wire() {
    // Call the REAL remove() against a stub and assert del_files=1 is on the wire.
    let (base, rx) = stub_get("{\"status\":true}");
    let config = sab_config_at(&base);
    let svc = sab_service(config.clone());
    let _ = super::remove(&svc, &config, "SABnzbd_nzo_x", true)
        .await
        .unwrap();
    let line = rx.recv().unwrap();
    assert!(line.starts_with("GET /api?"), "SAB ?mode= GET: {line}");
    assert!(line.contains("mode=queue"), "{line}");
    assert!(line.contains("name=delete"), "{line}");
    assert!(line.contains("value=SABnzbd_nzo_x"), "{line}");
    assert!(
        line.contains("del_files=1"),
        "del_files on the wire: {line}"
    );
}

#[tokio::test]
async fn remove_without_delete_files_omits_del_files_on_the_wire() {
    let (base, rx) = stub_get("{\"status\":true}");
    let config = sab_config_at(&base);
    let svc = sab_service(config.clone());
    let _ = super::remove(&svc, &config, "SABnzbd_nzo_x", false)
        .await
        .unwrap();
    let line = rx.recv().unwrap();
    assert!(
        !line.contains("del_files"),
        "no del_files by default: {line}"
    );
}

#[test]
fn queue_slim_keeps_expected_fields() {
    let slots = json!([{
        "nzo_id": "SABnzbd_nzo_a",
        "filename": "Ubuntu.iso",
        "status": "Downloading",
        "percentage": "42",
        "mb": "700",
        "mbleft": "406",
        "timeleft": "0:05:00",
        "cat": "software",
        "priority": "Normal",
        "internal": "drop me"
    }]);
    let slimmed = slim(slots, QUEUE_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["nzo_id"], "SABnzbd_nzo_a");
    assert_eq!(row["filename"], "Ubuntu.iso");
    assert_eq!(row["percentage"], "42");
    assert!(row.get("internal").is_none(), "bulky fields dropped");
}
