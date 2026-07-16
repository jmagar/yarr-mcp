use super::*;

#[test]
fn token_bucket_caps_and_recovers_after_window() {
    let start = Instant::now();
    let mut bucket = TokenBucket::new();
    for _ in 0..TOKEN_ISSUANCE_LIMIT {
        assert!(bucket.admit(start));
    }
    assert!(!bucket.admit(start));
    assert!(bucket.admit(start + TOKEN_ISSUANCE_WINDOW));
}
