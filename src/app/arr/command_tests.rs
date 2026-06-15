//! Tests for the async `/command`-intent methods (search/refresh) — the split-out
//! counterpart to `write_tests.rs`. These cover the dry-run/capability guards and,
//! crucially, the P2-6 bounded-concurrency fan-out for Sonarr multi-id search: the
//! test pins that EXACTLY N POSTs hit the wire and that the aggregated jobs/count
//! response shape is preserved, so the concurrency refactor stays behaviour-equivalent.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// Last contiguous run of digits in `body` — for Sonarr the POSTed
/// `{"name":"SeriesSearch","seriesId":<id>}` ends in the resource id, so this
/// recovers the id the stub was asked about.
fn last_int(body: &str) -> i64 {
    let mut last = String::new();
    let mut cur = String::new();
    for c in body.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            last = std::mem::take(&mut cur);
        }
    }
    if !cur.is_empty() {
        last = cur;
    }
    last.parse().unwrap_or(0)
}

/// Multi-connection stub `/command` server. Accepts all `expected` POSTs, parses
/// the resource id each one carries, then **responds in reverse-id order** so the
/// completion order deliberately differs from the input order — the client's
/// index-tagged fan-out must re-sort the jobs back into caller-id order. Each
/// reply echoes `{"id": <the id it was sent>}`. Returns `(base_url, counter)`;
/// the thread terminates after `expected` connections so the test doesn't leak it.
fn stub_command_server(expected: usize) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_thread = counter.clone();
    std::thread::spawn(move || {
        let mut conns: Vec<(std::net::TcpStream, i64)> = Vec::new();
        for _ in 0..expected {
            let (stream, _) = match listener.accept() {
                Ok(c) => c,
                Err(_) => break,
            };
            counter_thread.fetch_add(1, Ordering::SeqCst);
            let mut reader = BufReader::new(stream.try_clone().unwrap());
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
            let echoed = last_int(&String::from_utf8_lossy(&body));
            conns.push((stream, echoed));
        }
        // Reverse-id order: completion order != input order, so a broken impl that
        // returned jobs in completion order would fail the input-order assertion.
        conns.sort_by_key(|(_, id)| std::cmp::Reverse(*id));
        for (mut stream, echoed) in conns {
            let body = format!("{{\"id\":{echoed}}}");
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
async fn sonarr_multi_id_search_posts_once_per_id_and_aggregates_in_order() {
    // Sonarr has no plural /command form, so a 3-id search fans out to exactly 3
    // POSTs (run with bounded concurrency, P2-6). Assert the wire count, the
    // aggregated response shape {started, command, async, count, jobs:[...]}, AND
    // that jobs come back in caller-id order even though the stub responds in
    // reverse order (the index-tagged re-sort is the property under test).
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

    // Jobs are re-sorted into caller-id order despite reverse-order completion.
    let jobs: Vec<i64> = out["jobs"]
        .as_array()
        .expect("jobs is an array")
        .iter()
        .map(|j| j.as_i64().expect("job id is an integer"))
        .collect();
    assert_eq!(
        jobs,
        vec![10, 20, 30],
        "jobs must be in caller-id order regardless of upstream completion order"
    );
}
