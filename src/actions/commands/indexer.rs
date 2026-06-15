//! Indexer (Prowlarr) curated command descriptors (C4).
//!
//! The per-capability const slice the registry concatenates at its single
//! extension point (`build_curated_commands`). Each
//! [`CommandDescriptor`] is the SSOT for one curated command — its scope, params,
//! allowed kinds (via `capability` = [`Capability::Indexer`], so only Prowlarr),
//! schema fragment, help line, and handler.
//!
//! The registry is keyed by action name, so these use `indexer_`-prefixed names
//! to avoid colliding with the ArrManager `search` command — the CLI maps the
//! friendlier kebab verbs (`search`/`test`/`stats`) onto them.
//!
//! Handlers are THIN adapters: extract params with the shared parse helpers and
//! call the corresponding `RustarrService` method. No business logic here — the
//! path/slim/confirm logic lives in `crate::app::indexer`.

use serde_json::Value;

use crate::actions::model::{READ_SCOPE, WRITE_SCOPE};
use crate::actions::parse::{bool_arg, i64_arg, i64_array_arg, string_arg};
use crate::actions::registry::{
    CommandDescriptor, CommandFuture,
    ParamType::{Boolean, IntegerArray, String as StringParam},
};
use crate::app::RustarrService;
use crate::capability::Capability;

/// The Indexer (Prowlarr) curated commands.
pub const INDEXER_COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        name: "indexers",
        capability: Capability::Indexer,
        description: "list the configured indexers (id, name, enable, protocol, priority), slimmed.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        typed_params: &[],
        handler: handle_indexers,
    },
    CommandDescriptor {
        name: "indexer_search",
        capability: Capability::Indexer,
        description: "run a manual Newznab-style search across indexers (--query, optional --id \
             to restrict to specific indexer ids).",
        required_scope: READ_SCOPE,
        required_params: &["service", "query"],
        optional_params: &["ids"],
        confirm_required: false,
        mutates: false,
        typed_params: &[("query", StringParam), ("ids", IntegerArray)],
        handler: handle_search,
    },
    CommandDescriptor {
        name: "indexer_stats",
        capability: Capability::Indexer,
        description: "show per-indexer query/grab/failure counters (indexerstats), slimmed.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        typed_params: &[],
        handler: handle_stats,
    },
    CommandDescriptor {
        name: "indexer_test",
        capability: Capability::Indexer,
        description: "triggers an indexer health check (write); with --id tests one indexer, \
             without it tests all. Confirm required.",
        required_scope: WRITE_SCOPE,
        required_params: &["service"],
        optional_params: &["id", "confirm"],
        confirm_required: true,
        mutates: true,
        // `id` shares the global `id` schema property (String); the handler parses
        // it via `i64_arg`, which coerces numeric strings like "5".
        typed_params: &[("id", StringParam), ("confirm", Boolean)],
        handler: handle_test,
    },
];

// ── thin handler adapters (marshal params → service method) ──────────────────────

fn handle_indexers<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.indexer_list(&service).await
    })
}

fn handle_search<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let query = string_arg(args, "query")?;
        let ids = i64_array_arg(args, "ids");
        svc.indexer_search(&service, &query, &ids).await
    })
}

fn handle_stats<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.indexer_stats(&service).await
    })
}

fn handle_test<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        // Only "test all" when id is genuinely absent. A present-but-malformed id
        // must surface a clear error rather than silently testing every indexer.
        let id = if args.get("id").is_some() {
            Some(i64_arg(args, "id")?)
        } else {
            None
        };
        svc.indexer_test(&service, id, bool_arg(args, "confirm"))
            .await
    })
}

#[cfg(test)]
#[path = "indexer_tests.rs"]
mod tests;
