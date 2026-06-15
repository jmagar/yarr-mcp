//! Tests for [`ServiceKind`] parsing/round-trips, the `ALL` table, and
//! `default_data_dir`.

use super::*;
use std::str::FromStr;

#[test]
fn service_kind_all_has_15_entries() {
    assert_eq!(ServiceKind::ALL.len(), 15);
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
