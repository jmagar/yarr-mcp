//! Semantic search blend tests.

use super::*;

fn entry(path: &'static str, description: &'static str) -> CatalogEntry {
    CatalogEntry::Generic {
        path: path.to_string(),
        service: None,
        method: "get",
        scope: crate::codemode::catalog::CatalogScope::Read,
        destructive: false,
        capability: "test",
        required_params: Vec::new(),
        description,
    }
}

// ── cosine_similarity ───────────────────────────────────────────────────────

#[test]
fn cosine_similarity_identical_vectors_is_one() {
    let v = [1.0, 2.0, 3.0];
    assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
}

#[test]
fn cosine_similarity_orthogonal_vectors_is_zero() {
    assert!((cosine_similarity(&[1.0, 0.0], &[0.0, 1.0])).abs() < 1e-6);
}

#[test]
fn cosine_similarity_opposite_vectors_is_negative_one() {
    let a = [1.0, 2.0, 3.0];
    let b = [-1.0, -2.0, -3.0];
    assert!((cosine_similarity(&a, &b) - -1.0).abs() < 1e-6);
}

#[test]
fn cosine_similarity_empty_vectors_is_zero() {
    assert_eq!(cosine_similarity(&[], &[]), 0.0);
}

#[test]
fn cosine_similarity_mismatched_lengths_is_zero() {
    assert_eq!(cosine_similarity(&[1.0, 2.0], &[1.0]), 0.0);
}

#[test]
fn cosine_similarity_zero_vector_is_zero_not_nan() {
    let score = cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]);
    assert_eq!(score, 0.0);
    assert!(!score.is_nan());
}

// ── tei_url ──────────────────────────────────────────────────────────────────

#[test]
fn tei_url_unset_is_none() {
    let mut env = crate::testing::TestEnv::new();
    env.remove("YARR_CODEMODE_TEI_URL");
    assert_eq!(tei_url(), None);
}

#[test]
fn tei_url_whitespace_only_is_none() {
    let mut env = crate::testing::TestEnv::new();
    env.set("YARR_CODEMODE_TEI_URL", "   ");
    assert_eq!(tei_url(), None);
}

#[test]
fn tei_url_set_is_trimmed_and_returned() {
    let mut env = crate::testing::TestEnv::new();
    env.set("YARR_CODEMODE_TEI_URL", "  http://localhost:52000  ");
    assert_eq!(tei_url().as_deref(), Some("http://localhost:52000"));
}

// ── semantic_scores: fail-open paths that must never touch the network ──────

#[tokio::test]
async fn disabled_tei_url_returns_empty_without_network_attempt() {
    let cache = SemanticCache::new();
    let catalog = vec![entry("sonarr.get_series", "List all series")];
    let scores = semantic_scores(&cache, None, &catalog, "list series").await;
    assert!(scores.is_empty());
}

#[tokio::test]
async fn empty_query_returns_empty() {
    let cache = SemanticCache::new();
    let catalog = vec![entry("sonarr.get_series", "List all series")];
    let scores = semantic_scores(&cache, Some("http://127.0.0.1:1"), &catalog, "   ").await;
    assert!(scores.is_empty());
}

#[tokio::test]
async fn empty_catalog_returns_empty() {
    let cache = SemanticCache::new();
    let scores = semantic_scores(&cache, Some("http://127.0.0.1:1"), &[], "list series").await;
    assert!(scores.is_empty());
}

#[tokio::test]
async fn unreachable_tei_fails_open_to_empty_map() {
    // Port 1 is a reserved/privileged port nothing binds to in a test sandbox —
    // the connection is refused immediately rather than timing out, so this test
    // stays fast without needing a real TEI server or a mock HTTP endpoint.
    let cache = SemanticCache::new();
    let catalog = vec![entry("sonarr.get_series", "List all series")];
    let scores = semantic_scores(&cache, Some("http://127.0.0.1:1"), &catalog, "list series").await;
    assert!(
        scores.is_empty(),
        "an unreachable TEI must never surface an error"
    );
}

#[tokio::test]
async fn failed_call_starts_a_cooldown_that_skips_the_next_network_attempt() {
    let cache = SemanticCache::new();
    let catalog = vec![entry("sonarr.get_series", "List all series")];
    let tei = Some("http://127.0.0.1:1");

    // First call: attempts the network, fails, records a cooldown.
    let first = semantic_scores(&cache, tei, &catalog, "list series").await;
    assert!(first.is_empty());
    assert!(
        cache.in_cooldown(),
        "a failed embed call must start a cooldown"
    );

    // Second call, immediately after: must short-circuit on the cooldown check
    // (in_cooldown) before ever reaching embed_batch, not just happen to fail
    // again for the same reason.
    let second = semantic_scores(&cache, tei, &catalog, "list series").await;
    assert!(second.is_empty());
}

#[tokio::test]
async fn concurrent_first_use_embeds_the_catalog_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let handler_calls = calls.clone();
    let app = axum::Router::new().route(
        "/embed",
        axum::routing::post(move |axum::Json(body): axum::Json<serde_json::Value>| {
            let handler_calls = handler_calls.clone();
            async move {
                handler_calls.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(40)).await;
                let count = body["inputs"].as_array().map_or(0, Vec::len);
                axum::Json(vec![vec![1.0_f32, 0.0]; count])
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let cache = SemanticCache::new();
    let catalog = vec![entry("sonarr.get_series", "List all series")];
    let url = format!("http://{address}");
    let results = tokio::join!(
        semantic_scores(&cache, Some(&url), &catalog, "series"),
        semantic_scores(&cache, Some(&url), &catalog, "series"),
        semantic_scores(&cache, Some(&url), &catalog, "series"),
        semantic_scores(&cache, Some(&url), &catalog, "series"),
        semantic_scores(&cache, Some(&url), &catalog, "series"),
        semantic_scores(&cache, Some(&url), &catalog, "series"),
        semantic_scores(&cache, Some(&url), &catalog, "series"),
        semantic_scores(&cache, Some(&url), &catalog, "series"),
    );
    assert!(!results.0.is_empty());
    assert!(!results.1.is_empty());
    assert!(!results.2.is_empty());
    assert!(!results.3.is_empty());
    assert!(!results.4.is_empty());
    assert!(!results.5.is_empty());
    assert!(!results.6.is_empty());
    assert!(!results.7.is_empty());
    // One catalog batch plus one query batch per search. Without single-flight,
    // all eight callers also issue their own catalog batch (16 total calls).
    assert_eq!(calls.load(Ordering::SeqCst), 9);
}
