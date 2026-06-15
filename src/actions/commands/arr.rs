//! ArrManager curated command descriptors (C1: READ commands).
//!
//! This is the per-capability const slice the registry concatenates at its
//! single extension point ([`crate::actions::registry::curated_commands`]). Each
//! [`CommandDescriptor`] is the SSOT for one curated command — its scope, params,
//! allowed kinds (via `capability`), schema fragment, help line, and handler.
//!
//! Handlers are THIN adapters: extract params with the shared parse helpers and
//! call the corresponding `RustarrService` method. No business logic here — the
//! resource-noun/path/slim logic lives in `crate::app::arr`.

use serde_json::Value;

use crate::actions::model::READ_SCOPE;
use crate::actions::parse::string_arg;
use crate::actions::registry::{CommandDescriptor, CommandFuture};
use crate::app::RustarrService;
use crate::capability::Capability;

/// The ArrManager READ commands. C2 appends write/intent descriptors to its own
/// slice; both are concatenated by `registry::curated_commands()`.
pub const ARR_COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        name: "quality_profiles",
        capability: Capability::ArrManager,
        description:
            "list the configured quality profiles (id + name) for a sonarr/radarr service.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        handler: handle_quality_profiles,
    },
    CommandDescriptor {
        name: "list",
        capability: Capability::ArrManager,
        description: "list the managed library (series for sonarr, movies for radarr), slimmed.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        handler: handle_list,
    },
    CommandDescriptor {
        name: "wanted",
        capability: Capability::ArrManager,
        description: "list monitored items not yet acquired (wanted/missing).",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        handler: handle_wanted,
    },
    CommandDescriptor {
        name: "queue",
        capability: Capability::ArrManager,
        description: "show the current download/import queue.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        handler: handle_queue,
    },
    CommandDescriptor {
        name: "history",
        capability: Capability::ArrManager,
        description: "show recent grab/import/delete history events.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        handler: handle_history,
    },
    CommandDescriptor {
        name: "rootfolders",
        capability: Capability::ArrManager,
        description: "list configured root folders with free/total disk space.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        handler: handle_rootfolders,
    },
    CommandDescriptor {
        name: "health",
        capability: Capability::ArrManager,
        description: "list health-check messages (empty means healthy).",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        handler: handle_health,
    },
];

// ── thin handler adapters ───────────────────────────────────────────────────────

fn handle_quality_profiles<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.arr_quality_profiles(&service).await
    })
}

fn handle_list<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.arr_list(&service).await
    })
}

fn handle_wanted<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.arr_wanted(&service).await
    })
}

fn handle_queue<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.arr_queue(&service).await
    })
}

fn handle_history<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.arr_history(&service).await
    })
}

fn handle_rootfolders<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.arr_rootfolders(&service).await
    })
}

fn handle_health<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.arr_health(&service).await
    })
}

#[cfg(test)]
#[path = "arr_tests.rs"]
mod tests;
