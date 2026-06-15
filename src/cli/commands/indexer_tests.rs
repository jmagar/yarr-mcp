use crate::cli::command::Command;
use crate::cli::parse_args_from;
use serde_json::json;

#[test]
fn prowlarr_indexers_parses_to_curated() {
    let cmd = parse_args_from(["prowlarr", "indexers"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "indexers",
            params: json!({ "service": "prowlarr" }),
        }
    );
}

#[test]
fn prowlarr_stats_maps_kebab_to_snake() {
    let cmd = parse_args_from(["prowlarr", "stats"]).unwrap().unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "indexer_stats",
            params: json!({ "service": "prowlarr" }),
        }
    );
}

#[test]
fn prowlarr_search_parses_query() {
    // `search` verb maps to the non-colliding `indexer_search` action.
    let cmd = parse_args_from(["prowlarr", "search", "--query", "ubuntu"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "indexer_search",
            params: json!({ "service": "prowlarr", "query": "ubuntu" }),
        }
    );
}

#[test]
fn prowlarr_search_collects_indexer_ids() {
    let cmd = parse_args_from([
        "prowlarr", "search", "--query", "ubuntu", "--id", "1", "--id", "2",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "indexer_search",
            params: json!({ "service": "prowlarr", "query": "ubuntu", "ids": ["1", "2"] }),
        }
    );
}

#[test]
fn prowlarr_search_requires_query() {
    let err = parse_args_from(["prowlarr", "search"]).unwrap_err();
    assert!(err.to_string().contains("--query"));
}

#[test]
fn prowlarr_test_parses_confirm_and_id() {
    let cmd = parse_args_from(["prowlarr", "test", "--id", "3", "--confirm"])
        .unwrap()
        .unwrap();
    assert_eq!(
        cmd,
        Command::Curated {
            action: "indexer_test",
            params: json!({ "service": "prowlarr", "id": "3", "confirm": true }),
        }
    );
}

#[test]
fn sonarr_indexers_is_rejected() {
    // `indexers` is not an ArrManager verb, so the arr parse module returns None
    // and the generic passthrough rejects it as an unknown command for sonarr.
    let err = parse_args_from(["sonarr", "indexers"]).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unknown command") && msg.contains("sonarr"),
        "sonarr indexers should be rejected, got: {msg}"
    );
}
