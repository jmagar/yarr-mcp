use super::*;

#[test]
fn short_text_passes_through_unchanged() {
    let text = "hello world";
    assert_eq!(truncate_if_needed(text), text);
}

#[test]
fn empty_string_passes_through() {
    assert_eq!(truncate_if_needed(""), "");
}

#[test]
fn text_at_exact_limit_passes_through() {
    let text = "x".repeat(MAX_RESPONSE_BYTES);
    let result = truncate_if_needed(&text);
    assert!(!result.contains("[TRUNCATED"));
    assert_eq!(result.len(), MAX_RESPONSE_BYTES);
}

#[test]
fn text_over_limit_is_truncated() {
    let text = "x".repeat(MAX_RESPONSE_BYTES + 100);
    let result = truncate_if_needed(&text);
    assert!(result.contains("[TRUNCATED"));
    assert!(result.contains("limit/offset"));
    assert!(result.len() <= MAX_RESPONSE_BYTES);
}

#[test]
fn truncation_notice_mentions_token_limit() {
    let text = "y".repeat(MAX_RESPONSE_BYTES + 1);
    let result = truncate_if_needed(&text);
    assert!(result.contains("10K tokens"));
}

#[test]
fn serialize_with_limit_passes_small_payload_through() {
    let value = serde_json::json!({ "ok": true, "items": [1, 2, 3] });
    let (text, truncated) = serialize_with_limit(&value);
    assert!(!truncated);
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(parsed, value);
}

#[test]
fn serialize_with_limit_emits_parseable_truncation_marker() {
    let big = "x".repeat(MAX_RESPONSE_BYTES + 5_000);
    let value = serde_json::json!({ "blob": big });
    let (text, truncated) = serialize_with_limit(&value);
    assert!(truncated);
    assert!(text.len() <= MAX_RESPONSE_BYTES);
    // The result is VALID JSON (not a mid-string cut) with a {truncated:true} marker.
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("truncated output is valid JSON");
    assert_eq!(parsed["truncated"], true);
    assert!(parsed["reason"].is_string());
    assert!(parsed["partial"].is_string());
}

#[test]
fn truncates_at_utf8_boundary() {
    let mut text = "a".repeat(MAX_RESPONSE_BYTES - 1);
    text.push('€');
    let result = truncate_if_needed(&text);
    let notice_start = result.find("[TRUNCATED").expect("notice should be present") - 2;
    assert!(result[..notice_start].chars().all(|c| c == 'a'));
    assert!(result.len() <= MAX_RESPONSE_BYTES);
    assert!(result.contains("[TRUNCATED"));
}
