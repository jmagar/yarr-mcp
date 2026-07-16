//! Semantic search blend for `codemode.search()`.
//!
//! Lexical substring matching (`codemode.search`'s JS body, in
//! [`super::proxy`]) misses synonym queries entirely — a query like "roster of
//! saved queues" shares no tokens with the catalog entry it should match, so it
//! either returns nothing or, worse, a false-positive hit from a short token
//! (like "of") landing inside an unrelated word. This module supplies an
//! optional per-path similarity score, computed by embedding the query and the
//! catalog against a TEI (Text Embeddings Inference) server, that the JS side
//! blends into its existing lexical score.
//!
//! Design constraints (all load-bearing, not incidental):
//!
//! - **Fail-open, always.** [`semantic_scores`] never returns an `Err` and
//!   never panics on a reachability/response problem — it returns an empty map,
//!   and `codemode.search()` falls through to exactly its lexical-only ranking.
//!   A TEI outage must never break, slow-fail loudly, or change the shape of a
//!   script's `codemode.search()` call.
//! - **Disabled by default.** With `YARR_CODEMODE_TEI_URL` unset, no network
//!   call is ever attempted — see [`tei_url`].
//! - **Computed lazily, cached in memory, never at build time.** The catalog
//!   embeddings are computed from the *live* catalog on first use and cached for
//!   the process's lifetime, so they can never drift from what's actually being
//!   served (a build-time-baked cache could go stale if the catalog changes
//!   without someone remembering to regenerate it).
//! - **Bounded retry, not give-up-once.** A TEI failure starts a cooldown
//!   ([`COOLDOWN`]); calls during the cooldown skip the network entirely and
//!   return an empty map immediately. Once the cooldown elapses, the next call
//!   retries — so a TEI restart is picked up automatically, no yarr restart
//!   required, while a flapping/down TEI can't be hammered every search call.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::catalog::CatalogEntry;

type CatalogVectors = Arc<[(String, Vec<f32>)]>;

/// How long a TEI failure suppresses further attempts. Long enough that a
/// flapping/restarting TEI container isn't hammered on every `codemode.search`
/// call (searches can happen many times per script); short enough that recovery
/// is picked up within one typical agent working session, no restart needed.
const COOLDOWN: Duration = Duration::from_secs(30);

/// HTTP timeout for a single `/embed` call (catalog batch or query).
const EMBED_TIMEOUT: Duration = Duration::from_secs(5);

/// `YARR_CODEMODE_TEI_URL` — base URL of a TEI (Text Embeddings Inference)
/// server, e.g. `http://localhost:52000`. Unset (the default) disables semantic
/// search entirely: [`semantic_scores`] short-circuits before any network call.
pub fn tei_url() -> Option<String> {
    std::env::var("YARR_CODEMODE_TEI_URL")
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

/// Process-lifetime cache: the catalog's embedded vectors (computed once, on
/// first use) and the cooldown clock. Held on `YarrService` behind an `Arc`
/// so it's shared across every `codemode.search()` call for the life of the
/// server, not recomputed per script run (a script rebuilds the catalog *value*
/// fresh every run via `build_catalog`, but the embeddings for an unchanged
/// catalog are the same work redone — the cache exists precisely to not repeat
/// it).
pub struct SemanticCache {
    client: reqwest::Client,
    state: Mutex<CacheState>,
    /// Serializes the first catalog embedding request. The state mutex is never
    /// held across network I/O; concurrent cold callers wait here and re-check
    /// the cache after the winner completes.
    initialize: tokio::sync::Mutex<()>,
}

#[derive(Default)]
struct CacheState {
    /// `(path, vector)` pairs for the catalog, once successfully embedded.
    catalog_vectors: Option<CatalogVectors>,
    /// Set on any TEI failure; cleared once it elapses.
    cooldown_until: Option<Instant>,
}

impl SemanticCache {
    pub fn new() -> Self {
        Self {
            // A dedicated client, not YarrService's upstream one: TEI needs no
            // per-service auth headers, and keeping it separate means a change to
            // upstream service auth can never accidentally leak into TEI requests
            // (or vice versa).
            client: reqwest::Client::builder()
                .timeout(EMBED_TIMEOUT)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            state: Mutex::new(CacheState::default()),
            initialize: tokio::sync::Mutex::new(()),
        }
    }

    fn in_cooldown(&self) -> bool {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state
            .cooldown_until
            .is_some_and(|until| Instant::now() < until)
    }

    fn record_failure(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.cooldown_until = Some(Instant::now() + COOLDOWN);
    }

    fn cached_catalog_vectors(&self) -> Option<CatalogVectors> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.catalog_vectors.clone()
    }

    fn store_catalog_vectors(&self, vectors: CatalogVectors) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.catalog_vectors = Some(vectors);
    }
}

