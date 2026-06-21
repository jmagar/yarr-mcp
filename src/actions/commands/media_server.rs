//! MediaServer (Plex, Jellyfin) curated command descriptors (C6).
//!
//! The per-capability const slice the registry concatenates at its single
//! extension point (`build_curated_commands`). Each
//! [`CommandDescriptor`] is the SSOT for one curated command — its scope, params,
//! allowed kinds (via `capability` = [`Capability::MediaServer`], so only Plex +
//! Jellyfin), schema fragment, help line, and handler.
//!
//! ACTION-NAME UNIQUENESS: registry action names are GLOBALLY unique across
//! capabilities, and the ArrManager surface already owns a `search` command. To
//! stay collision-free (now and for future capabilities) every command here is
//! `media_`-prefixed (`media_sessions`, `media_libraries`, `media_search`,
//! `media_scan`); the CLI maps the friendlier kebab verbs (`sessions`,
//! `libraries`, `search`, `scan`) onto them.
//!
//! Handlers are THIN adapters: extract params with the shared parse helpers and
//! call the corresponding `RustarrService` method. No business logic here — the
//! per-server (Plex vs Jellyfin) path/accept-mime/slim/confirm logic lives in
//! `crate::app::media_server`.

use serde_json::Value;

use crate::actions::model::{READ_SCOPE, WRITE_SCOPE};
use crate::actions::parse::{optional_string, string_arg};
use crate::actions::registry::{
    CommandDescriptor, CommandFuture, ParamType::String as StringParam,
};
use crate::app::RustarrService;
use crate::capability::Capability;

/// The MediaServer (Plex, Jellyfin) curated commands.
pub const MEDIA_COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        name: "media_sessions",
        capability: Capability::MediaServer,
        description: "list active streaming sessions (plex /status/sessions, \
             jellyfin /Sessions), slimmed.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        typed_params: &[],
        handler: handle_sessions,
    },
    CommandDescriptor {
        name: "media_libraries",
        capability: Capability::MediaServer,
        description: "list libraries (plex /library/sections, jellyfin \
             /Library/VirtualFolders), slimmed.",
        required_scope: READ_SCOPE,
        required_params: &["service"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        typed_params: &[],
        handler: handle_libraries,
    },
    CommandDescriptor {
        name: "media_search",
        capability: Capability::MediaServer,
        description: "search the library by --query (plex /library/search, \
             jellyfin /Items with includeItemTypes), slimmed.",
        required_scope: READ_SCOPE,
        required_params: &["service", "query"],
        optional_params: &[],
        confirm_required: false,
        mutates: false,
        typed_params: &[("query", StringParam)],
        handler: handle_search,
    },
    CommandDescriptor {
        name: "media_scan",
        capability: Capability::MediaServer,
        description: "trigger a library scan/refresh (plex requires --library \
             section id; jellyfin refreshes server-wide) (write). Non-destructive — \
             runs immediately.",
        required_scope: WRITE_SCOPE,
        required_params: &["service"],
        optional_params: &["library"],
        confirm_required: false,
        mutates: true,
        typed_params: &[("library", StringParam)],
        handler: handle_scan,
    },
];

// ── thin handler adapters (marshal params → service method) ──────────────────────

fn handle_sessions<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.media_sessions(&service).await
    })
}

fn handle_libraries<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        svc.media_libraries(&service).await
    })
}

fn handle_search<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let query = string_arg(args, "query")?;
        svc.media_search(&service, &query).await
    })
}

fn handle_scan<'a>(svc: &'a RustarrService, args: &'a Value) -> CommandFuture<'a> {
    Box::pin(async move {
        let service = string_arg(args, "service")?;
        let library = optional_string(args, "library");
        svc.media_scan(&service, library.as_deref()).await
    })
}

#[cfg(test)]
#[path = "media_server_tests.rs"]
mod tests;
