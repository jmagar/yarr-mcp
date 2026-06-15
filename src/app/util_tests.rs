use super::*;

#[test]
fn urlencode_escapes_spaces() {
    assert_eq!(urlencode("the office"), "the%20office");
}

#[test]
fn urlencode_keeps_unreserved_set() {
    assert_eq!(urlencode("Abc-1_2.3~z"), "Abc-1_2.3~z");
}

#[test]
fn urlencode_escapes_query_injection_chars() {
    // A value like `x&type=movie` must not survive as separators.
    assert_eq!(urlencode("x&type=movie"), "x%26type%3Dmovie");
}
