use super::*;
use crate::config::ServiceKind;

#[test]
fn action_metadata_matches_rustarr_surface() {
    assert_eq!(
        action_names(),
        vec![
            "service_status",
            "api_get",
            "api_post",
            "api_put",
            "api_delete",
            "help",
            "codemode",
            "op",
            "snippet_list",
            "snippet_save",
            "snippet_run",
            "snippet_delete",
        ]
    );
    assert_eq!(required_scope_for_action("api_get"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_post"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_put"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("api_delete"), Some(WRITE_SCOPE));
    assert_eq!(required_scope_for_action("help"), None);
    assert_eq!(required_scope_for_action("codemode"), Some(WRITE_SCOPE));
    // `codemode` is MCP-only (and CLI via the infra path), so it is excluded from
    // the REST action surface.
    assert_eq!(
        rest_action_names(),
        vec![
            "service_status",
            "api_get",
            "api_post",
            "api_put",
            "api_delete",
            "help"
        ]
    );
    assert_eq!(
        mcp_only_action_names(),
        vec![
            "codemode",
            "op",
            "snippet_list",
            "snippet_save",
            "snippet_run",
            "snippet_delete"
        ]
    );
    assert_eq!(required_scope_for_action("snippet_list"), Some(READ_SCOPE));
    assert_eq!(required_scope_for_action("snippet_save"), Some(WRITE_SCOPE));
    // Snippet deletes are mutating-not-destructive.
    assert!(!action_is_destructive("snippet_delete"));
}

/// P2-4: every param a curated command declares (required + optional) — except
/// the always-string, globally-declared `service` — MUST carry a `ParamType` in
/// the command's `typed_params`, so the schema generator never falls back to the
/// `string` default for a param a handler treats as integer/array/boolean.
#[test]
fn typed_params_cover_every_declared_param() {
    for cmd in curated_commands() {
        for p in cmd.required_params.iter().chain(cmd.optional_params) {
            if *p == "service" {
                continue;
            }
            let declared = cmd.typed_params.iter().any(|(name, _)| name == p);
            assert!(
                declared,
                "command `{}` declares param `{p}` without a ParamType in typed_params",
                cmd.name
            );
        }
        // And every typed_params entry corresponds to a declared param (no orphans).
        for (name, _) in cmd.typed_params {
            let declared = cmd
                .required_params
                .iter()
                .chain(cmd.optional_params)
                .any(|p| p == name);
            assert!(
                declared,
                "command `{}` types param `{name}` that is not in required/optional params",
                cmd.name
            );
        }
    }
}

/// A given param NAME is a SHARED schema property under `additionalProperties:false`,
/// so it must carry ONE consistent type across every command that declares it —
/// otherwise the schema can only advertise one of them and `curated_param_type`'s
/// first-seen choice would silently mistype the others.
#[test]
fn typed_params_are_consistent_across_commands() {
    use std::collections::HashMap;
    let mut seen: HashMap<&str, ParamType> = HashMap::new();
    for cmd in curated_commands() {
        for (name, ty) in cmd.typed_params {
            if let Some(prev) = seen.get(name) {
                assert_eq!(
                    *prev, *ty,
                    "param `{name}` is typed inconsistently across commands ({prev:?} vs {ty:?})"
                );
            } else {
                seen.insert(name, *ty);
            }
        }
    }
}

#[test]
fn unknown_action_denies() {
    assert_eq!(required_scope_for_action("nope"), Some(DENY_SCOPE));
    assert!(!is_known_action("nope"));
}

#[test]
fn no_read_only_curated_command_carries_write_scope() {
    // Security S2: a non-mutating curated command must NEVER require write scope,
    // so read-only tokens work for dashboards.
    for cmd in curated_commands() {
        if !cmd.mutates {
            assert_ne!(
                cmd.required_scope, WRITE_SCOPE,
                "read-only command {} must not require write scope",
                cmd.name
            );
        }
    }
}

#[test]
fn action_allowed_for_kind_allows_infra_for_all_kinds() {
    for kind in ServiceKind::ALL {
        for action in action_names() {
            assert!(
                action_allowed_for_kind(action, kind),
                "infra action {action} should be allowed for {}",
                kind.as_str()
            );
        }
        // Unknown action fails closed for every kind.
        assert!(!action_allowed_for_kind("totally_unknown", kind));
    }
}

#[test]
fn valid_actions_for_kind_includes_infra() {
    let valid = valid_actions_for_kind(ServiceKind::Plex);
    assert!(valid.contains(&"service_status"));
    assert!(valid.contains(&"help"));
}

#[test]
fn curated_registry_populated_with_doc_based_commands() {
    // The doc-based capabilities (download, stats) keep curated commands; an
    // unknown name still misses. (Spec-backed kinds use generated ops instead.)
    assert!(!curated_commands().is_empty());
    assert!(curated_command("anything").is_none());
    assert!(curated_command("download_queue").is_some());
    assert!(curated_command("stats_activity").is_some());
}

#[test]
fn all_action_names_unions_generic_and_curated() {
    // Generic action names come first, then every curated command name.
    let names = all_action_names();
    for generic in action_names() {
        assert!(names.contains(&generic), "missing generic action {generic}");
    }
    for curated in curated_command_names() {
        assert!(names.contains(&curated), "missing curated action {curated}");
    }
    assert!(!curated_command_names().is_empty());
    // The curated param union is first-seen-ordered and de-duplicated. C1 read
    // commands contribute only `service`; C2 write commands add the selectors and
    // safety flags. Assert the union contains each declared param without pinning
    // exact ordering (so later beads can extend it freely).
    let params = curated_param_names();
    for expected in [
        "service",
        "url",
        "start",
        "length",
        "user",
        "id",
        "hash",
        "delete_files",
        "confirm",
    ] {
        assert!(
            params.contains(&expected),
            "curated param union missing {expected}: {params:?}"
        );
    }
}

#[test]
fn required_params_mirror_parser_contract() {
    assert_eq!(required_params_for_action("help"), Vec::<&str>::new());
    assert_eq!(
        required_params_for_action("service_status"),
        vec!["service"]
    );
    assert_eq!(
        required_params_for_action("api_get"),
        vec!["service", "path"]
    );
    // confirm is no longer a required param for the write passthroughs: api_post/
    // api_put run immediately, and the destructive api_delete obtains confirmation
    // out-of-band (MCP elicitation / CLI --confirm).
    assert_eq!(
        required_params_for_action("api_post"),
        vec!["service", "path"]
    );
    assert_eq!(
        required_params_for_action("api_delete"),
        vec!["service", "path"]
    );
}

#[test]
fn allowed_kind_names_covers_all_kinds_for_infra() {
    let all = allowed_kind_names_for_action("api_get");
    assert_eq!(all.len(), ServiceKind::ALL.len());
    // Unknown action → no allowed kinds (fail closed).
    assert!(allowed_kind_names_for_action("unknown").is_empty());
}

#[test]
fn action_is_destructive_covers_exactly_the_gated_set() {
    // The generic destructive passthrough + the curated deletes (download + stats).
    for destructive in ["api_delete", "download_remove", "stats_delete_image_cache"] {
        assert!(
            action_is_destructive(destructive),
            "{destructive} must be destructive (gated)"
        );
    }
    // Representative non-destructive writes, reads, and the other passthroughs
    // must NOT be gated.
    for plain in [
        "api_get",
        "api_post",
        "api_put",
        "set_quality",
        "add",
        "monitor",
        "search",
        "download_add",
        "media_scan",
        "request_create",
        "request_decline",
        "indexer_test",
        "stats_refresh_libraries",
        "list",
        "help",
        "unknown_action",
    ] {
        assert!(
            !action_is_destructive(plain),
            "{plain} must NOT be destructive"
        );
    }

    // SSOT invariant: for every curated command, action_is_destructive agrees
    // with the descriptor's `destructive` flag.
    for cmd in curated_commands() {
        assert_eq!(
            action_is_destructive(cmd.name),
            cmd.destructive,
            "{} destructive flag vs action_is_destructive mismatch",
            cmd.name
        );
    }
}

#[test]
fn capability_digest_lists_doc_based_commands() {
    // The digest renders the doc-based capabilities (download, stats) with their
    // commands and the kinds that share each capability.
    let digest = capability_digest().expect("digest should exist with download/stats commands");
    assert!(
        digest.contains("download_client("),
        "digest should label download capability: {digest}"
    );
    assert!(
        digest.contains("qbittorrent") || digest.contains("sabnzbd"),
        "download kinds should appear: {digest}"
    );
    assert!(
        digest.contains("download_queue"),
        "digest should list commands: {digest}"
    );
}

#[test]
fn download_commands_valid_only_for_download_kinds() {
    // A curated download command is allowed for sabnzbd/qbittorrent but rejected
    // for a non-download kind like plex.
    assert!(action_allowed_for_kind(
        "download_queue",
        ServiceKind::Qbittorrent
    ));
    assert!(action_allowed_for_kind(
        "download_queue",
        ServiceKind::Sabnzbd
    ));
    assert!(!action_allowed_for_kind(
        "download_queue",
        ServiceKind::Plex
    ));
    // And it appears in the valid-action list only for download kinds.
    assert!(valid_actions_for_kind(ServiceKind::Qbittorrent).contains(&"download_queue"));
    assert!(!valid_actions_for_kind(ServiceKind::Plex).contains(&"download_queue"));
    // allowed-kind names for a download command are a strict subset.
    let kinds = allowed_kind_names_for_action("download_queue");
    assert!(kinds.contains(&"sabnzbd") && kinds.contains(&"qbittorrent"));
    assert!(!kinds.contains(&"plex"));
    assert!(kinds.len() < ServiceKind::ALL.len());
}

/// C9 + C10: GenericOnly kinds (bazarr deferred from curated, plus tracearr)
/// expose ONLY the infra/passthrough actions —
/// every curated command is rejected, and infra actions remain available.
#[test]
fn generic_only_kinds_reject_all_curated_commands() {
    let generic_kinds = [ServiceKind::Bazarr, ServiceKind::Tracearr];
    // A representative curated command from every other capability family.
    let curated = [
        "list",           // ArrManager
        "set_quality",    // ArrManager (write)
        "indexers",       // Indexer
        "download_queue", // DownloadClient
        "media_sessions", // MediaServer
        "requests",       // Requests
        "stats_activity", // Stats
    ];
    let infra = [
        "service_status",
        "api_get",
        "api_post",
        "api_put",
        "api_delete",
        "help",
    ];

    for kind in generic_kinds {
        assert_eq!(
            kind.capability(),
            crate::capability::Capability::GenericOnly
        );
        // No curated command is valid for a GenericOnly kind.
        for action in curated {
            assert!(
                !action_allowed_for_kind(action, kind),
                "{kind:?} must reject curated action `{action}`"
            );
        }
        // Infra/passthrough actions remain available.
        for action in infra {
            assert!(
                action_allowed_for_kind(action, kind),
                "{kind:?} must allow infra action `{action}`"
            );
        }
        // The valid-action list is exactly the infra set — no curated leakage.
        let valid = valid_actions_for_kind(kind);
        for action in curated {
            assert!(
                !valid.contains(&action),
                "{kind:?} valid-actions must not contain curated `{action}`"
            );
        }
        assert_eq!(
            valid.len(),
            action_names().len(),
            "{kind:?} should expose only the infra actions"
        );
    }
}
