//! Process-local defense-in-depth for OAuth token signing.
//!
//! `lab-auth` currently signs RS256 tokens through the RustCrypto RSA stack,
//! which carries RUSTSEC-2023-0071. Authorization-code and refresh-token checks
//! already prevent arbitrary signing, and this limiter additionally bounds the
//! remotely observable signing rate while the upstream Ed25519 migration is in
//! progress. Reverse proxies should enforce a per-client limit as well.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::Request;
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

const TOKEN_ISSUANCE_LIMIT: usize = 30;
const TOKEN_ISSUANCE_WINDOW: Duration = Duration::from_secs(60);

pub(crate) struct TokenBucket {
    attempts: VecDeque<Instant>,
}

impl TokenBucket {
    fn new() -> Self {
        Self {
            attempts: VecDeque::with_capacity(TOKEN_ISSUANCE_LIMIT),
        }
    }

    fn admit(&mut self, now: Instant) -> bool {
        while self
            .attempts
            .front()
            .is_some_and(|attempt| now.saturating_duration_since(*attempt) >= TOKEN_ISSUANCE_WINDOW)
        {
            self.attempts.pop_front();
        }
        if self.attempts.len() >= TOKEN_ISSUANCE_LIMIT {
            return false;
        }
        self.attempts.push_back(now);
        true
    }
}

pub(crate) type TokenLimiter = Arc<Mutex<TokenBucket>>;

pub(crate) fn new_limiter() -> TokenLimiter {
    Arc::new(Mutex::new(TokenBucket::new()))
}

pub(crate) async fn enforce_token_rate_limit(
    axum::extract::State(limiter): axum::extract::State<TokenLimiter>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() == Method::POST && request.uri().path() == "/token" {
        let admitted = limiter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .admit(Instant::now());
        let outcome = if admitted { "admitted" } else { "rate_limited" };
        axum_prometheus::metrics::counter!(
            "yarr_auth_token_issuance_total",
            "outcome" => outcome
        )
        .increment(1);
        if !admitted {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, "60")],
                axum::Json(serde_json::json!({
                    "error": "temporarily_unavailable",
                    "error_description": "token issuance rate limit exceeded"
                })),
            )
                .into_response();
        }
    }
    next.run(request).await
}

#[cfg(test)]
#[path = "token_rate_limit_tests.rs"]
mod tests;
