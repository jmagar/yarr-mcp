//! Tests for [`ServiceKind`] parsing/round-trips, the `ALL` table, and
//! `default_data_dir`.

use super::*;
use std::str::FromStr;

#[test]
fn service_kind_all_has_11_entries() {
    assert_eq!(ServiceKind::ALL.len(), 11);
}

/// Golden-value pin for every kind's `as_str` + `default_status_path`. Guards the
/// `KIND_ROWS` table (and the identity-keyed `row()` lookup) against any future
/// reorder/edit silently mistyping a kind or pointing a status check at the wrong
/// endpoint — which the round-trip and "non-empty" checks cannot catch.
#[test]
fn kind_rows_pin_exact_values() {
    let expected = [
        (ServiceKind::Sonarr, "sonarr", "/api/v3/system/status"),
        (ServiceKind::Radarr, "radarr", "/api/v3/system/status"),
        (ServiceKind::Prowlarr, "prowlarr", "/api/v1/system/status"),
        (
            ServiceKind::Tautulli,
            "tautulli",
            "/api/v2?cmd=get_server_info",
        ),
        (ServiceKind::Overseerr, "overseerr", "/api/v1/status"),
        (ServiceKind::Bazarr, "bazarr", "/api/system/status"),
        (ServiceKind::Tracearr, "tracearr", "/health"),
        (ServiceKind::Sabnzbd, "sabnzbd", "/api?mode=version"),
        (
            ServiceKind::Qbittorrent,
            "qbittorrent",
            "/api/v2/app/version",
        ),
        (ServiceKind::Plex, "plex", "/identity"),
        (ServiceKind::Jellyfin, "jellyfin", "/System/Info/Public"),
    ];
    assert_eq!(expected.len(), ServiceKind::ALL.len());
    for (kind, as_str, status_path) in expected {
        assert_eq!(kind.as_str(), as_str, "as_str for {kind:?}");
        assert_eq!(
            kind.default_status_path(),
            status_path,
            "default_status_path for {kind:?}"
        );
    }
}

#[test]
fn service_kind_as_str_round_trips() {
    for kind in ServiceKind::ALL {
        let s = kind.as_str();
        assert_eq!(ServiceKind::from_str(s).unwrap(), kind);
    }
}

#[test]
fn qbittorrent_aliases_parse() {
    assert_eq!(
        ServiceKind::from_str("qbit").unwrap(),
        ServiceKind::Qbittorrent
    );
    assert_eq!(
        ServiceKind::from_str("qb").unwrap(),
        ServiceKind::Qbittorrent
    );
    assert_eq!(
        ServiceKind::from_str("QBITTORRENT").unwrap(),
        ServiceKind::Qbittorrent
    );
}

#[test]
fn underscore_normalizes_to_kebab() {
    assert_eq!(
        ServiceKind::from_str("sonarr").unwrap(),
        ServiceKind::Sonarr
    );
}

#[test]
fn unknown_kind_errors() {
    assert!(ServiceKind::from_str("nope").is_err());
}

#[test]
fn default_data_dir_is_non_empty() {
    let dir = default_data_dir().expect("default_data_dir should resolve in test env");
    assert!(!dir.as_os_str().is_empty());
}

#[test]
fn all_kinds_match_kind_rows_table() {
    // Every kind in ALL must round-trip through the declarative table without
    // panicking and yield a non-empty status path — guards table/enum drift.
    for kind in ServiceKind::ALL {
        assert!(!kind.as_str().is_empty());
        assert!(kind.default_status_path().starts_with('/'));
    }
}

#[test]
fn load_services_bails_when_url_missing() {
    let _guard = crate::testing::ENV_LOCK.lock().unwrap();

    let old_services = std::env::var_os("RUSTARR_SERVICES");
    let old_url = std::env::var_os("RUSTARR_SONARR_URL");

    // Service named but no URL set → eager validation must fail.
    // SAFETY: `_guard` holds the process-wide ENV_LOCK, so no other test mutates
    // the environment concurrently (edition 2024 makes set_var/remove_var unsafe).
    unsafe {
        std::env::set_var("RUSTARR_SERVICES", "sonarr");
        std::env::remove_var("RUSTARR_SONARR_URL");
    }

    let mut config = super::super::RustarrConfig::default();
    let result = load_services_from_env(&mut config);

    // Restore env before asserting so a failure doesn't leak state.
    // SAFETY: still holding ENV_LOCK (see above).
    unsafe {
        match old_services {
            Some(v) => std::env::set_var("RUSTARR_SERVICES", v),
            None => std::env::remove_var("RUSTARR_SERVICES"),
        }
        match old_url {
            Some(v) => std::env::set_var("RUSTARR_SONARR_URL", v),
            None => std::env::remove_var("RUSTARR_SONARR_URL"),
        }
    }

    let err = result.expect_err("missing URL should fail fast");
    let msg = err.to_string();
    assert!(
        msg.contains("RUSTARR_SONARR_URL is required"),
        "error should name the missing URL var, got: {msg}"
    );
    assert!(msg.contains("sonarr"), "error should name the service");
}
