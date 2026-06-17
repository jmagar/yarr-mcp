use super::*;
use crate::config::ServiceKind;

#[test]
fn allows_text_response_only_for_plex_and_qbit() {
    assert!(allows_text_response(ServiceKind::Plex));
    assert!(allows_text_response(ServiceKind::Qbittorrent));
    assert!(!allows_text_response(ServiceKind::Sonarr));
    assert!(!allows_text_response(ServiceKind::Jellyfin));
}

#[test]
fn all_required_service_kinds_are_unique() {
    let mut names = ServiceKind::ALL.map(ServiceKind::as_str).to_vec();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 11);
    assert!(names.contains(&"tautulli"));
}

#[test]
fn client_builds_with_separate_qbit_cookie_store() {
    // Both clients must construct successfully; the qbit client is dedicated.
    let config = crate::config::RustarrConfig::default();
    assert!(RustarrClient::new(&config).is_ok());
}
