//! Stats capability (Tautulli) curated commands (C8).
//!
//! Tautulli is the sole [`Capability::Stats`](crate::capability::Capability)
//! kind. Its API is a single GET endpoint `/api/v2?apikey=&cmd=NAME[&params]`
//! that wraps every result in `{response:{result, data, message}}` — there is no
//! REST resource surface. Each command therefore:
//!   1. resolves a Stats service (capability-checked — a non-tautulli kind is
//!      rejected before any request is built);
//!   2. builds the request through [`query_get`](crate::rustarr::query_get), which
//!      percent-encodes every param value (user text like `"x&cmd=delete"` cannot
//!      inject a second `cmd`, S6) AND injects the `apikey` exactly once via the
//!      shared query-auth path — so this module NEVER adds `apikey` itself (no
//!      double key, and the key never appears in `cmd`/path strings, which Tautulli
//!      logs to its access log);
//!   3. unwraps the envelope ([`unwrap_tautulli`]) — surfacing the upstream
//!      `message` as an error when `result != "success"` — and slims the payload to
//!      the fields agents need (AN-6 context budget).
//!
//! All four commands are READ scope (Tautulli is read-only stats); none mutate and
//! none are confirm-gated. Field-selection and the envelope/cmd shape are *business*
//! decisions and live here, never in a shim.

use anyhow::Result;
use serde_json::{Value, json};

use crate::app::RustarrService;
use crate::capability::Capability;
use crate::config::ServiceConfig;
use crate::rustarr::{query_get, slim};

/// Tautulli's single API base path. `cmd=` selects the command; auth (`apikey`)
/// is injected by `query_get`, never appended here.
const TAUTULLI_API: &str = "/api/v2";

/// Fields kept for a slimmed per-stream `session` (from `get_activity.sessions`):
/// enough to see who is watching what and how far along, without the very large
/// per-stream metadata blob.
const SESSION_FIELDS: &[&str] = &[
    "user",
    "full_title",
    "title",
    "state",
    "progress_percent",
    "media_type",
];

/// Fields kept for a slimmed `history` row (from `get_history.data`): enough to
/// reason about who watched what and when.
const HISTORY_FIELDS: &[&str] = &[
    "date",
    "user",
    "full_title",
    "title",
    "media_type",
    "watched_status",
    "percent_complete",
];

/// Fields kept for a slimmed `user` row (from `get_users`).
const USER_FIELDS: &[&str] = &["user_id", "username", "plays"];

/// Fields kept for a slimmed `library` row (from `get_libraries`).
const LIBRARY_FIELDS: &[&str] = &[
    "section_id",
    "section_name",
    "section_type",
    "count",
    "parent_count",
    "child_count",
];

/// Unwrap Tautulli's `{response:{result, data, message}}` envelope.
///
/// Tautulli returns HTTP 200 even on command failure, encoding the outcome in
/// `response.result`: `"success"` carries the payload in `response.data`, anything
/// else carries a human reason in `response.message`. This surfaces that reason as
/// an error rather than handing the caller an empty/error envelope.
fn unwrap_tautulli(value: Value) -> Result<Value> {
    let response = value
        .get("response")
        .ok_or_else(|| anyhow::anyhow!("tautulli response missing `response` envelope"))?;
    let result = response.get("result").and_then(Value::as_str).unwrap_or("");
    if result != "success" {
        let message = response
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        anyhow::bail!("tautulli command failed: {message}");
    }
    Ok(response.get("data").cloned().unwrap_or(Value::Null))
}

impl RustarrService {
    /// Resolve a Stats service and verify its capability. Central helper so every
    /// stats method shares one capability-checked resolution path; a non-tautulli
    /// kind is rejected here before any request is built.
    fn stats_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::Stats)
    }

    /// Run a Tautulli `cmd=NAME` GET with the given extra params, returning the
    /// unwrapped `response.data`. `apikey` is injected by `query_get`; never pass
    /// it in `extra`.
    async fn stats_cmd(
        &self,
        config: &ServiceConfig,
        command: &str,
        extra: &[(&str, &str)],
    ) -> Result<Value> {
        let mut params: Vec<(&str, &str)> = vec![("cmd", command)];
        params.extend_from_slice(extra);
        let url = query_get(config, TAUTULLI_API, &params)?;
        let raw = self.client_ref().send_get(config, url, None).await?;
        unwrap_tautulli(raw)
    }

    /// GET `?cmd=get_activity` → current streams. Slims to a stream count plus the
    /// per-stream essentials ([`SESSION_FIELDS`]). READ.
    pub async fn stats_activity(&self, service: &str) -> Result<Value> {
        let config = self.stats_context(service)?;
        let data = self.stats_cmd(config, "get_activity", &[]).await?;
        let stream_count = data.get("stream_count").cloned().unwrap_or(json!(0));
        let sessions = data
            .get("sessions")
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));
        Ok(json!({
            "stream_count": stream_count,
            "sessions": slim(sessions, SESSION_FIELDS),
        }))
    }

    /// GET `?cmd=get_history[&start=&length=&user=]` → watch history, slimmed to
    /// [`HISTORY_FIELDS`]. `start`/`length` are Tautulli's pagination knobs and
    /// `user` filters by username. READ.
    pub async fn stats_history(
        &self,
        service: &str,
        start: Option<i64>,
        length: Option<i64>,
        user: Option<&str>,
    ) -> Result<Value> {
        let config = self.stats_context(service)?;
        // Owned strings for the numeric knobs so we can pass &str into query_get.
        let start_s = start.map(|v| v.to_string());
        let length_s = length.map(|v| v.to_string());
        let mut extra: Vec<(&str, &str)> = Vec::new();
        if let Some(s) = start_s.as_deref() {
            extra.push(("start", s));
        }
        if let Some(l) = length_s.as_deref() {
            extra.push(("length", l));
        }
        if let Some(u) = user {
            extra.push(("user", u));
        }
        let data = self.stats_cmd(config, "get_history", &extra).await?;
        // Tautulli wraps history rows under `data.data`; slim that array in place
        // and keep the rest of the pagination envelope (recordsTotal, etc.).
        let slimmed = match data {
            Value::Object(mut map) => {
                if let Some(rows) = map.remove("data") {
                    map.insert("data".into(), slim(rows, HISTORY_FIELDS));
                }
                Value::Object(map)
            }
            other => slim(other, HISTORY_FIELDS),
        };
        Ok(slimmed)
    }

    /// GET `?cmd=get_users` → user list, slimmed to [`USER_FIELDS`]. READ.
    pub async fn stats_users(&self, service: &str) -> Result<Value> {
        let config = self.stats_context(service)?;
        let data = self.stats_cmd(config, "get_users", &[]).await?;
        Ok(slim(data, USER_FIELDS))
    }

    /// GET `?cmd=get_libraries` → library list, slimmed to [`LIBRARY_FIELDS`]. READ.
    pub async fn stats_libraries(&self, service: &str) -> Result<Value> {
        let config = self.stats_context(service)?;
        let data = self.stats_cmd(config, "get_libraries", &[]).await?;
        Ok(slim(data, LIBRARY_FIELDS))
    }
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
