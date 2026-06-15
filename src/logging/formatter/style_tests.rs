//! Tests for the style/sanitize helpers used by the Aurora formatter.

use super::*;

#[test]
fn sanitize_strips_ansi_escapes() {
    let dirty = "\x1b[31mFAKE\x1b[0m";
    let clean = sanitize_field_value(dirty);
    assert!(!clean.contains('\x1b'));
    assert!(clean.contains("FAKE"));
}

#[test]
fn sanitize_preserves_clean_input_borrowed() {
    let clean = sanitize_field_value("hello");
    assert!(matches!(clean, std::borrow::Cow::Borrowed("hello")));
}

#[test]
fn sanitize_keeps_tab_and_newline() {
    let s = sanitize_field_value("a\tb\nc");
    assert_eq!(s, "a\tb\nc");
}

#[test]
fn format_field_value_quotes_whitespace() {
    assert_eq!(format_field_value("plain"), "plain");
    assert_eq!(format_field_value("two words"), "\"two words\"");
}

#[test]
fn should_skip_suppresses_false_destructive_flag() {
    assert!(should_skip_field("destructive", "false"));
    assert!(!should_skip_field("destructive", "true"));
    assert!(!should_skip_field("other", "false"));
}
