use crate::cli::command::Command;
use crate::cli::parse_args_from;
use serde_json::json;

#[test]
fn qbittorrent_queue_maps_kebab_to_download_queue() {
    // `queue` verb maps to the non-colliding `download_queue` action.
    let cmd = parse_args_from(["qbittorrent", "queue"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "download_queue",
            params: json!({ "service": "qbittorrent" }),
        }
    );
}

#[test]
fn sabnzbd_queue_maps_kebab_to_download_queue() {
    let cmd = parse_args_from(["sabnzbd", "queue"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "download_queue",
            params: json!({ "service": "sabnzbd" }),
        }
    );
}

#[test]
fn sabnzbd_add_parses_url() {
    let cmd = parse_args_from(["sabnzbd", "add", "--url", "http://x/a.nzb"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "download_add",
            params: json!({ "service": "sabnzbd", "url": "http://x/a.nzb" }),
        }
    );
}

#[test]
fn add_requires_url() {
    let err = parse_args_from(["sabnzbd", "add"]).unwrap_err();
    assert!(err.to_string().contains("--url"));
}

#[test]
fn qbittorrent_pause_with_hash() {
    let cmd = parse_args_from(["qbittorrent", "pause", "--hash", "abc123"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "download_pause",
            params: json!({ "service": "qbittorrent", "hash": "abc123" }),
        }
    );
}

#[test]
fn qbittorrent_resume_all_when_no_id() {
    let cmd = parse_args_from(["qbittorrent", "resume"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "download_resume",
            params: json!({ "service": "qbittorrent" }),
        }
    );
}

#[test]
fn qbittorrent_remove_with_hash_and_delete_files() {
    let cmd = parse_args_from([
        "qbittorrent",
        "remove",
        "--hash",
        "abc123",
        "--delete-files",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "download_remove",
            params: json!({
                "service": "qbittorrent",
                "delete_files": true,
                "hash": "abc123"
            }),
        }
    );
}

#[test]
fn remove_requires_id_or_hash() {
    let err = parse_args_from(["sabnzbd", "remove"]).unwrap_err();
    assert!(err.to_string().contains("--id") || err.to_string().contains("--hash"));
}

#[test]
fn sabnzbd_remove_with_nzo_id() {
    let cmd = parse_args_from(["sabnzbd", "remove", "--id", "SABnzbd_nzo_x"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "download_remove",
            params: json!({ "service": "sabnzbd", "id": "SABnzbd_nzo_x" }),
        }
    );
}

#[test]
fn plex_queue_is_rejected_wrong_kind() {
    // `queue` is not a MediaServer verb (plex), so the download parse module is
    // never consulted for plex; the generic passthrough rejects it.
    let err = parse_args_from(["plex", "queue"]).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown command") && msg.contains("plex"),
        "plex queue should be rejected, got: {msg}"
    );
}
