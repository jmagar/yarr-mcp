//! Tests for the per-capability CLI verb tables and the action→verb lookup.

use super::*;

#[test]
fn capability_verb_tables_cover_all_capabilities() {
    let tables = capability_verb_tables();
    assert_eq!(tables.len(), 6);
    // Every table should declare at least one friendly verb.
    for (_, verbs) in tables {
        assert!(!verbs.is_empty());
    }
}

#[test]
fn cli_verb_for_action_round_trips_a_known_mapping() {
    // Pick the first declared (verb, action) pair and assert the reverse lookup.
    let (_, verbs) = capability_verb_tables()[0];
    let (verb, action) = verbs[0];
    assert_eq!(cli_verb_for_action(action), Some(verb));
}

#[test]
fn cli_verb_for_action_returns_none_for_unknown() {
    assert_eq!(cli_verb_for_action("not_a_real_action"), None);
}
