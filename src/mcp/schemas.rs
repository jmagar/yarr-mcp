//! Tool JSON schema for the service-named MCP tools.
//!
//! The schema is fully GENERATED from the action registry + capability map:
//!   - [`properties`] — the property set (generic params + curated params +
//!     verbose/fields), with the `action` enum coming from `action_names()` plus
//!     curated command names.
//!   - [`conditionals`] — action→required-params and action→allowed-kinds
//!     `allOf` fragments.
//!
//! Each advertised tool is named after one configured media-service kind
//! (`sonarr`, `radarr`, ...). Adding a curated command descriptor makes it
//! appear on every compatible service tool with no hand-maintained schema list.
//! The authoritative action×kind enforcement is the shared dispatch guard, not
//! these schema hints.
//!
//! NOTE: the substring `action_names()` is asserted by the schema-contract doc
//! test (`tests/template_invariants.rs`) — the enum is derived from action
//! metadata via `properties::properties()`, which calls `all_action_names()`
//! (generic `action_names()` plus curated names). Do not remove the reference in
//! this comment or the derivation in `properties`.

mod conditionals;
mod properties;

use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::actions::registry::ParamType;
use crate::actions::{
    ACTION_SPECS, CommandDescriptor, action_allowed_for_kind, curated_command,
    required_params_for_action, valid_actions_for_kind,
};
use crate::config::ServiceKind;

/// Cached JSON schema definitions (static data, built once at first call).
static TOOL_DEFINITIONS: OnceLock<Vec<Value>> = OnceLock::new();

pub(super) const SERVICE_TOOL_KINDS: &[ServiceKind] = &[
    ServiceKind::Sonarr,
    ServiceKind::Radarr,
    ServiceKind::Prowlarr,
    ServiceKind::Overseerr,
    ServiceKind::Tautulli,
    ServiceKind::Plex,
    ServiceKind::Tracearr,
    ServiceKind::Sabnzbd,
    ServiceKind::Qbittorrent,
    ServiceKind::Jellyfin,
    ServiceKind::Bazarr,
];

/// Return the JSON schema definitions for all tools (cached after first call).
///
/// Returns a `Vec<Value>` where each item is a tool definition object matching
/// the MCP `Tool` schema: `{ name, description, inputSchema }`.
///
/// This is also used by the schema resource (`rustarr://schema/mcp-tool`).
pub(super) fn tool_definitions() -> &'static Vec<Value> {
    TOOL_DEFINITIONS.get_or_init(build_tool_definitions)
}

/// Return tool definitions for a runtime-selected set of service kinds.
pub(super) fn tool_definitions_for_kinds(kinds: &[ServiceKind]) -> Vec<Value> {
    kinds
        .iter()
        .copied()
        .filter(|kind| SERVICE_TOOL_KINDS.contains(kind))
        .map(tool_definition)
        .collect()
}

/// Build the tool definitions. The action enum for each service is derived from
/// action metadata — see `action_names()` (via `properties::properties`).
fn build_tool_definitions() -> Vec<Value> {
    SERVICE_TOOL_KINDS
        .iter()
        .copied()
        .map(tool_definition)
        .collect()
}

fn tool_definition(kind: ServiceKind) -> Value {
    json!({
    "name": kind.as_str(),
    "description": tool_description(kind),
    "inputSchema": {
        "type": "object",
        "properties": properties::properties(kind),
        "required": ["action"],
        "additionalProperties": false,
        "allOf": conditionals::conditionals(kind),
        "x-rustarr-action-metadata": action_metadata(kind),
        "x-rustarr-service-metadata": service_metadata(kind),
        "x-rustarr-agent-guidance": agent_guidance(kind),
    }
    })
}

/// Tool description, with a registry-derived capability digest appended when
/// curated commands exist (AN-1) so agents can pick the right action up front.
fn tool_description(kind: ServiceKind) -> String {
    let actions = crate::actions::valid_actions_for_kind(kind).join(", ");
    format!(
        "{} media-service MCP tool. The service is implicit; pass action plus action-specific params. Valid actions: {actions}",
        kind.as_str()
    )
}

fn action_metadata(kind: ServiceKind) -> Value {
    let actions = valid_actions_for_kind(kind)
        .into_iter()
        .map(|action| match curated_command(action) {
            Some(command) => curated_action_metadata(kind, command),
            None => generic_action_metadata(kind, action),
        })
        .collect::<Vec<_>>();
    Value::Array(actions)
}

