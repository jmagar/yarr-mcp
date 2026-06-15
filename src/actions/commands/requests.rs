//! Requests (Overseerr) curated command descriptors (C7).
//!
//! The per-capability const slice the registry concatenates at its single
//! extension point ([`crate::actions::registry::build_curated_commands`]). Each
//! [`CommandDescriptor`] is the SSOT for one curated command — its scope, params,
//! allowed kinds (via `capability` = [`Capability::Requests`], so only Overseerr),
//! schema fragment, help line, and handler.
//!
//! The registry is keyed by action name, so these use `request`-prefixed names to
//! avoid colliding with the ArrManager `search`/`add` commands and to keep the
//! mutation verbs unambiguous (`request_approve`/`request_decline`). The CLI maps
//! the friendlier kebab verbs (`requests`/`request`/`approve`/`decline`/`search`)
//! onto them.
//!
//! Handlers are THIN adapters: extract params with the shared parse helpers and
//! call the corresponding `RustarrService` method. No business logic here — the
//! path/slim/confirm/body logic lives in `crate::app::requests`.

use serde_json::Value;

use crate::actions::model::{READ_SCOPE, WRITE_SCOPE};
use crate::actions::parse::{
    bool_arg, i64_arg, i64_array_arg, optional_i64, optional_string, string_arg,
};
use crate::actions::registry::{
    CommandDescriptor, CommandFuture,
    ParamType::{Boolean, Integer, IntegerArray, String as StringParam},
};
use crate::app::RustarrService;
use crate::capability::Capability;

/// The Requests (Overseerr) curated commands.
pub const REQUEST_COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        name: "requests",
        capability: Capability::Requests,
        description: "list media requests (id, type, status, media, requestedBy), slimmed. \
             Optional --filter (pending|approved|available), --take, --skip.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &["filter", "take", "skip"],
        confirm_required: false,
        mutates: false,
        typed_params: &[
            ("filter", StringParam),
            ("take", Integer),
            ("skip", Integer),
        ],
        handler: handle_requests,
    },
    CommandDescriptor {
        name: "request_create",
        capability: Capability::Requests,
        description: "create a media request (--media-type movie|tv, --media-id TMDB id, \
             optional --season N for TV). Write; confirm required.",
        required_scope: WRITE_SCOPE,
        required_params: &["service", "media_type", "media_id"],
        optional_params: &["seasons", "confirm"],
        confirm_required: true,
        mutates: true,
        typed_params: &[
            ("media_type", StringParam),
            ("media_id", Integer),
            ("seasons", IntegerArray),
            ("confirm", Boolean),
        ],
        handler: handle_create,
    },
    CommandDescriptor {
        name: "request_approve",
        capability: Capability::Requests,
        description: "approve a pending request (--id). Requires MANAGE_REQUESTS \
             (admin API key). Write; confirm required.",
        required_scope: WRITE_SCOPE,
        required_params: &["service", "id"],
        optional_params: &["confirm"],
        confirm_required: true,
        mutates: true,
        typed_params: &[("id", StringParam), ("confirm", Boolean)],
        handler: handle_approve,
    },
    CommandDescriptor {
        name: "request_decline",
        capability: Capability::Requests,
        description: "decline a pending request (--id). Requires MANAGE_REQUESTS \
             (admin API key). Write; confirm required.",
        required_scope: WRITE_SCOPE,
        required_params: &["service", "id"],
        optional_params: &["confirm"],
        confirm_required: true,
        mutates: true,
        typed_params: &[("id", StringParam), ("confirm", Boolean)],
        handler: handle_decline,
    },
    CommandDescriptor {
        name: "request_search",
        capability: Capability::Requests,
        description: "search for titles to request (--query); results carry the TMDB \
             id to feed into request_create.",
        required_scope: READ_SCOPE,
        required_params: &["service", "query"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        typed_params: &[("query", StringParam)],
        handler: handle_search,
    },
];

// ── thin handler adapters (marshal params → service method) ──────────────────────

fn handle_requests<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let filter = optional_string(args, "filter");
        let take = optional_i64(args, "take")?;
        let skip = optional_i64(args, "skip")?;
        svc.req_list(&service, filter.as_deref(), take, skip).await
    })
}

fn handle_create<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let media_type = string_arg(args, "media_type")?;
        let media_id = i64_arg(args, "media_id")?;
        let seasons = i64_array_arg(args, "seasons");
        svc.req_create(
            &service,
            &media_type,
            media_id,
            &seasons,
            bool_arg(args, "confirm"),
        )
        .await
    })
}

fn handle_approve<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let id = i64_arg(args, "id")?;
        svc.req_approve(&service, id, bool_arg(args, "confirm"))
            .await
    })
}

fn handle_decline<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let id = i64_arg(args, "id")?;
        svc.req_decline(&service, id, bool_arg(args, "confirm"))
            .await
    })
}

fn handle_search<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let query = string_arg(args, "query")?;
        svc.req_search(&service, &query).await
    })
}

#[cfg(test)]
#[path = "requests_tests.rs"]
mod tests;
