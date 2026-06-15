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

#[test]
fn urlencode_escapes_reserved_and_non_ascii() {
    // Guards the S6 protection: fragment, query-start, path-sep, and a non-ASCII
    // byte must each be percent-escaped so they can't break out of a query value.
    assert_eq!(urlencode("#"), "%23");
    assert_eq!(urlencode("?"), "%3F");
    assert_eq!(urlencode("/"), "%2F");
    // `é` is U+00E9 → UTF-8 bytes 0xC3 0xA9, each escaped.
    assert_eq!(urlencode("é"), "%C3%A9");
    // Combined, in order, with surrounding reserved chars.
    assert_eq!(urlencode("a#b?c/dé"), "a%23b%3Fc%2Fd%C3%A9");
}
