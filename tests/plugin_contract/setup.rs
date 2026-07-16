use std::fs;

use serde_json::Value;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use super::common::{assert_private_file, setup_command};

#[test]
fn setup_plugin_hook_no_repair_emits_json_contract() {
    let dir = tempdir().unwrap();
    let output = setup_command(dir.path())
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["exit_policy"], "advisory_failure");
    assert_eq!(json["ran_repair"], false);
    assert_eq!(json["no_repair"], true);
    assert!(json["blocking_failures"].as_array().unwrap().is_empty());
    assert!(
        json["advisory_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["code"] == "env_file_missing")
    );
    assert!(!dir.path().join(".env").exists());
}

#[test]
fn setup_repair_creates_env_file_without_upstream_contact() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("appdata");
    let output = setup_command(&data_dir)
        .args(["setup", "repair"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["exit_policy"], "success");
    assert_eq!(json["ran_repair"], true);
    assert_eq!(json["no_repair"], false);

    let env_file = fs::read_to_string(data_dir.join(".env")).unwrap();
    assert!(env_file.contains("YARR_SONARR_URL=https://api.yarr.test"));
    assert!(env_file.contains("YARR_SONARR_API_KEY=yarr-secret"));
    assert!(env_file.contains("YARR_MCP_TOKEN=mcp-secret"));
    assert_private_file(&data_dir.join(".env"));
}

#[test]
fn setup_repair_replaces_existing_env_file_with_private_mode() {
    let dir = tempdir().unwrap();
    let env_path = dir.path().join(".env");
    fs::write(&env_path, "OLD_VALUE=1\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&env_path, fs::Permissions::from_mode(0o644)).unwrap();

    let output = setup_command(dir.path())
        .args(["setup", "repair"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let env_file = fs::read_to_string(&env_path).unwrap();
    assert!(!env_file.contains("OLD_VALUE"));
    assert!(env_file.contains("YARR_SONARR_URL=https://api.yarr.test"));
    assert_private_file(&env_path);
}
