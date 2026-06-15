use crate::cli::command::Command;
use crate::cli::parse_args_from;
use serde_json::json;

#[test]
fn plex_sessions_maps_kebab_to_media_sessions() {
    let cmd = parse_args_from(["plex", "sessions"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "media_sessions",
            params: json!({ "service": "plex" }),
        }
    );
}

#[test]
fn jellyfin_libraries_maps_kebab_to_media_libraries() {
    let cmd = parse_args_from(["jellyfin", "libraries"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "media_libraries",
            params: json!({ "service": "jellyfin" }),
        }
    );
}

#[test]
fn jellyfin_search_parses_query() {
    // `search` maps to the non-colliding `media_search` action (ArrManager owns
    // the bare `search` action name).
    let cmd = parse_args_from(["jellyfin", "search", "--query", "dune"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "media_search",
            params: json!({ "service": "jellyfin", "query": "dune" }),
        }
    );
}

#[test]
fn plex_search_parses_query() {
    let cmd = parse_args_from(["plex", "search", "--query", "dune"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "media_search",
            params: json!({ "service": "plex", "query": "dune" }),
        }
    );
}

#[test]
fn search_requires_query() {
    let err = parse_args_from(["plex", "search"]).unwrap_err();
    assert!(err.to_string().contains("--query"));
}

#[test]
fn plex_scan_parses_library_and_confirm() {
    let cmd = parse_args_from(["plex", "scan", "--library", "3", "--confirm"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "media_scan",
            params: json!({ "service": "plex", "library": "3", "confirm": true }),
        }
    );
}

#[test]
fn jellyfin_scan_without_library() {
    // Jellyfin refresh is server-wide; --library may be omitted at the CLI.
    let cmd = parse_args_from(["jellyfin", "scan", "--confirm"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "media_scan",
            params: json!({ "service": "jellyfin", "confirm": true }),
        }
    );
}

#[test]
fn sonarr_sessions_rejected_wrong_kind() {
    // `sessions` is a MediaServer verb; sonarr is ArrManager, so its parse module
    // is never consulted and the generic passthrough rejects the unknown verb.
    let err = parse_args_from(["sonarr", "sessions"]).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown command") && msg.contains("sonarr"),
        "sonarr sessions should be rejected, got: {msg}"
    );
}
