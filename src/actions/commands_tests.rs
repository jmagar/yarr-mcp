//! Tests for the curated-command module declarations: verify each per-capability
//! slice is reachable through the re-exports and that the aggregated registry is
//! non-empty.

use super::*;

#[test]
fn per_capability_slices_are_reachable() {
    // Each re-exported slice must resolve and be non-empty.
    assert!(!ARR_COMMANDS.is_empty());
    assert!(!DOWNLOAD_COMMANDS.is_empty());
    assert!(!INDEXER_COMMANDS.is_empty());
    assert!(!MEDIA_COMMANDS.is_empty());
    assert!(!REQUEST_COMMANDS.is_empty());
    assert!(!STATS_COMMANDS.is_empty());
}

#[test]
fn aggregated_curated_commands_is_non_empty() {
    assert!(!crate::actions::curated_commands().is_empty());
}