fn generic_action_metadata(kind: ServiceKind, action: &'static str) -> Value {
    let spec = ACTION_SPECS
        .iter()
        .find(|spec| spec.name == action)
        .expect("valid generic action should have an ActionSpec");
    json!({
        "name": spec.name,
        "kind": "generic",
        "scope": spec.required_scope.unwrap_or("public"),
        "transport": action_transport_label(spec.transport),
        "description": generic_action_description(spec.name),
        "required_params": required_params_for_action(spec.name)
            .into_iter()
            .filter(|param| *param != "service")
            .collect::<Vec<_>>(),
        "optional_params": generic_optional_params(spec.name),
        "destructive": generic_destructive(spec.name),
        "mutates": generic_mutates(spec.name),
        "allowed_kinds": allowed_kinds_for_action(spec.name),
        "available_on_this_tool": action_allowed_for_kind(spec.name, kind),
    })
}

fn curated_action_metadata(kind: ServiceKind, command: &CommandDescriptor) -> Value {
    json!({
        "name": command.name,
        "kind": "curated",
        "capability": format!("{:?}", command.capability),
        "scope": command.required_scope,
        "transport": "any",
        "description": command.description,
        "required_params": command.required_params
            .iter()
            .copied()
            .filter(|param| *param != "service")
            .collect::<Vec<_>>(),
        "optional_params": command.optional_params,
        "typed_params": command.typed_params
            .iter()
            .map(|(name, ty)| json!({ "name": name, "type": param_type_label(*ty) }))
            .collect::<Vec<_>>(),
        "destructive": command.destructive,
        "mutates": command.mutates,
        "allowed_kinds": allowed_kinds_for_action(command.name),
        "available_on_this_tool": action_allowed_for_kind(command.name, kind),
    })
}

fn service_metadata(kind: ServiceKind) -> Value {
    let descriptor = kind.descriptor();
    json!({
        "kind": kind.as_str(),
        "capability": format!("{:?}", descriptor.capability),
        "api_prefix": descriptor.api_prefix,
        "auth_style": format!("{:?}", descriptor.auth_style),
        "resource_noun": descriptor.resource_noun,
        "path_allowlist": descriptor.path_allowlist,
        "has_metadata_profiles": descriptor.has_metadata_profiles,
        "valid_actions": valid_actions_for_kind(kind),
    })
}

fn agent_guidance(kind: ServiceKind) -> Value {
    let mut read_first = vec!["service_status", "help"];
    if matches!(
        kind,
        ServiceKind::Sonarr
            | ServiceKind::Radarr
            | ServiceKind::Prowlarr
            | ServiceKind::Overseerr
            | ServiceKind::Tautulli
            | ServiceKind::Plex
            | ServiceKind::Sabnzbd
            | ServiceKind::Qbittorrent
            | ServiceKind::Jellyfin
    ) {
        read_first.insert(0, "integrations");
    }
    json!({
        "cost_order": ["read", "write"],
        "first_pass": read_first,
        "generic_passthrough": {
            "read": "api_get",
            "write": ["api_post", "api_put", "api_delete"],
            "path_allowlist": kind.descriptor().path_allowlist,
        },
        "write_guard": {
            "confirm_field": "confirm",
            "model": "Writes run immediately. Only DESTRUCTIVE deletes are gated: on the MCP \
                surface the client is prompted to confirm via elicitation before the delete runs; \
                passing confirm=true overrides the prompt (and is required for clients that cannot \
                elicit).",
            "gated_actions": "see x-rustarr-action-metadata[*].destructive (true == destructive/gated)"
        },
        "response_shaping": {
            "default": "slim",
            "opt_in_fields": ["verbose", "fields"]
        },
        "schema_resource": "rustarr://schema/mcp-tool"
    })
}

fn action_transport_label(transport: crate::actions::ActionTransport) -> &'static str {
    match transport {
        crate::actions::ActionTransport::Any => "any",
        crate::actions::ActionTransport::McpOnly => "mcp_only",
    }
}

