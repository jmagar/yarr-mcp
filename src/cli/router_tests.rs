use super::{INFRA_VERBS, is_infra_verb, parse_capability_command, route, route_configured};
use crate::config::{ServiceConfig, ServiceKind, YarrConfig};

/// Architecture F3-a invariant: the infra-verb set and the ServiceKind name set
/// must be disjoint, so `token1` is always unambiguously infra OR service.
#[test]
fn infra_verbs_disjoint_from_service_kinds() {
    for kind in ServiceKind::ALL {
        assert!(
            !INFRA_VERBS.contains(&kind.as_str()),
            "ServiceKind `{}` collides with an infra verb",
            kind.as_str()
        );
    }
}

#[test]
fn infra_verb_recognised() {
    assert!(is_infra_verb("help"));
    assert!(is_infra_verb("serve"));
    assert!(!is_infra_verb("sonarr"));
}

#[test]
fn empty_routes_to_none() {
    let cmd = route(&[]).unwrap();
    assert!(cmd.is_none());
}

#[test]
fn service_token_resolves_through_capability_hook() {
    let cmd = route(&["radarr".into(), "status".into()]).unwrap().unwrap();
    assert!(matches!(
        cmd,
        crate::cli::Command::Status { service } if service == "radarr"
    ));
}

#[test]
fn qbittorrent_alias_resolves() {
    // FromStr accepts `qb`/`qbit` aliases — router normalises to canonical name.
    let cmd = route(&["qb".into(), "status".into()]).unwrap().unwrap();
    assert!(matches!(
        cmd,
        crate::cli::Command::Status { service } if service == "qbittorrent"
    ));
}

#[test]
fn serve_and_mcp_are_reserved_but_unreachable() {
    // They are infra verbs (reserved names) but rejected here — main.rs handles
    // them as run modes before parsing.
    let err = route(&["serve".into()]).unwrap_err();
    assert!(err.to_string().contains("run mode"));
}

#[test]
fn unknown_token_lists_both_sets() {
    let err = route(&["nope".into()]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("services:"));
    assert!(msg.contains("infra commands:"));
}

#[test]
fn capability_hook_rejects_unknown_verb() {
    let err = parse_capability_command(
        ServiceKind::Sonarr,
        ServiceKind::Sonarr.capability(),
        "sessions",
        &[],
    )
    .unwrap_err();
    assert!(err.to_string().contains("unknown command"));
    assert!(err.to_string().contains("sonarr"));
}

#[test]
fn get_rejects_body() {
    let err = route(&[
        "sonarr".into(),
        "get".into(),
        "--path".into(),
        "/x".into(),
        "--body".into(),
        "{}".into(),
    ])
    .unwrap_err();
    assert!(err.to_string().contains("does not accept --body"));
}

fn configured_services() -> YarrConfig {
    YarrConfig {
        services: vec![
            ServiceConfig {
                name: "sonarr-east".into(),
                kind: ServiceKind::Sonarr,
                ..ServiceConfig::default()
            },
            ServiceConfig {
                name: "sonarr-west".into(),
                kind: ServiceKind::Sonarr,
                ..ServiceConfig::default()
            },
            ServiceConfig {
                name: "movies".into(),
                kind: ServiceKind::Radarr,
                ..ServiceConfig::default()
            },
        ],
    }
}

#[test]
fn configured_cli_uses_exact_name_and_unique_kind_fallback() {
    let config = configured_services();
    let exact = route_configured(&["sonarr-east".into(), "status".into()], &config)
        .unwrap()
        .unwrap();
    assert!(matches!(exact, crate::cli::Command::Status { service } if service == "sonarr-east"));

    let unique = route_configured(&["radarr".into(), "status".into()], &config)
        .unwrap()
        .unwrap();
    assert!(matches!(unique, crate::cli::Command::Status { service } if service == "movies"));
}

#[test]
fn configured_cli_rejects_ambiguous_kind_fallback() {
    let error =
        route_configured(&["sonarr".into(), "status".into()], &configured_services()).unwrap_err();
    assert!(error.to_string().contains("ambiguous"));
    assert!(error.to_string().contains("sonarr-east"));
}

#[test]
fn configured_cli_rejects_reserved_service_identity() {
    let config = YarrConfig {
        services: vec![ServiceConfig {
            name: "help".into(),
            kind: ServiceKind::Sonarr,
            ..ServiceConfig::default()
        }],
    };
    let error = route_configured(&["help".into()], &config).unwrap_err();
    assert!(error.to_string().contains("reserved"));
}
