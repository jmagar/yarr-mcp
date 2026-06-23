//! Tests for the generated-operation executor helpers.

use super::parse_method;

#[test]
fn parse_method_maps_known_verbs_and_rejects_others() {
    assert_eq!(parse_method("GET").unwrap(), reqwest::Method::GET);
    assert_eq!(parse_method("POST").unwrap(), reqwest::Method::POST);
    assert_eq!(parse_method("PUT").unwrap(), reqwest::Method::PUT);
    assert_eq!(parse_method("DELETE").unwrap(), reqwest::Method::DELETE);
    assert_eq!(parse_method("PATCH").unwrap(), reqwest::Method::PATCH);
    assert!(parse_method("TRACE").is_err());
    assert!(parse_method("nonsense").is_err());
}
