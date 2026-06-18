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

use crate::actions::model::{READ_SCOPE, WRITE_SCOPE};
use crate::actions::parse::{
    bool_arg, i64_array_arg, optional_string, string_arg, string_array_arg,
};
use crate::actions::registry::{
    CommandDescriptor, CommandFuture,
    ParamType::{Boolean, IntegerArray, String as StringParam, StringArray},
};
use crate::app::RustarrService;
use crate::capability::Capability;

/// The ArrManager READ commands. C2 appends write/intent descriptors to its own
/// slice; both are concatenated by `registry::curated_commands()`.
pub const ARR_COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        name: "quality_profiles",
        capability: Capability::ArrManager,
        description: "list the configured quality profiles (id + name) for a sonarr/radarr service.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        typed_params: &[],
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
        typed_params: &[],
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
        typed_params: &[],
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
        typed_params: &[],
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
        typed_params: &[],
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
        typed_params: &[],
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
        typed_params: &[],
        handler: handle_health,
    },
    // ── C2 WRITE / intent commands ──────────────────────────────────────────────
    CommandDescriptor {
        name: "set_quality",
        capability: Capability::ArrManager,
        description: "bulk-change the quality profile of selected items by NAME (--from/--to). \
             Without confirm returns a dry-run preview; confirm applies a PUT /<res>/editor.",
        required_scope: WRITE_SCOPE,
        required_params: &["service", "to"],
        optional_params: &["from", "title", "ids", "confirm", "bulk"],
        confirm_required: true,
        mutates: true,
        typed_params: &[
            ("to", StringParam),
            ("from", StringParam),
            ("title", StringArray),
            ("ids", IntegerArray),
            ("confirm", Boolean),
            ("bulk", Boolean),
        ],
        handler: handle_set_quality,
    },
    CommandDescriptor {
        name: "search",
        capability: Capability::ArrManager,
        description: "start an ASYNC search job (POST /command); no selector searches the whole \
             monitored library. Fire-and-forget — does not poll. Confirm required.",
        required_scope: WRITE_SCOPE,
        required_params: &["service"],
        optional_params: &["ids", "confirm", "bulk"],
        confirm_required: true,
        mutates: true,
        typed_params: &[
            ("ids", IntegerArray),
            ("confirm", Boolean),
            ("bulk", Boolean),
        ],
        handler: handle_search,
    },
    CommandDescriptor {
        name: "refresh",
        capability: Capability::ArrManager,
        description: "start an ASYNC refresh/rescan job (POST /command). Fire-and-forget — does not \
             poll. Confirm required.",
        required_scope: WRITE_SCOPE,
        required_params: &["service"],
        optional_params: &["ids", "confirm", "bulk"],
        confirm_required: true,
        mutates: true,
        typed_params: &[
            ("ids", IntegerArray),
            ("confirm", Boolean),
            ("bulk", Boolean),
        ],
        handler: handle_refresh,
    },
    CommandDescriptor {
        name: "monitor",
        capability: Capability::ArrManager,
        description: "set selected items monitored=true via PUT /<res>/editor. Without confirm \
             previews; count-capped. Confirm required.",
        required_scope: WRITE_SCOPE,
        required_params: &["service"],
        optional_params: &["title", "ids", "confirm", "bulk"],
        confirm_required: true,
        mutates: true,
        typed_params: &[
            ("title", StringArray),
            ("ids", IntegerArray),
            ("confirm", Boolean),
            ("bulk", Boolean),
        ],
        handler: handle_monitor,
    },
    CommandDescriptor {
        name: "unmonitor",
        capability: Capability::ArrManager,
        description: "set selected items monitored=false via PUT /<res>/editor. Without confirm \
             previews; count-capped. Confirm required.",
        required_scope: WRITE_SCOPE,
        required_params: &["service"],
        optional_params: &["title", "ids", "confirm", "bulk"],
        confirm_required: true,
        mutates: true,
        typed_params: &[
            ("title", StringArray),
            ("ids", IntegerArray),
            ("confirm", Boolean),
            ("bulk", Boolean),
        ],
        handler: handle_unmonitor,
    },
    CommandDescriptor {
        name: "add",
        capability: Capability::ArrManager,
        description: "add an item: lookup by --term, then POST /<res> with --quality-profile and \
             --root-folder. Without confirm previews the resolved match.",
        required_scope: WRITE_SCOPE,
        required_params: &["service", "term", "quality_profile", "root_folder"],
        optional_params: &["confirm"],
        confirm_required: true,
        mutates: true,
        typed_params: &[
            ("term", StringParam),
            ("quality_profile", StringParam),
            ("root_folder", StringParam),
            ("confirm", Boolean),
        ],
        handler: handle_add,
    },
    CommandDescriptor {
        name: "delete",
        capability: Capability::ArrManager,
        description: "delete an item by --id via DELETE /<res>/{id}; --delete-files is opt-in. \
             Mutating: previews without confirm, applies only with confirm.",
        required_scope: WRITE_SCOPE,
        required_params: &["service", "id"],
        optional_params: &["delete_files", "confirm"],
        confirm_required: true,
        mutates: true,
        // `id` is advertised as a string (not integer): it's a SHARED schema
        // property and the DownloadClient `id`/`hash` are non-numeric (nzo_id /
        // torrent hash). `i64_arg` accepts numeric strings, so a string `id` is
        // compatible with the integer-coercing arr/requests handlers too.
        typed_params: &[
            ("id", StringParam),
            ("delete_files", Boolean),
            ("confirm", Boolean),
        ],
        handler: handle_delete,
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

// ── C2 write handler adapters (thin: marshal params → service method) ────────────

fn handle_set_quality<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let to = string_arg(args, "to")?;
        let from = optional_string(args, "from");
        let ids = i64_array_arg(args, "ids");
        let titles = string_array_arg(args, "title");
        svc.arr_set_quality(
            &service,
            crate::app::arr::write::SetQualityRequest {
                from: from.as_deref(),
                to: &to,
                ids: &ids,
                titles: &titles,
                confirm: bool_arg(args, "confirm"),
                bulk: bool_arg(args, "bulk"),
            },
        )
        .await
    })
}

