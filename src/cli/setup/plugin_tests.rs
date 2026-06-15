//! Tests for the `CLAUDE_PLUGIN_OPTION_*` → `RUSTARR_*` mapping table and
//! `apply_plugin_options` behavior.

use super::*;

#[test]
fn plugin_option_map_is_non_empty_and_well_formed() {
    assert!(!PLUGIN_OPTION_MAP.is_empty());
    for (option_var, rustarr_var) in PLUGIN_OPTION_MAP {
        assert!(
            option_var.starts_with("CLAUDE_PLUGIN_OPTION_"),
            "{option_var} should start with CLAUDE_PLUGIN_OPTION_"
        );
        assert!(
            rustarr_var.starts_with("RUSTARR_"),
            "{rustarr_var} should start with RUSTARR_"
        );
    }
}

#[test]
fn plugin_option_map_targets_are_unique() {
    let mut targets: Vec<&str> = PLUGIN_OPTION_MAP.iter().map(|(_, t)| *t).collect();
    let before = targets.len();
    targets.sort_unstable();
    targets.dedup();
    assert_eq!(before, targets.len(), "duplicate RUSTARR_* targets in map");
}
