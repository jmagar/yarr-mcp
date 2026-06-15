use crate::config::{ServiceConfig, ServiceKind};
use crate::rustarr::slim;
use serde_json::json;

use super::{qbit_path, TORRENT_FIELDS};

fn qbit_config() -> ServiceConfig {
    ServiceConfig {
        name: "qbittorrent".into(),
        kind: ServiceKind::Qbittorrent,
        base_url: "http://localhost:8081".into(),
        username: Some("admin".into()),
        password: Some("pw".into()),
        ..ServiceConfig::default()
    }
}

#[test]
fn qbit_path_uses_v2_prefix() {
    // Descriptor-driven: qbittorrent is /api/v2, no hardcoded version.
    let config = qbit_config();
    assert_eq!(
        qbit_path(&config, "/torrents/info"),
        "/api/v2/torrents/info"
    );
    assert_eq!(qbit_path(&config, "/torrents/add"), "/api/v2/torrents/add");
    assert_eq!(
        qbit_path(&config, "/torrents/delete"),
        "/api/v2/torrents/delete"
    );
}

#[test]
fn qbit_pause_resume_use_v5_stop_start_not_pause_resume() {
    // FACT (bead, HIGH): qBittorrent v5 renamed pause->stop, resume->start.
    let config = qbit_config();
    // pause targets /torrents/stop ...
    assert_eq!(
        qbit_path(&config, "/torrents/stop"),
        "/api/v2/torrents/stop"
    );
    // ... resume targets /torrents/start.
    assert_eq!(
        qbit_path(&config, "/torrents/start"),
        "/api/v2/torrents/start"
    );
    // The v4 names must NOT be what these resolve to.
    assert_ne!(
        qbit_path(&config, "/torrents/stop"),
        "/api/v2/torrents/pause"
    );
    assert_ne!(
        qbit_path(&config, "/torrents/start"),
        "/api/v2/torrents/resume"
    );
}

#[test]
fn queue_slim_keeps_torrent_fields() {
    let raw = json!([{
        "hash": "abc123",
        "name": "Ubuntu.iso",
        "state": "downloading",
        "progress": 0.42,
        "dlspeed": 1048576,
        "size": 734003200_i64,
        "category": "linux",
        "tracker": "drop me",
        "magnet_uri": "drop me too"
    }]);
    let slimmed = slim(raw, TORRENT_FIELDS);
    let row = &slimmed[0];
    assert_eq!(row["hash"], "abc123");
    assert_eq!(row["name"], "Ubuntu.iso");
    assert_eq!(row["state"], "downloading");
    assert_eq!(row["progress"], 0.42);
    assert_eq!(row["category"], "linux");
    assert!(row.get("tracker").is_none(), "bulky fields dropped");
    assert!(row.get("magnet_uri").is_none());
}
