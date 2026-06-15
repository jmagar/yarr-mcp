use super::usage;
use crate::actions::CURATED_COMMANDS;
use crate::config::ServiceKind;

#[test]
fn usage_lists_infra_and_service_grammar() {
    let text = usage();
    for expected in [
        "rustarr integrations",
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
    for cmd in CURATED_COMMANDS {
        assert!(
            text.contains(cmd.name),
            "usage missing curated command `{}`",
            cmd.name
        );
    }
}

#[test]
fn usage_is_stable_across_calls() {
    assert_eq!(usage(), usage());
}
