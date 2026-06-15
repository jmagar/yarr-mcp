//! Tests for the parsed-CLI [`Command`] enum.

use super::*;

#[test]
fn command_variants_construct_and_compare() {
    let a = Command::Status {
        service: "sonarr".into(),
    };
    let b = Command::Status {
        service: "sonarr".into(),
    };
    assert_eq!(a, b);

    let c = Command::Status {
        service: "radarr".into(),
    };
    assert_ne!(a, c);
}

#[test]
fn distinct_variants_are_not_equal() {
    assert_ne!(Command::Integrations, Command::Help);
}

#[test]
fn curated_carries_action_and_params() {
    let cmd = Command::Curated {
        action: "stats_activity",
        params: serde_json::json!({ "service": "tautulli" }),
    };
    match cmd {
        Command::Curated { action, params } => {
            assert_eq!(action, "stats_activity");
            assert_eq!(params["service"], "tautulli");
        }
        _ => panic!("expected Curated variant"),
    }
}
