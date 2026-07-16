use super::REQUIRED_TOOLS;

#[test]
fn local_ci_requires_every_external_gate_instead_of_skipping() {
    for required in ["cargo-nextest", "taplo", "cargo-deny", "actionlint", "npm"] {
        assert!(REQUIRED_TOOLS.contains(&required));
    }
}