fn param_type_label(ty: ParamType) -> &'static str {
    match ty {
        ParamType::String => "string",
        ParamType::Integer => "integer",
        ParamType::IntegerArray => "integer[]",
        ParamType::StringArray => "string[]",
        ParamType::Boolean => "boolean",
    }
}

fn generic_action_description(action: &str) -> &'static str {
    match action {
        "integrations" => "Return configured and supported service integrations.",
        "service_status" => "Call the default status endpoint for the implicit service kind.",
        "api_get" => "Run an allowlisted GET against the implicit upstream service.",
        "api_post" => {
            "Run an allowlisted POST against the implicit upstream service (runs immediately)."
        }
        "api_put" => {
            "Run an allowlisted PUT against the implicit upstream service (runs immediately)."
        }
        "api_delete" => {
            "Run an allowlisted DELETE against the implicit upstream service. Destructive: the MCP \
             client is prompted to confirm (elicitation); pass confirm=true to override."
        }
        "help" => "Return registry-derived action help.",
        "snippet_list" => "List saved Code Mode snippets (metadata).",
        "snippet_save" => "Save (create/overwrite) a named, reusable Code Mode snippet.",
        "snippet_run" => "Run a saved Code Mode snippet, binding `input` as `globalThis.input`.",
        "snippet_delete" => "Delete a saved Code Mode snippet (mutating, not destructive).",
        "codemode" => {
            "Run a JavaScript async arrow function (`code`) in an in-process QuickJS sandbox to \
             orchestrate rustarr in one call. Pass `code` as `async () => { ... }`; the sandbox \
             awaits its return and replies { result, calls, logs, artifacts, artifactsRunId? } — \
             only that envelope leaves the sandbox. Available inside `code`: \
             callTool(action, params) and tools.<action>(params) dispatch any rustarr action; \
             api.<service>.get/post/put/delete(path, body) call a service's raw upstream API \
             (e.g. api.sonarr.get('/series')); codemode.search(query) and codemode.describe(name) \
             discover actions AND upstream response types ON DEMAND — \
             codemode.describe('sonarr.SeriesResource') returns that type's TypeScript interface, so \
             you pull only the shapes you need instead of guessing fields; \
             codemode.run(name, input)/codemode.snippets() use saved snippets; \
             writeArtifact(path, content, options?) writes a sandboxed file; console.* is captured; \
             `input` holds the snippet input. \
             QuickJS limits (64 MiB heap / 30s wall); destructive deletes (api_delete) are refused \
             mid-script. Requires rustarr:write."
        }
        _ => "",
    }
}

fn generic_optional_params(action: &str) -> Vec<&'static str> {
    match action {
        "api_post" | "api_put" => vec!["body"],
        // `confirm` is the explicit override for the destructive DELETE (MCP
        // clients that can't elicit, or automation, pass it directly).
        "api_delete" => vec!["body", "confirm"],
        _ => Vec::new(),
    }
}

/// Whether the generic action is *destructive* (and therefore gated). Mirrors
/// [`crate::actions::action_is_destructive`] for the generic passthroughs — only
/// `api_delete` qualifies; `api_post`/`api_put` mutate but are not destructive.
fn generic_destructive(action: &str) -> bool {
    matches!(action, "api_delete")
}

/// Whether the generic action mutates upstream state. All three write
/// passthroughs mutate; only `api_delete` is *also* destructive (see
/// [`generic_destructive`]).
fn generic_mutates(action: &str) -> bool {
    // `codemode` is potentially-mutating: its script may perform non-destructive
    // writes (it cannot delete). `snippet_save/run/delete` mutate the snippet store
    // (and snippet_run may perform writes); none are destructive.
    matches!(
        action,
        "api_post"
            | "api_put"
            | "api_delete"
            | "codemode"
            | "snippet_save"
            | "snippet_run"
            | "snippet_delete"
    )
}

fn allowed_kinds_for_action(action: &str) -> Vec<&'static str> {
    SERVICE_TOOL_KINDS
        .iter()
        .copied()
        .filter(|kind| action_allowed_for_kind(action, *kind))
        .map(ServiceKind::as_str)
        .collect()
}

pub(super) fn service_tool_kind(name: &str) -> Option<ServiceKind> {
    SERVICE_TOOL_KINDS
        .iter()
        .copied()
        .find(|kind| kind.as_str() == name)
}

#[cfg(test)]
#[path = "schemas_tests.rs"]
mod tests;
