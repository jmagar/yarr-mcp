use super::{ResetTarget, reset_script};

const TARGET: ResetTarget = ResetTarget {
    service: "sonarr",
    container: "sonarr",
    dataset: "sonarr",
    snapshot: "configured-v1",
};

#[test]
fn reset_preflights_everything_before_stopping_the_fleet() {
    let script = reset_script(&[&TARGET], &["sonarr", "radarr"]);
    let preflight = script.find("newest=$(zfs list").unwrap();
    let stop = script.find("docker stop $containers").unwrap();
    assert!(preflight < stop);
    assert!(script.contains("missing container"));
    assert!(script.contains("managed fleet remains stopped"));
    assert!(script.contains("docker stop $containers >/dev/null 2>&1 || true"));
}

#[test]
fn reset_never_uses_recursive_destructive_rollback() {
    let script = reset_script(&[&TARGET], &["sonarr"]);
    assert!(script.contains("zfs rollback backup/lab/live/golden/sonarr@configured-v1"));
    assert!(!script.contains("zfs rollback -r"));
    assert!(script.contains("is not newest"));
}
