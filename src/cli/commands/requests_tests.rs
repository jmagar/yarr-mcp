use crate::cli::command::Command;
use crate::cli::parse_args_from;
use serde_json::json;

#[test]
fn overseerr_requests_maps_kebab_to_requests() {
    let cmd = parse_args_from(["overseerr", "requests"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "requests",
            params: json!({ "service": "overseerr" }),
        }
    );
}

#[test]
fn overseerr_requests_parses_filter_take_skip() {
    let cmd = parse_args_from([
        "overseerr",
        "requests",
        "--filter",
        "pending",
        "--take",
        "10",
        "--skip",
        "5",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "requests",
            params: json!({
                "service": "overseerr",
                "filter": "pending",
                "take": "10",
                "skip": "5"
            }),
        }
    );
}

#[test]
fn overseerr_request_create_parses_media_type_id_and_confirm() {
    let cmd = parse_args_from([
        "overseerr",
        "request",
        "--media-type",
        "movie",
        "--media-id",
        "123",
        "--confirm",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "request_create",
            params: json!({
                "service": "overseerr",
                "media_type": "movie",
                "media_id": "123",
                "confirm": true
            }),
        }
    );
}

#[test]
fn overseerr_request_create_parses_seasons() {
    let cmd = parse_args_from([
        "overseerr",
        "request",
        "--media-type",
        "tv",
        "--media-id",
        "1399",
        "--season",
        "1",
        "--season",
        "2",
        "--confirm",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "request_create",
            params: json!({
                "service": "overseerr",
                "media_type": "tv",
                "media_id": "1399",
                "seasons": ["1", "2"],
                "confirm": true
            }),
        }
    );
}

#[test]
fn request_create_requires_media_type_and_id() {
    let err = parse_args_from(["overseerr", "request", "--media-id", "1"]).unwrap_err();
    assert!(err.to_string().contains("--media-type"));
    let err = parse_args_from(["overseerr", "request", "--media-type", "movie"]).unwrap_err();
    assert!(err.to_string().contains("--media-id"));
}

#[test]
fn overseerr_approve_parses_id_and_confirm() {
    let cmd = parse_args_from(["overseerr", "approve", "--id", "5", "--confirm"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "request_approve",
            params: json!({ "service": "overseerr", "id": "5", "confirm": true }),
        }
    );
}

#[test]
fn overseerr_decline_parses_id() {
    let cmd = parse_args_from(["overseerr", "decline", "--id", "5", "--confirm"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "request_decline",
            params: json!({ "service": "overseerr", "id": "5", "confirm": true }),
        }
    );
}

#[test]
fn approve_requires_id() {
    let err = parse_args_from(["overseerr", "approve", "--confirm"]).unwrap_err();
    assert!(err.to_string().contains("--id"));
}

#[test]
fn overseerr_search_parses_query() {
    // `search` maps to the non-colliding `request_search` action (ArrManager owns
    // the bare `search` action name).
    let cmd = parse_args_from(["overseerr", "search", "--query", "dune"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "request_search",
            params: json!({ "service": "overseerr", "query": "dune" }),
        }
    );
}

#[test]
fn search_requires_query() {
    let err = parse_args_from(["overseerr", "search"]).unwrap_err();
    assert!(err.to_string().contains("--query"));
}

#[test]
fn sonarr_requests_rejected_wrong_kind() {
    // `requests` is a Requests verb; sonarr is ArrManager, so its parse module is
    // never consulted and the generic passthrough rejects the unknown verb.
    let err = parse_args_from(["sonarr", "requests"]).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown command") && msg.contains("sonarr"),
        "sonarr requests should be rejected, got: {msg}"
    );
}
