//! Response size cap — prevents context-window exhaustion in MCP clients.
//!
//! # TEMPLATE: The 10K token philosophy
//!
//! MCP servers communicate with AI agents that have finite context windows.
//! A single oversized response can consume a large fraction of that window,
//! leaving little room for the agent's reasoning and subsequent tool calls.
//!
//! **Rule**: no single MCP tool response may exceed ~10,000 tokens (~40KB).
//!
//! ## Why 40KB?
//!
//! - ~4 bytes/token on average (English prose, JSON, code)
//! - 40,000 bytes / 4 bytes ≈ 10,000 tokens
//! - 10K tokens is a generous upper bound that fits comfortably in any modern
//!   LLM context window without crowding out reasoning
//!
//! ## What to do instead of returning huge responses
//!
//! 1. **Paginate**: add `limit`/`offset` parameters to list actions
//! 2. **Filter**: add `filter` or `query` parameters to narrow results
//! 3. **Summarize**: return counts and top-N items, with a link to get more
//! 4. **Stream**: for logs/events, return the most recent N lines
//!
//! ## Where to apply truncation
//!
//! Apply [`serialize_with_limit()`] in the MCP response path (see
//! `mcp/rmcp_server.rs`) AFTER the service call, when serializing the result
//! into the tool response. It returns a *parseable* JSON marker rather than a
//! human notice, so the agent can detect truncation programmatically:
//!
//! ```rust,ignore
//! use rustarr::token_limit;
//!
//! let result = state.service.list_things(limit, offset).await?;
//! let (text, truncated) = token_limit::serialize_with_limit(&result);
//! // `text` is bounded to MAX_RESPONSE_BYTES; `truncated` flags the cap.
//! ```
//!
//! ## Truncation is a safety net, not the primary strategy
//!
//! Truncation is the last resort. Design your actions to return bounded data
//! by default (limit=50, summary-only, etc.) so truncation rarely triggers.
//! When it does trigger, the truncation message tells the agent exactly what
//! to do next.

/// Maximum response size in bytes.
///
/// This constant is the single source of truth for the 10K token cap.
/// Change it here to adjust the cap for all actions simultaneously.
///
/// # TEMPLATE: Adjusting the cap
///
/// For services that return very dense data (e.g. binary-encoded metrics),
/// you may want a lower cap. For services that return sparse text (e.g.
/// configuration files), the cap may be relaxed slightly.
///
/// Never exceed 100KB (25K tokens) — at that size, agents start losing
/// context from earlier in the conversation.
pub const MAX_RESPONSE_BYTES: usize = 40_000;

/// Serialize `value` for an MCP tool result, enforcing [`MAX_RESPONSE_BYTES`]
/// with a *parseable* truncation marker.
///
/// Unlike a plain-text truncation notice (which would break JSON parsing), this
/// returns a string that is **always valid JSON**:
///
/// - Under the cap → the compact serialization of `value`.
/// - Over the cap → a JSON object `{"truncated":true,"reason":...,"partial":"…"}`
///   where `partial` is the boundary-safe head of the original payload. An agent
///   can `JSON.parse` the result, branch on `truncated`, and re-query with
///   `limit`/`fields` rather than choke on a mid-string cut (AN-6).
///
/// Returns the JSON text and a flag indicating whether truncation occurred.
pub fn serialize_with_limit(value: &serde_json::Value) -> (String, bool) {
    let full = serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned());
    if full.len() <= MAX_RESPONSE_BYTES {
        return (full, false);
    }

    let reason = format!(
        "response exceeded {MAX_RESPONSE_BYTES} bytes (~10K tokens); \
         re-query with limit/offset or fields= to narrow the result"
    );

    // A fixed envelope reserve can be exceeded by escape-heavy payloads (every
    // `"` / `\` in `partial` becomes two bytes once re-serialized). Iteratively
    // shrink the boundary until the serialized marker actually fits, so the hard
    // cap holds regardless of escape density.
    let mut boundary = MAX_RESPONSE_BYTES.min(full.len());
    loop {
        // Snap to a char boundary so we never slice mid-codepoint.
        while boundary > 0 && !full.is_char_boundary(boundary) {
            boundary -= 1;
        }
        let marker = serde_json::json!({
            "truncated": true,
            "reason": reason,
            "partial": &full[..boundary],
        });
        let text = serde_json::to_string(&marker)
            .unwrap_or_else(|_| r#"{"truncated":true,"reason":"response too large"}"#.to_owned());
        if text.len() <= MAX_RESPONSE_BYTES || boundary == 0 {
            return (text, true);
        }
        // Shrink and retry. Geometric step keeps this O(log n).
        boundary -= (boundary / 8).max(1);
    }
}

#[cfg(test)]
#[path = "token_limit_tests.rs"]
mod tests;