fn handle_search<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let ids = i64_array_arg(args, "ids");
        svc.arr_search(
            &service,
            &ids,
            bool_arg(args, "confirm"),
            bool_arg(args, "bulk"),
        )
        .await
    })
}

fn handle_refresh<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let ids = i64_array_arg(args, "ids");
        svc.arr_refresh(
            &service,
            &ids,
            bool_arg(args, "confirm"),
            bool_arg(args, "bulk"),
        )
        .await
    })
}

fn handle_monitor<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let ids = i64_array_arg(args, "ids");
        let titles = string_array_arg(args, "title");
        svc.arr_set_monitored(
            &service,
            &ids,
            &titles,
            true,
            bool_arg(args, "confirm"),
            bool_arg(args, "bulk"),
        )
        .await
    })
}

fn handle_unmonitor<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let ids = i64_array_arg(args, "ids");
        let titles = string_array_arg(args, "title");
        svc.arr_set_monitored(
            &service,
            &ids,
            &titles,
            false,
            bool_arg(args, "confirm"),
            bool_arg(args, "bulk"),
        )
        .await
    })
}

fn handle_add<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let term = string_arg(args, "term")?;
        let quality_profile = string_arg(args, "quality_profile")?;
        let root_folder = string_arg(args, "root_folder")?;
        svc.arr_add(
            &service,
            &term,
            &quality_profile,
            &root_folder,
            bool_arg(args, "confirm"),
        )
        .await
    })
}

fn handle_delete<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let id = crate::actions::parse::i64_arg(args, "id")?;
        svc.arr_delete(
            &service,
            id,
            bool_arg(args, "delete_files"),
            bool_arg(args, "confirm"),
        )
        .await
    })
}

#[cfg(test)]
#[path = "arr_tests.rs"]
mod tests;
