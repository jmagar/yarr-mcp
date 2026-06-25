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

#[test]
fn skill_fallback_groups_by_service_escapes_and_skips_non_services() {
    let vars = vec![
        (
            "CLAUDE_PLUGIN_OPTION_SONARR_URL".to_string(),
            "http://localhost:8989".to_string(),
        ),
        (
            "CLAUDE_PLUGIN_OPTION_SONARR_API_KEY".to_string(),
            "k'ey".to_string(), // embedded single quote must be escaped
        ),
        (
            "CLAUDE_PLUGIN_OPTION_BAZARR_URL".to_string(),
            "http://localhost:6767".to_string(),
        ),
        // Non-service options must not produce a file group.
        (
            "CLAUDE_PLUGIN_OPTION_SERVER_URL".to_string(),
            "http://x".to_string(),
        ),
        (
            "CLAUDE_PLUGIN_OPTION_API_TOKEN".to_string(),
            "secret".to_string(),
        ),
        // Empty and unrelated vars are ignored.
        ("CLAUDE_PLUGIN_OPTION_PLEX_URL".to_string(), String::new()),
        ("PATH".to_string(), "/usr/bin".to_string()),
    ];

    let bodies = build_skill_fallback_bodies(vars);

    let sonarr = bodies.get("sonarr").expect("sonarr group present");
    assert!(sonarr.contains("SONARR_URL='http://localhost:8989'\n"));
    // single quote escaped as '\'' inside the single-quoted value
    assert!(sonarr.contains("SONARR_API_KEY='k'\\''ey'\n"));
    assert!(bodies.contains_key("bazarr"));

    // Non-service prefixes and empty values do not create groups.
    assert!(!bodies.contains_key("server"));
    assert!(!bodies.contains_key("plex"));
    assert_eq!(bodies.len(), 2, "only sonarr + bazarr groups expected");
}
