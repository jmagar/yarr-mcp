use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::{slim, RustarrClient};
use serde_json::json;

use super::{qbit_path, TORRENT_FIELDS};

fn qbit_config() -> ServiceConfig {
    ServiceConfig {
        name: "qbittorrent".into(),
        kind: ServiceKind::Qbittorrent,
        base_url: "http://localhost:8081".into(),
        username: Some("admin".into()),
        password: Some("pw".into()),
        ..ServiceConfig::default()
    }
}

/// A qbit config with NO username/password, so `ensure_qbittorrent_session`
/// returns early and the stub server only has to answer ONE request (the mutation
/// itself) rather than a login round-trip first.
fn qbit_config_no_auth(base_url: &str) -> ServiceConfig {
    ServiceConfig {
        name: "qbittorrent".into(),
        kind: ServiceKind::Qbittorrent,
        base_url: base_url.into(),
        ..ServiceConfig::default()
    }
}

fn qbit_service(config: ServiceConfig) -> RustarrService {
    let cfg = RustarrConfig {
        services: vec![config],
    };
    let client = RustarrClient::new(&cfg).unwrap();
    RustarrService::new(client, cfg)
}

/// Captured request: the start-line (`METHOD path HTTP/1.1`) plus the body text.
struct CapturedRequest {
    request_line: String,
    body: String,
}

/// Single-request TCP stub: capture the request line + body, return an EMPTY 200
/// (exactly what qBittorrent does for stop/start/delete), and hand the captured
/// request back over a channel. Lets us assert the REAL method puts the right path
/// and form body on the wire.
fn stub_empty_200() -> (String, std::sync::mpsc::Receiver<CapturedRequest>) {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel::<CapturedRequest>();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        let mut content_length = 0_usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = rest.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0_u8; content_length];
        if content_length > 0 {
            reader.read_exact(&mut body).unwrap();
        }
        tx.send(CapturedRequest {
            request_line: request_line.trim_end().to_string(),
            body: String::from_utf8_lossy(&body).to_string(),
        })
        .unwrap();
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").unwrap();
        let _ = stream.flush();
    });
    (format!("http://{addr}"), rx)
}

#[test]
fn qbit_path_uses_v2_prefix() {
    // Descriptor-driven: qbittorrent is /api/v2, no hardcoded version.
    let config = qbit_config();
    assert_eq!(
        qbit_path(&config, "/torrents/info"),
        "/api/v2/torrents/info"
    );
    assert_eq!(qbit_path(&config, "/torrents/add"), "/api/v2/torrents/add");
    assert_eq!(
        qbit_path(&config, "/torrents/delete"),
        "/api/v2/torrents/delete"
    );
}

#[tokio::test]
async fn pause_hits_torrents_stop_on_the_wire() {
    // FACT (bead, HIGH): qBittorrent v5 renamed pause->stop. Prove the REAL method
    // POSTs /api/v2/torrents/stop with the hashes form field.
    let (base, rx) = stub_empty_200();
    let svc = qbit_service(qbit_config_no_auth(&base));
    let config = qbit_config_no_auth(&base);
    let out = super::pause(&svc, &config, Some("abc123")).await.unwrap();
    let req = rx.recv().unwrap();
    assert!(
        req.request_line.starts_with("POST /api/v2/torrents/stop "),
        "pause must POST /api/v2/torrents/stop, got: {}",
        req.request_line
    );
    assert!(
        req.body.contains("hashes=abc123"),
        "hashes form field on the wire: {}",
        req.body
    );
    // Fix 4: returns a `submitted` envelope, not a bare {ok:true}.
    assert_eq!(out["submitted"], json!(true));
    assert!(out.get("note").is_some(), "note pointing at queue: {out}");
}

#[tokio::test]
async fn resume_hits_torrents_start_on_the_wire() {
    // FACT (bead, HIGH): qBittorrent v5 renamed resume->start. Prove the REAL
    // method POSTs /api/v2/torrents/start.
    let (base, rx) = stub_empty_200();
    let svc = qbit_service(qbit_config_no_auth(&base));
    let config = qbit_config_no_auth(&base);
    let out = super::resume(&svc, &config, None).await.unwrap();
    let req = rx.recv().unwrap();
    assert!(
        req.request_line.starts_with("POST /api/v2/torrents/start "),
        "resume must POST /api/v2/torrents/start, got: {}",
        req.request_line
    );
    // No id -> hashes=all.
    assert!(
        req.body.contains("hashes=all"),
        "default resume targets all: {}",
        req.body
    );
    assert_eq!(out["submitted"], json!(true));
}

#[tokio::test]
async fn remove_hits_torrents_delete_with_delete_files_flag() {
    let (base, rx) = stub_empty_200();
    let svc = qbit_service(qbit_config_no_auth(&base));
    let config = qbit_config_no_auth(&base);
    let _ = super::remove(&svc, &config, "abc123", true).await.unwrap();
    let req = rx.recv().unwrap();
    assert!(
        req.request_line
            .starts_with("POST /api/v2/torrents/delete "),
        "remove must POST /api/v2/torrents/delete, got: {}",
        req.request_line
    );
    assert!(req.body.contains("hashes=abc123"), "{}", req.body);
    assert!(req.body.contains("deleteFiles=true"), "{}", req.body);
}

#[test]
fn qbit_pause_resume_paths_are_v5_stop_start_not_pause_resume() {
    // Path-shape guard kept alongside the on-the-wire tests.
    let config = qbit_config();
    assert_eq!(
        qbit_path(&config, "/torrents/stop"),
        "/api/v2/torrents/stop"
    );
    assert_eq!(
        qbit_path(&config, "/torrents/start"),
        "/api/v2/torrents/start"
    );
}

#[test]
fn queue_slim_keeps_torrent_fields() {
    let raw = json!([{
        "hash": "abc123",
        "name": "Ubuntu.iso",
        "state": "downloading",
        "progress": 0.42,
        "dlspeed": 1048576,
        "size": 734003200_i64,
        "category": "linux",
        "tracker": "drop me",
        "magnet_uri": "drop me too"
    }]);
    let slimmed = slim(raw, TORRENT_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["hash"], "abc123");
    assert_eq!(row["name"], "Ubuntu.iso");
    assert_eq!(row["state"], "downloading");
    assert_eq!(row["progress"], 0.42);
    assert_eq!(row["category"], "linux");
    assert!(row.get("tracker").is_none(), "bulky fields dropped");
    assert!(row.get("magnet_uri").is_none());
}
