//! Shared app-layer utilities that are not tied to any one capability.
//!
//! Lives here (rather than inside `arr::editor`) so cross-capability callers such
//! as `requests` and `indexer` can reuse them without coupling to Arr internals.

/// Minimal percent-encode for a query value (RFC 3986 unreserved set kept; all
/// other bytes percent-escaped). Used for lookup/search/filter terms that are
/// interpolated into a query string.
pub fn urlencode(term: &str) -> String {
    let mut out = String::with_capacity(term.len());
    for b in term.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
#[path = "util_tests.rs"]
mod tests;
