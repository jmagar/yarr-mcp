use super::*;

#[test]
fn scopes_satisfy_write_covers_read() {
    let write = vec![WRITE_SCOPE.to_string()];
    assert!(scopes_satisfy(&write, READ_SCOPE));
    assert!(scopes_satisfy(&write, WRITE_SCOPE));
    assert_eq!(READ_SCOPE, "yarr:read");
    assert_eq!(WRITE_SCOPE, "yarr:write");
    assert_ne!(READ_SCOPE, "rustarr:read");
    assert_ne!(WRITE_SCOPE, "rustarr:write");

    let read = vec![READ_SCOPE.to_string()];
    assert!(scopes_satisfy(&read, READ_SCOPE));
    assert!(!scopes_satisfy(&read, WRITE_SCOPE));

    let legacy_write = vec!["rustarr:write".to_string()];
    assert!(!scopes_satisfy(&legacy_write, READ_SCOPE));
    assert!(!scopes_satisfy(&legacy_write, WRITE_SCOPE));
}

#[test]
fn action_not_valid_for_kind_display_includes_valid_list() {
    let err = ValidationError::ActionNotValidForKind {
        action: "set_quality".into(),
        kind: "plex".into(),
        valid_actions: vec!["integrations".into(), "service_status".into()],
    };
    let msg = err.to_string();
    assert!(msg.contains("action=set_quality"));
    assert!(msg.contains("kind=plex"));
    assert!(msg.contains("integrations"));
    assert!(msg.contains("service_status"));
}

#[test]
fn rustarr_action_name_round_trip() {
    assert_eq!(YarrAction::Help.name(), "help");
    assert_eq!(
        YarrAction::ServiceStatus {
            service: "sonarr".into()
        }
        .name(),
        "service_status"
    );
}
