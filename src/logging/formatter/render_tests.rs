//! Tests for [`AuroraFormatter`]: it is a zero-sized, copyable formatter.

use super::*;

#[test]
fn formatter_is_copy_and_zero_sized() {
    let fmt = AuroraFormatter;
    // Copy: using `fmt` after copying it must still be valid.
    let copy = fmt;
    let _ = fmt;
    let _ = copy;
    assert_eq!(std::mem::size_of::<AuroraFormatter>(), 0);
}
