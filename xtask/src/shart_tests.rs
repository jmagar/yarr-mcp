use super::{container_command, container_names, status_script};

fn containers() -> Vec<String> {
    vec!["radarr".into(), "sonarr".into()]
}

#[test]
fn lifecycle_commands_are_limited_to_named_containers() {
    assert_eq!(
        container_command("start", &containers()),
        "set -eu; if ! docker info >/dev/null 2>&1; then echo 'shart Docker is unavailable; start the Unraid array before managing the test stack' >&2; exit 1; fi; docker start radarr sonarr"
    );
    assert_eq!(
        container_command("stop", &containers()),
        "set -eu; if ! docker info >/dev/null 2>&1; then echo 'shart Docker is unavailable; start the Unraid array before managing the test stack' >&2; exit 1; fi; docker stop radarr sonarr"
    );
}

#[test]
fn managed_containers_are_the_guarded_kind_allowlist() {
    assert_eq!(container_names().len(), 11);
    assert!(container_names().contains(&"sonarr".to_owned()));
    assert!(container_names().contains(&"tracearr".to_owned()));
}

#[test]
fn strict_status_requires_every_container_to_run() {
    let script = status_script(&containers(), true);
    assert!(script.contains("for container in radarr sonarr"));
    assert!(script.contains("state=missing"));
    assert!(script.contains(r#"if [ "$state" != "running" ]; then failed=1; fi"#));
}

#[test]
fn post_stop_status_allows_stopped_containers_but_not_missing_ones() {
    let script = status_script(&containers(), false);
    assert!(!script.contains(r#"if [ "$state" != "running" ]; then failed=1; fi"#));
    assert!(script.contains("state=missing"));
    assert!(script.contains("failed=1"));
}
