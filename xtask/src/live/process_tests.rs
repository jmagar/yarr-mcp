use super::output_for_command;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[test]
fn timeout_does_not_wait_for_grandchild_holding_output_open() {
    let start = Instant::now();
    let err = output_for_command(
        "/bin/sh",
        &["-c", "(sleep 5) & sleep 10"],
        &BTreeMap::new(),
        None,
        Duration::from_millis(200),
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("timed out after 0s"), "{err}");
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "timeout waited for inherited pipe handles: {:?}",
        start.elapsed()
    );
}
