//! Mechanical CLI ↔ MCP parity guard (bead rustarr-zha.16 / Z1).
//!
//! The CLAUDE.md "CLI ↔ MCP action parity" table is documentation that drifts.
//! THIS test is the actual guard: it iterates the curated-command descriptor
//! registry and proves, for every curated command, that the command is reachable
//! on BOTH surfaces:
//!
//!   1. MCP — the command name appears in the generated MCP schema action enum
//!      (`all_action_names()`, which is exactly what `properties::properties()`
//!      emits as `inputSchema.properties.action.enum`).
//!   2. CLI — parsing `yarr <service> <friendly-verb> [minimal flags]` for a
//!      service of the command's capability produces a `Command::Curated` whose
//!      `action` is that command's registry name.
//!
//! It also asserts the confirm/dry-run contract is internally consistent
//! (`mutates => destructive`) and that the per-capability CLI `VERBS` tables
//! neither over- nor under-cover the registry (no orphan verbs, no uncovered
//! descriptors). Together these mean a new curated command cannot ship reachable
//! on one surface but not the other — the failure is a compile/test failure, not
//! a silent doc drift.
//!
//! No live services: parsing is pure and the MCP enum is derived from static
//! registry data.

use yarr::{
    Capability, Command, ServiceKind, action_is_destructive, all_action_names,
    capability_verb_tables, curated_commands, parse_args_from,
};

/// A representative service name for each capability, used to drive CLI parsing.
fn representative_service(cap: Capability) -> &'static str {
    ServiceKind::ALL
        .iter()
        .find(|k| k.capability() == cap)
        .map(|k| k.as_str())
        .unwrap_or_else(|| panic!("capability {cap:?} has no representative ServiceKind"))
}

/// Minimal valid flag set for a curated command whose CLI parser requires flags.
/// Commands not listed here take only the positional service. The exact values
/// are irrelevant — parity only cares that parsing succeeds and yields the right
/// `Command::Curated { action, .. }`.
fn minimal_flags(action: &str) -> &'static [&'static str] {
    match action {
        // ArrManager write/intent verbs.
        "set_quality" => &["--from", "SD", "--to", "HD", "--title", "Foo"],
        "add" => &[
            "--term",
            "Foo",
            "--quality-profile",
            "HD",
            "--root-folder",
            "/m",
        ],
        "delete" => &["--id", "1"],
        // Indexer.
        "indexer_search" => &["--query", "foo"],
        // DownloadClient.
        "download_add" => &["--url", "http://example/x.torrent"],
        "download_remove" => &["--id", "1"],
        // MediaServer.
        "media_search" => &["--query", "foo"],
        // Requests.
        "request_create" => &["--media-type", "movie", "--media-id", "1"],
        "request_approve" | "request_decline" => &["--id", "1"],
        "request_search" => &["--query", "foo"],
        // Tracearr.
        "trace_terminate_stream" => &["--id", "stream-1"],
        _ => &[],
    }
}

/// Parse `yarr <service> <verb> [flags]` and return the resulting `Command`.
fn parse_cli(service: &str, verb: &str, flags: &[&str]) -> Command {
    let mut args: Vec<String> = vec![service.to_string(), verb.to_string()];
    args.extend(flags.iter().map(|s| s.to_string()));
    parse_args_from(args)
        .unwrap_or_else(|e| panic!("`yarr {service} {verb} ...` failed to parse: {e}"))
        .unwrap_or_else(|| panic!("`yarr {service} {verb} ...` produced no command"))
}

/// Every curated command name is present in the MCP schema action enum.
#[test]
fn every_curated_command_is_in_the_mcp_action_enum() {
    let enum_names = all_action_names();
    for cmd in curated_commands() {
        assert!(
            enum_names.contains(&cmd.name),
            "curated command `{}` is missing from the MCP action enum (all_action_names())",
            cmd.name
        );
    }
}

/// Every curated command is reachable from the CLI: parsing
/// `yarr <service> <friendly-verb> [minimal flags]` yields a
/// `Command::Curated` with that command's registry action name.
#[test]
fn every_curated_command_is_reachable_from_the_cli() {
    for (cap, verbs) in capability_verb_tables() {
        let service = representative_service(*cap);
        for (verb, action) in *verbs {
            let cmd = parse_cli(service, verb, minimal_flags(action));
            match cmd {
                Command::Curated { action: got, .. } => assert_eq!(
                    got, *action,
                    "`yarr {service} {verb}` produced action `{got}`, expected `{action}`"
                ),
                other => {
                    panic!("`yarr {service} {verb}` produced {other:?}, expected Command::Curated")
                }
            }
        }
    }
}

/// The CLI verb tables and the registry cover EXACTLY the same set of curated
/// actions — no orphan CLI verbs (mapping to a non-existent action) and no
/// registry descriptors without a CLI verb. This is the bidirectional half of
/// parity: neither surface may have a command the other lacks.
#[test]
fn cli_verb_tables_and_registry_cover_the_same_actions() {
    use std::collections::BTreeSet;

    let registry_actions: BTreeSet<&str> = curated_commands().iter().map(|c| c.name).collect();
    let cli_actions: BTreeSet<&str> = capability_verb_tables()
        .iter()
        .flat_map(|(_, verbs)| verbs.iter().map(|(_, action)| *action))
        .collect();

    let registry_only: Vec<&&str> = registry_actions.difference(&cli_actions).collect();
    let cli_only: Vec<&&str> = cli_actions.difference(&registry_actions).collect();

    assert!(
        registry_only.is_empty(),
        "curated descriptors with no CLI verb (MCP-only, breaks parity): {registry_only:?}"
    );
    assert!(
        cli_only.is_empty(),
        "CLI verbs mapping to non-existent registry actions (orphan verbs): {cli_only:?}"
    );
}

/// Each CLI verb table entry must declare the same capability as the registry
/// descriptor it maps to — a verb cannot be wired into the wrong service group.
#[test]
fn cli_verb_capability_matches_registry_capability() {
    for (cap, verbs) in capability_verb_tables() {
        for (verb, action) in *verbs {
            let descriptor = curated_commands()
                .iter()
                .find(|c| c.name == *action)
                .unwrap_or_else(|| panic!("CLI verb `{verb}` maps to unknown action `{action}`"));
            assert_eq!(
                descriptor.capability, *cap,
                "CLI verb `{verb}` (action `{action}`) is in the {cap:?} table but the \
                 descriptor declares {:?}",
                descriptor.capability
            );
        }
    }
}

/// The confirm contract is internally consistent: a command is confirm-gated iff
/// it is DESTRUCTIVE, and any gated command necessarily mutates. (The CLI and MCP
/// shims share `execute_service_action`, so this single contract governs both
/// surfaces; the MCP shim turns the gate into an elicitation prompt, the CLI into
/// `--confirm`.)
#[test]
fn only_destructive_commands_are_confirm_gated() {
    for cmd in curated_commands() {
        // `destructive` is the SSOT for "destructive" on curated commands.
        assert_eq!(
            cmd.destructive,
            action_is_destructive(cmd.name),
            "curated command `{}`: destructive must equal action_is_destructive",
            cmd.name
        );
        // A gated command must also mutate (you can't gate a read).
        if cmd.destructive {
            assert!(
                cmd.mutates,
                "curated command `{}` is confirm-gated but mutates=false",
                cmd.name
            );
        }
    }
}
