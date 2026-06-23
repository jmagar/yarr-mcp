use super::usage;
use crate::actions::curated_commands;
use crate::config::ServiceKind;

#[test]
fn usage_lists_infra_and_service_grammar() {
    let text = usage();
    for expected in [
        "rustarr help",
        "rustarr doctor",
        "rustarr <service> status",
        "rustarr <service> get --path PATH",
        "rustarr <service> post",
        "Services:",
        "Infra verbs:",
    ] {
        assert!(text.contains(expected), "usage missing `{expected}`");
    }
}

#[test]
fn usage_lists_every_service_name() {
    let text = usage();
    for kind in ServiceKind::ALL {
        assert!(
            text.contains(kind.as_str()),
            "usage missing service `{}`",
            kind.as_str()
        );
    }
}

#[test]
fn usage_lists_every_configured_curated_command() {
    let text = usage();
    for cmd in curated_commands() {
        // Usage shows the friendly CLI verb, not the snake_case registry name.
        let verb = super::cli_verb(cmd.name);
        assert!(
            text.contains(&verb),
            "usage missing curated command `{verb}`"
        );
    }
}

/// USAGE must render the friendly capability-local verb (e.g. `activity`), never
/// the kebab spelling of the globally-unique registry action name
/// (`stats-activity`). This is the cosmetic guard for the Z1 fix.
#[test]
fn usage_shows_friendly_verbs_not_prefixed_action_names() {
    let text = usage();
    // (registry action name, friendly verb that should appear, prefixed kebab
    // spelling that must NOT appear as a standalone usage verb).
    for (action, friendly, prefixed) in [
        ("stats_activity", "activity", "stats-activity"),
        ("download_queue", "queue", "download-queue"),
    ] {
        assert_eq!(
            super::cli_verb(action),
            friendly,
            "cli_verb({action}) should be the friendly verb"
        );
        assert!(
            !text.contains(&format!("> {prefixed} ")),
            "usage should not render prefixed action name `{prefixed}` as a verb"
        );
    }
}

#[test]
fn usage_is_stable_across_calls() {
    assert_eq!(usage(), usage());
}
