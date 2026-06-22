//! Tests for the C2 write command METHODS (orchestration). The pure builders /
//! selectors / cap / preview are tested directly in `editor_tests.rs`; here we
//! assert the methods reject the wrong capability before any network call and
//! that the headline `set_quality` request struct threads through unchanged.

use crate::app::RustarrService;
use crate::app::arr::write::SetQualityRequest;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::RustarrClient;

fn service_with(kind: ServiceKind, name: &str) -> RustarrService {
    let config = RustarrConfig {
        services: vec![ServiceConfig {
            name: name.into(),
            kind,
            base_url: "http://127.0.0.1:1".into(),
            api_key: Some("x".into()),
            ..ServiceConfig::default()
        }],
    };
    let client = RustarrClient::new(&config).expect("client builds");
    RustarrService::new(client, config)
}

#[tokio::test]
async fn write_methods_reject_wrong_capability_before_network() {
    // Plex is a MediaServer: every arr write method must reject it via the
    // capability guard, synchronously, before any request is built.
    let svc = service_with(ServiceKind::Plex, "plex");
    let req = SetQualityRequest {
        from: None,
        to: "HD-1080p",
        ids: &[],
        titles: &[],
        bulk: false,
    };
    let err = svc
        .arr_set_quality("plex", req)
        .await
        .expect_err("plex is not an arr kind");
    assert!(
        err.to_string().contains("does not provide") || err.to_string().contains("ArrManager"),
        "{err}"
    );
    assert!(svc.arr_search("plex", &[], false).await.is_err());
    assert!(svc.arr_delete("plex", 1, false, true).await.is_err());
}

#[tokio::test]
async fn delete_without_confirm_returns_preview_and_mutates_nothing() {
    // delete mutates upstream state: with confirm absent it returns a preview WITHOUT
    // issuing the DELETE — so it never touches the (unreachable) stub at all.
    let svc = service_with(ServiceKind::Radarr, "radarr");
    let preview = svc
        .arr_delete("radarr", 9, true, false)
        .await
        .expect("delete dry-run builds a preview without any network call");
    assert_eq!(preview["would_do"], serde_json::json!("delete"));
    assert_eq!(preview["id"], serde_json::json!(9));
    assert_eq!(preview["delete_files"], serde_json::json!(true));
    assert_eq!(preview["mutating"], serde_json::json!(true));
    assert_eq!(preview["confirm_required"], serde_json::json!(true));
    assert!(
        preview.get("deleted").is_none(),
        "preview must not report a delete"
    );
}

/// One-shot TCP stub serving a JSON resource array of `row_count` items.
fn stub_resource_array(row_count: usize) -> String {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" || line == "\n" {
                break;
            }
        }
        let mut sink = [0_u8; 64];
        let _ = reader
            .get_mut()
            .set_read_timeout(Some(std::time::Duration::from_millis(20)));
        let _ = reader.get_mut().read(&mut sink);
        let rows: Vec<serde_json::Value> = (0..row_count)
            .map(|i| serde_json::json!({ "id": i, "title": format!("t{i}") }))
            .collect();
        let body = serde_json::to_string(&rows).unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let _ = stream.flush();
    });
    base
}

fn sonarr_at(base_url: &str) -> RustarrService {
    let config = RustarrConfig {
        services: vec![ServiceConfig {
            name: "sonarr".into(),
            kind: ServiceKind::Sonarr,
            base_url: base_url.into(),
            api_key: Some("x".into()),
            ..ServiceConfig::default()
        }],
    };
    let client = RustarrClient::new(&config).expect("client builds");
    RustarrService::new(client, config)
}

#[tokio::test]
async fn whole_library_search_apply_enforces_cap() {
    // Empty ids must count the real library and refuse > MAX_BULK (100) items
    // without bulk=true — the whole-library path can no longer bypass the cap by
    // passing 0 to guard_count.
    let base = stub_resource_array(101);
    let svc = sonarr_at(&base);
    let err = svc
        .arr_search("sonarr", &[], false)
        .await
        .expect_err("101 items without bulk must be refused");
    assert!(
        err.to_string().contains("refusing to act on 101 items"),
        "{err}"
    );
}

/// Multi-connection stub for the `set_quality` apply flow: routes by request path
/// to serve the qualityprofile list, the resource rows, and the editor PUT echo,
/// and captures the editor PUT (request line + body) on the returned channel.
/// Sends `Connection: close` so reqwest opens a fresh connection per request
/// (defeating pool reuse) — the three sequential calls map to three accepts.
fn stub_set_quality_flow() -> (String, std::sync::mpsc::Receiver<String>) {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = match listener.accept() {
                Ok(c) => c,
                Err(_) => break,
            };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            let _ = reader.read_line(&mut request_line);
            let mut content_length = 0usize;
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line == "\r\n" || line == "\n" => break,
                    Ok(_) => {
                        if let Some(rest) =
                            line.to_ascii_lowercase().strip_prefix("content-length:")
                        {
                            content_length = rest.trim().parse().unwrap_or(0);
                        }
                    }
                    Err(_) => break,
                }
            }
            let mut body = vec![0_u8; content_length];
            let _ = reader.read_exact(&mut body);
            let body = String::from_utf8_lossy(&body).to_string();

            // Route by path. `qualityprofile` must be checked before the bare
            // resource path; `/editor` is the PUT we capture.
            let resp_body = if request_line.contains("qualityprofile") {
                r#"[{"id":4,"name":"HD-1080p"}]"#.to_string()
            } else if request_line.contains("/editor") {
                let _ = tx.send(format!("{}\n{}", request_line.trim(), body));
                r#"[{"id":1}]"#.to_string()
            } else {
                r#"[{"id":1,"title":"Show","qualityProfileId":2}]"#.to_string()
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{resp_body}",
                resp_body.len()
            );
            let _ = stream.flush();
        }
    });
    (base, rx)
}

#[tokio::test]
async fn set_quality_apply_puts_editor_and_summarizes() {
    // set_quality is non-destructive now and applies immediately (no preview): it
    // resolves the target profile name, selects the items, and PUTs /<res>/editor.
    // Assert the editor PUT carries the resolved ids + profile id and the summary
    // reports the upstream-confirmed change.
    let (base, rx) = stub_set_quality_flow();
    let svc = sonarr_at(&base);
    let out = svc
        .arr_set_quality(
            "sonarr",
            SetQualityRequest {
                from: None,
                to: "HD-1080p",
                ids: &[1],
                titles: &[],
                bulk: false,
            },
        )
        .await
        .expect("set_quality applies");

    let editor = rx
        .recv()
        .expect("editor PUT should reach the wire (apply path, not a preview)");
    assert!(editor.contains("PUT "), "must be a PUT: {editor}");
    assert!(editor.contains("/editor"), "must hit /editor: {editor}");
    assert!(
        editor.contains("seriesIds"),
        "body carries seriesIds: {editor}"
    );
    assert!(
        editor.contains("\"qualityProfileId\":4"),
        "body carries the resolved target profile id: {editor}"
    );

    assert_eq!(out["attempted"], serde_json::json!(1));
    assert_eq!(out["confirmed"], serde_json::json!(true));
    assert_eq!(out["to"], serde_json::json!("HD-1080p"));
}
