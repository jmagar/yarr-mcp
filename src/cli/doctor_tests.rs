use super::checks::{check_port_available, check_required_var};

#[test]
fn check_required_var_passes_when_value_is_set() {
    let result = check_required_var("RUSTARR_SONARR_API_KEY", "sk-test-value");
    assert!(result.ok, "non-empty value should pass");
}

#[test]
fn check_required_var_fails_when_value_is_empty() {
    let result = check_required_var("RUSTARR_SONARR_API_KEY", "");
    assert!(!result.ok, "empty value should fail");
    assert!(result.hint.is_some(), "failed check should have a hint");
}

#[tokio::test]
async fn check_port_available_passes_for_unused_high_port() {
    let result = check_port_available("127.0.0.1", 59999).await;
    assert!(result.ok, "unused high port should be available");
}
