use super::*;

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
fn serialize_with_limit_holds_cap_for_escape_heavy_payload() {
    // Every `"` and `\` doubles in length once re-serialized into `partial`, so a
    // fixed envelope reserve could be blown. The iterative shrink must still keep
    // the final bytes under the hard cap.
    let big = "\"\\".repeat(MAX_RESPONSE_BYTES);
    let value = serde_json::json!({ "blob": big });
    let (text, truncated) = serialize_with_limit(&value);
    assert!(truncated);
    assert!(
        text.len() <= MAX_RESPONSE_BYTES,
        "escape-heavy payload broke the cap: {} bytes",
        text.len()
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("truncated output is valid JSON");
    assert_eq!(parsed["truncated"], true);
}
