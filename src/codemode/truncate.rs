//! Code-Mode-aware response budgeting.
//!
//! The Code Mode envelope (`{result, calls, logs, artifacts, …}`) can blow an
//! agent's context window. rustarr's transport layer already caps the whole MCP
//! tool response at [`crate::token_limit::MAX_RESPONSE_BYTES`] — but that cap is a
//! *blind byte slice* that chops `result` mid-JSON and discards the audit envelope
//! structure. This pass shapes the envelope INTELLIGENTLY, strictly BELOW the
//! transport cap, so the agent always receives parseable JSON:
//!
//! 1. If the serialized envelope already fits the budget → leave it untouched.
//! 2. Otherwise trim `logs` oldest-first (keeping the newest lines, prepending a
//!    `[logs truncated …]` sentinel) to preserve `result` — the script's answer is
//!    more valuable than its debug output. (rustarr diverges from lab here, which
//!    caps the result first; rustarr prioritizes keeping the answer.)
//! 3. Only if the non-log payload (`result`/`calls`/`artifacts`) is itself over
//!    budget do we replace an oversized `result` with a structured, parseable
//!    `{truncated, original_bytes, original_tokens, preview, next_action}` marker
//!    (matching the shape token_limit uses), then fit back as many newest log
//!    lines as the remaining budget allows.
//!
//! The budget is *derived from* `MAX_RESPONSE_BYTES` (3/5 of it ≈ 24 KB, the same
//! figure lab and Cloudflare's codemode use) so the two caps can never invert and
//! the shaped envelope never trips the transport truncation that would undo this
//! work. Token counts in the marker are informational (bytes / 4, a conservative
//! over-estimate for JSON); the budget *check* is bytes, matching the transport cap.

use serde_json::{Value, json};

use crate::token_limit::MAX_RESPONSE_BYTES;

/// Byte budget for the shaped Code Mode envelope. 3/5 of the transport cap leaves
/// generous headroom (~16 KB at the default 40 KB cap) so re-serialization /
/// escape growth can never push the shaped envelope past the transport truncation.
const RESPONSE_BUDGET: usize = MAX_RESPONSE_BYTES / 5 * 3;

/// Bytes-per-token estimate for the (informational) token count in the marker.
const TOKEN_DIVISOR: usize = 4;

/// Head-preview size (bytes) of an oversized result placed in the marker.
const PREVIEW_BYTES: usize = 1024;

/// Shape `response` to fit [`RESPONSE_BUDGET`] in place (see module docs).
pub fn fit_response(response: &mut Value) {
    if within_budget(response) {
        return;
    }

    // Pull logs out so we can measure the rest of the envelope on its own.
    let logs = take_logs(response);

    if !within_budget(response) {
        // The non-log payload (result/calls/artifacts) is itself over budget, so
        // trimming logs alone can't help — replace an oversized `result` with a
        // compact marker (only when that actually shrinks it).
        marker_oversized_result(response);
    }

    // Fit back as many of the NEWEST log lines as the remaining budget allows.
    restore_newest_logs(response, logs);
}

/// True iff the compact serialization of `value` is within budget.
fn within_budget(value: &Value) -> bool {
    serialized_len(value) <= RESPONSE_BUDGET
}

fn serialized_len(value: &Value) -> usize {
    serde_json::to_string(value).map(|s| s.len()).unwrap_or(0)
}

/// Remove the `logs` array from the envelope (replacing it with `[]`) and return
/// its lines. Returns empty if `logs` is absent or not an array.
fn take_logs(response: &mut Value) -> Vec<Value> {
    match response.get_mut("logs") {
        Some(slot) if slot.is_array() => {
            let taken = std::mem::replace(slot, Value::Array(Vec::new()));
            match taken {
                Value::Array(lines) => lines,
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}

/// Replace `response["result"]` with a structured truncation marker, but only if
/// the marker is smaller than the original result (otherwise a small result that
/// isn't the bottleneck would needlessly grow).
fn marker_oversized_result(response: &mut Value) {
    let Some(result) = response.get("result") else {
        return;
    };
    if result.is_null() {
        return;
    }
    let serialized = serde_json::to_string(result).unwrap_or_else(|_| "null".to_string());
    let marker = json!({
        "truncated": true,
        "original_bytes": serialized.len(),
        "original_tokens": serialized.len() / TOKEN_DIVISOR,
        "preview": utf8_prefix(&serialized, PREVIEW_BYTES),
        "next_action": "Result exceeded the Code Mode response budget. Narrow it: \
                        request fewer fields, add limit/offset/filter params, or \
                        writeArtifact() the full payload and return a summary.",
    });
    if serialized_len(&marker) < serialized.len() {
        response["result"] = marker;
    }
}

/// Fit the largest suffix of `logs` (the newest lines) that keeps the envelope
/// within budget, prepending a sentinel when any lines are dropped. Binary search
/// keeps this O(n log n) serializations rather than O(n^2).
fn restore_newest_logs(response: &mut Value, logs: Vec<Value>) {
    let total = logs.len();
    if total == 0 {
        return;
    }

    // `keep` = number of newest lines retained. Find the largest `keep` that fits.
    let set_logs = |response: &mut Value, keep: usize| {
        let dropped = total - keep;
        let mut out: Vec<Value> = Vec::with_capacity(keep + 1);
        if dropped > 0 {
            out.push(Value::String(format!(
                "[logs truncated to fit response budget — {dropped} line(s) dropped]"
            )));
        }
        out.extend(logs[total - keep..].iter().cloned());
        response["logs"] = Value::Array(out);
    };

    // Fast path: do all lines fit?
    set_logs(response, total);
    if within_budget(response) {
        return;
    }

    let (mut lo, mut hi) = (0usize, total);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2); // bias toward keeping more
        set_logs(response, mid);
        if within_budget(response) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    set_logs(response, lo);
}

/// The largest char-boundary prefix of `s` that is at most `max_bytes` bytes.
fn utf8_prefix(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
#[path = "truncate_tests.rs"]
mod tests;
