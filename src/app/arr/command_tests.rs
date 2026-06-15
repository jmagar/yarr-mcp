//! Tests for the async `/command`-intent methods (search/refresh) — the split-out
//! counterpart to `write_tests.rs`. These cover the dry-run/capability guards and,
//! crucially, the P2-6 bounded-concurrency fan-out for Sonarr multi-id search: the
//! test pins that EXACTLY N POSTs hit the wire and that the aggregated jobs/count
//! response shape is preserved, so the concurrency refactor stays behaviour-equivalent.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::app::RustarrService;
use crate::config::{RustarrConfig, ServiceConfig, ServiceKind};
use crate::rustarr::RustarrClient;

fn service_with(kind: ServiceKind, name: &str, base_url: &str) -> RustarrService {
    let config = RustarrConfig {
        services: vec![ServiceConfig {
            name: name.into(),
            kind,
            base_url: base_url.into(),
            api_key: Some("x".into()),
            ..ServiceConfig::default()
        }],
    };
    let client = RustarrClient::new(&config).expect("client builds");
    RustarrService::new(client, config)
}

#[tokio::test]
async fn search_dry_run_builds_preview_without_network() {
    // Unreachable base — a dry-run must not touch it.
    let svc = service_with(ServiceKind::Sonarr, "sonarr", "http://127.0.0.1:1");
    let preview = svc
        .arr_search("sonarr", &[1, 2, 3], false, false)
        .await
        .expect("search dry-run builds a preview without any network call");
    assert_eq!(preview["would_do"], serde_json::json!("search"));
    assert_eq!(preview["command"], serde_json::json!("SeriesSearch"));
    assert_eq!(preview["count"], serde_json::json!(3));
    assert!(preview.get("started").is_none());
}

#[tokio::test]
async fn refresh_rejects_wrong_capability_before_network() {
    let svc = service_with(ServiceKind::Plex, "plex", "http://127.0.0.1:1");
    assert!(svc.arr_refresh("plex", &[], false, false).await.is_err());
}

/// Multi-connection stub `/command` server: accepts up to `expected` POSTs,
/// replies to each with `{"id": <n>}`, and counts how many requests it served.
/// Returns `(base_url, counter)`; the spawned thread terminates after `expected`
/// connections so the test does not leak it.
fn stub_command_server(expected: usize) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_thread = counter.clone();
    std::thread::spawn(move || {
        for _ in 0..expected {
            let (mut stream, _) = match listener.accept() {
                Ok(c) => c,
                Err(_) => break,
            };
            let n = counter_thread.fetch_add(1, Ordering::SeqCst);
            // Drain the request headers + a little body, then respond. Each POST is
            // a tiny JSON body; we don't need to parse it for the count assertion.
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line == "\r\n" || line == "\n" => break,
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            let _ = reader
                .get_mut()
                .set_read_timeout(Some(std::time::Duration::from_millis(20)));
            let mut sink = [0_u8; 256];
            let _ = reader.get_mut().read(&mut sink);
            let body = format!("{{\"id\":{}}}", n + 1000);
            let _ = write!(
                stream,
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.flush();
        }
    });
    (base, counter)
}

#[tokio::test]
async fn sonarr_multi_id_search_posts_once_per_id_and_aggregates() {
    // Sonarr has no plural /command form, so a 3-id search fans out to exactly 3
    // POSTs (run with bounded concurrency, P2-6). Assert the wire count AND the
    // aggregated response shape: {started, command, async, count, jobs:[...]}.
    let ids = [10_i64, 20, 30];
    let (base, counter) = stub_command_server(ids.len());
    let svc = service_with(ServiceKind::Sonarr, "sonarr", &base);

    let out = svc
        .arr_search("sonarr", &ids, true, false)
        .await
        .expect("multi-id search applies");

    // Exactly N POSTs reached the wire — one per id.
    assert_eq!(counter.load(Ordering::SeqCst), ids.len(), "one POST per id");

    // Aggregated jobs/count shape is preserved exactly.
    assert_eq!(out["started"], serde_json::json!("search"));
    assert_eq!(out["command"], serde_json::json!("SeriesSearch"));
    assert_eq!(out["async"], serde_json::json!(true));
    assert_eq!(out["count"], serde_json::json!(ids.len()));
    let jobs = out["jobs"].as_array().expect("jobs is an array");
    assert_eq!(jobs.len(), ids.len(), "one job per id, in input order");
    // Every job id is a number echoed by the stub (>= 1000); order is preserved by
    // the index-tagged fan-out, so all three slots are populated.
    for job in jobs {
        assert!(
            job.as_i64().is_some_and(|n| n >= 1000),
            "job id present: {job}"
        );
    }
}
