use super::RemoteOutput;

#[test]
fn remote_output_preserves_structured_failure_fields() {
    let output = RemoteOutput {
        status: Some(7),
        stdout: "partial status".into(),
        stderr: "docker unavailable".into(),
    };
    let error = output
        .ensure_success("inspect fleet")
        .unwrap_err()
        .to_string();
    assert!(error.contains("exit=7"));
    assert!(error.contains("partial status"));
    assert!(error.contains("docker unavailable"));
}
