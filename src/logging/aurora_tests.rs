use super::*;

#[test]
fn service_name_is_pink_256() {
    assert_eq!(SERVICE_NAME, 211);
}

#[test]
fn all_constants_are_u8_values() {
    // All aurora constants are u8 by type — this test ensures none are accidentally
    // widened to a larger type when the palette is updated.
    let _: &[u8] = &[
        SERVICE_NAME,
        ACCENT_PRIMARY,
        TEXT_MUTED,
        SUCCESS,
        WARN,
        ERROR,
    ];
}

#[test]
fn success_and_warn_and_error_are_distinct() {
    assert_ne!(SUCCESS, WARN);
    assert_ne!(WARN, ERROR);
    assert_ne!(SUCCESS, ERROR);
}
