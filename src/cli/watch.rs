//! `rustarr watch` — health monitor for the MCP HTTP server.
//!
//! Polls the server health endpoint on a fixed interval and emits a single stdout line
//! whenever the server state changes. Stdout is the event stream; the plugin
//! monitor runtime delivers each line to Claude as a notification.
//!
//! States:
//!   UP       — /health returned 2xx
//!   DEGRADED — /health returned a non-2xx HTTP response
//!   DOWN     — connection refused / timeout / DNS failure
//!
//! Only state *changes* produce output, so Claude isn't spammed while the
//! server is stable. The initial state always emits one line.

use std::time::{Duration, Instant};

use anyhow::Result;

/// Server reachability state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerState {
    Up,
    Degraded(u16),
    Down,
}

impl std::fmt::Display for ServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerState::Up => write!(f, "UP"),
            ServerState::Degraded(code) => write!(f, "DEGRADED(HTTP {code})"),
            ServerState::Down => write!(f, "DOWN"),
        }
    }
}

/// Run the health watch loop. Exits only on CTRL+C or fatal error.
///
/// Each state change emits exactly one line to stdout. The first poll always
/// emits regardless of state. All other output goes to stderr so it doesn't
/// pollute the event stream.
pub async fn run_watch(base_url: &str, interval_secs: u64) -> Result<()> {
    if interval_secs == 0 {
        return Err(anyhow::anyhow!("interval must be at least 1 second"));
    }
    let health_url = health_url_for(base_url);
    let interval = Duration::from_secs(interval_secs);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    eprintln!(
        "[rustarr watch] polling {} every {}s — emitting stdout on state change",
        health_url, interval_secs
    );

    let mut last_state: Option<ServerState> = None;
    let mut state_since = Instant::now();

    loop {
        let current = probe(&client, &health_url).await;
        let changed = last_state.map(|s| s != current).unwrap_or(true);

        if changed {
            let line = format_event(
                base_url,
                &current,
                last_state,
                state_since.elapsed(),
                interval_secs,
            );
            println!("{line}");
            state_since = Instant::now();
            last_state = Some(current);
        }

        tokio::time::sleep(interval).await;
    }
}

fn health_url_for(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/health") {
        trimmed.to_owned()
    } else {
        format!("{trimmed}/health")
    }
}

/// Probe the health endpoint and classify the result.
async fn probe(client: &reqwest::Client, url: &str) -> ServerState {
    match client.get(url).send().await {
        Ok(r) if r.status().is_success() => ServerState::Up,
        Ok(r) => ServerState::Degraded(r.status().as_u16()),
        Err(e) => {
            // Timeouts and connection refusals are expected transient DOWN states.
            // Anything else (TLS error, DNS failure, etc.) likely indicates a
            // configuration problem — log it to stderr so the operator can diagnose.
            if !e.is_timeout() && !e.is_connect() {
                eprintln!("[rustarr watch] probe error (may indicate config issue): {e}");
            }
            ServerState::Down
        }
    }
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;

/// Format a state-change event line for the monitor stream.
fn format_event(
    base_url: &str,
    current: &ServerState,
    prev: Option<ServerState>,
    prev_duration: Duration,
    interval_secs: u64,
) -> String {
    match (current, prev) {
        // Recovery — bind prev_state so we avoid unwrap and display it cleanly.
        (ServerState::Up, Some(prev_state @ (ServerState::Down | ServerState::Degraded(_)))) => {
            format!(
                "[rustarr] UP — {} recovered after {}s (was {})",
                base_url,
                prev_duration.as_secs(),
                prev_state,
            )
        }
        // Initial healthy state
        (ServerState::Up, _) => format!("[rustarr] UP — {} is healthy", base_url),
        // Went down
        (ServerState::Down, _) => format!(
            "[rustarr] DOWN — {} is unreachable (retrying every {}s)",
            base_url, interval_secs
        ),
        // Degraded (non-2xx)
        (ServerState::Degraded(code), _) => {
            format!("[rustarr] DEGRADED — {} returned HTTP {code}", base_url)
        }
    }
}