impl Default for SemanticCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Return `{ path: similarity }` for every catalog entry, ranking `query`
/// against the (cached, lazily-computed) catalog embeddings. Empty map when
/// semantic search is disabled (`tei_url` is `None`), cooling down after a
/// recent failure, or the embedding call itself fails — every one of those is a
/// silent no-op from the caller's perspective, by design (see module docs).
pub async fn semantic_scores(
    cache: &SemanticCache,
    tei_url: Option<&str>,
    catalog: &[CatalogEntry],
    query: &str,
) -> HashMap<String, f32> {
    let Some(tei_url) = tei_url else {
        return HashMap::new();
    };
    if query.trim().is_empty() || catalog.is_empty() || cache.in_cooldown() {
        return HashMap::new();
    }

    let catalog_vectors = match ensure_catalog_vectors(cache, tei_url, catalog).await {
        Some(vectors) => vectors,
        // Failure already recorded a cooldown inside ensure_catalog_vectors.
        None => return HashMap::new(),
    };

    let query_vector = match embed_batch(
        &cache.client,
        tei_url,
        std::slice::from_ref(&query.to_owned()),
    )
    .await
    {
        Ok(mut vectors) if vectors.len() == 1 => normalize_vector(vectors.remove(0)),
        _ => {
            cache.record_failure();
            return HashMap::new();
        }
    };

    catalog_vectors
        .iter()
        .map(|(path, vector)| {
            let score = dot_similarity(&query_vector, vector);
            (path.clone(), score)
        })
        .collect()
}

/// The catalog's embedded vectors, computing and caching them on first use.
/// Returns `None` (having already recorded a cooldown) if embedding fails.
async fn ensure_catalog_vectors(
    cache: &SemanticCache,
    tei_url: &str,
    catalog: &[CatalogEntry],
) -> Option<Arc<[(String, Vec<f32>)]>> {
    if let Some(cached) = cache.cached_catalog_vectors() {
        return Some(cached);
    }
    let _initialize = cache.initialize.lock().await;
    if let Some(cached) = cache.cached_catalog_vectors() {
        return Some(cached);
    }
    let paths: Vec<String> = catalog
        .iter()
        .map(|entry| entry.path().to_owned())
        .collect();
    let descriptions: Vec<String> = catalog
        .iter()
        .map(|entry| entry.description().to_owned())
        .collect();

    match embed_batch(&cache.client, tei_url, &descriptions).await {
        Ok(vectors) if vectors.len() == paths.len() => {
            let pairs: Arc<[(String, Vec<f32>)]> = paths
                .into_iter()
                .zip(vectors.into_iter().map(normalize_vector))
                .collect::<Vec<_>>()
                .into();
            cache.store_catalog_vectors(pairs.clone());
            Some(pairs)
        }
        _ => {
            cache.record_failure();
            None
        }
    }
}

/// One batched `POST {tei_url}/embed` call: `{"inputs": texts}` in, one vector
/// per input (in input order) out.
async fn embed_batch(
    client: &reqwest::Client,
    tei_url: &str,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, String> {
    let url = format!("{}/embed", tei_url.trim_end_matches('/'));
    let response = client
        .post(&url)
        .json(&serde_json::json!({ "inputs": texts }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("TEI {} responded {}", url, response.status()));
    }
    response
        .json::<Vec<Vec<f32>>>()
        .await
        .map_err(|e| format!("TEI {url} response was not the expected shape: {e}"))
}

/// Cosine similarity in `[-1.0, 1.0]`; `0.0` for mismatched/zero-length/zero-norm
/// vectors rather than panicking or dividing by zero — a defensive fallback,
/// not an expected path (TEI always returns fixed-dimension, non-zero vectors
/// for non-empty text).
#[cfg(test)]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

fn normalize_vector(mut vector: Vec<f32>) -> Vec<f32> {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

fn dot_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b).map(|(left, right)| left * right).sum()
}

#[cfg(test)]
#[path = "semantic_tests.rs"]
mod tests;
