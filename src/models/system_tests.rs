//! Deserialization fixtures for the status/version/health models.

use super::*;
use serde_json::json;

#[test]
fn servarr_status_decodes_and_ignores_extra_fields() {
    let raw = json!({
        "appName": "Sonarr",
        "instanceName": "Sonarr",
        "version": "4.0.10.2544",
        "buildTime": "2024-09-01T00:00:00Z",
        "branch": "main",
        "runtimeVersion": "6.0.0",
        "osName": "ubuntu",
        "isDocker": true,
        // fields the slim model intentionally drops must not break decoding:
        "startupPath": "/app/sonarr/bin",
        "authentication": "forms"
    });
    let status: ServarrSystemStatus = serde_json::from_value(raw).unwrap();
    assert_eq!(status.app_name.as_deref(), Some("Sonarr"));
    assert_eq!(status.version.as_deref(), Some("4.0.10.2544"));
    assert_eq!(status.is_docker, Some(true));
}

#[test]
fn overseerr_status_decodes() {
    let raw = json!({
        "version": "1.33.2",
        "commitTag": "v1.33.2",
        "updateAvailable": false,
        "commitsBehind": 0,
        "restartRequired": false
    });
    let status: OverseerrStatus = serde_json::from_value(raw).unwrap();
    assert_eq!(status.version.as_deref(), Some("1.33.2"));
    assert_eq!(status.update_available, Some(false));
}

#[test]
fn sab_version_decodes() {
    let v: SabVersion = serde_json::from_value(json!({ "version": "4.3.3" })).unwrap();
    assert_eq!(v.version.as_deref(), Some("4.3.3"));
}

#[test]
fn jellyfin_public_info_decodes_pascal_case() {
    let raw = json!({
        "ServerName": "jellyfin",
        "Version": "10.9.11",
        "ProductName": "Jellyfin Server",
        "OperatingSystem": "Linux",
        "Id": "abc123",
        "StartupWizardCompleted": true,
        "LocalAddress": "http://10.0.0.2:8096"
    });
    let info: JellyfinPublicInfo = serde_json::from_value(raw).unwrap();
    assert_eq!(info.server_name.as_deref(), Some("jellyfin"));
    assert_eq!(info.version.as_deref(), Some("10.9.11"));
    assert_eq!(info.startup_wizard_completed, Some(true));
}

#[test]
fn bazarr_status_unwraps_data_envelope() {
    let raw = json!({
        "data": {
            "bazarr_version": "1.4.3",
            "sonarr_version": "4.0.10.2544",
            "radarr_version": "5.11.0.9244",
            "operating_system": "Linux",
            "python_version": "3.11.2"
        }
    });
    let status: BazarrStatus = serde_json::from_value(raw).unwrap();
    let data = status.data.expect("data present");
    assert_eq!(data.bazarr_version.as_deref(), Some("1.4.3"));
    assert_eq!(data.python_version.as_deref(), Some("3.11.2"));
}

#[test]
fn tracearr_health_decodes_minimal() {
    let health: TracearrHealth = serde_json::from_value(json!({ "status": "ok" })).unwrap();
    assert_eq!(health.status.as_deref(), Some("ok"));
    assert_eq!(health.version, None);
}

#[test]
fn missing_fields_default_to_none() {
    // An empty object must decode cleanly — every field is optional.
    let status: ServarrSystemStatus = serde_json::from_value(json!({})).unwrap();
    assert_eq!(status.version, None);
    assert_eq!(status.app_name, None);
}
