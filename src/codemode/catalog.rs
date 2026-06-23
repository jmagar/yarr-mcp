//! Code Mode discovery catalog.
//!
//! Builds a list of **fully-qualified callables** for the *configured* services,
//! derived from the action registry (the SSOT). Each curated command and the
//! per-service status verb becomes one entry whose `path` is exactly what a script
//! calls — `sonarr.list`, `radarr.add`, `plex.media_sessions` — with the service
//! baked in. This mirrors lab's `codemode.<upstream>.<tool>` and Cloudflare's
//! `<connector>.<method>`: the agent searches by capability and gets back a
//! self-contained callable, so it never enumerates services or passes a `service`
//! param.
//!
//! The catalog is serialized once and injected as `globalThis.__codemodeCatalog`;
//! the pure-JS `codemode.search`/`codemode.describe` helpers (see [`super::proxy`])
//! read it with zero host round-trips. The raw passthrough client is documented by
//! four service-agnostic `api.<service>.{get,post,put,delete}` entries.

use serde::Serialize;

use crate::actions::{
    CommandDescriptor, READ_SCOPE, WRITE_SCOPE, action_is_destructive, curated_command,
    curated_commands, required_params_for_action, required_scope_for_action,
};
use crate::config::ServiceKind;

/// One catalog row, surfaced to scripts via `codemode.search`/`describe`.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    /// The exact in-sandbox callable, e.g. `sonarr.list` or `api.<service>.get`.
    pub path: String,
    /// The configured service the callable is bound to. `None` for the generic
    /// raw-API client docs (those are service-agnostic).
    pub service: Option<String>,
    /// The method/action name, e.g. `list`.
    pub method: &'static str,
    /// `"curated"` (capability command), `"generic"` (infra verb / raw API).
    pub kind: &'static str,
    /// `"read"` / `"write"` / `"public"`.
    pub scope: &'static str,
    /// True only for destructive deletes — these are refused inside Code Mode.
    pub destructive: bool,
    /// Capability class for curated commands, else `"infra"`.
    pub capability: String,
    /// Required params, with the baked-in `service` dropped (it's never passed).
    pub required_params: Vec<&'static str>,
    pub description: &'static str,
}

/// The action names exposed as `<service>.<method>()` callables for a kind: the
/// per-service status verb plus every curated command whose capability matches the
/// kind. Generic passthroughs (`api_*`) are NOT here — they live under the separate
/// `api.<service>` client. `help`/`codemode`/`snippet_*` are not service-scoped.
pub fn service_action_names(kind: ServiceKind) -> Vec<&'static str> {
    let cap = kind.capability();
    let mut names = vec!["service_status"];
    names.extend(
        curated_commands()
            .iter()
            .filter(|cmd| cmd.capability == cap)
            .map(|cmd| cmd.name),
    );
    names
}

/// Build the catalog for the configured services. One entry per `(service, action)`
/// callable, plus four service-agnostic raw-API client entries.
pub fn build_catalog(services: &[(String, ServiceKind)]) -> Vec<CatalogEntry> {
    let mut out: Vec<CatalogEntry> = Vec::new();
    for (name, kind) in services {
        for action in service_action_names(*kind) {
            out.push(service_entry(name, action));
        }
    }
    out.extend(generic_api_entries());
    out
}

/// A `<service>.<action>` callable entry.
fn service_entry(service: &str, action: &'static str) -> CatalogEntry {
    let cmd: Option<&'static CommandDescriptor> = curated_command(action);
    let scope = match required_scope_for_action(action) {
        Some(WRITE_SCOPE) => "write",
        Some(READ_SCOPE) => "read",
        _ => "public",
    };
    CatalogEntry {
        path: format!("{service}.{action}"),
        service: Some(service.to_string()),
        method: action,
        kind: if cmd.is_some() { "curated" } else { "generic" },
        scope,
        destructive: action_is_destructive(action),
        capability: cmd
            .map(|c| format!("{:?}", c.capability))
            .unwrap_or_else(|| "infra".to_string()),
        // `service` is baked into the callable, so it is never a param the script
        // passes — drop it from the advertised signature.
        required_params: required_params_for_action(action)
            .into_iter()
            .filter(|param| *param != "service")
            .collect(),
        description: cmd
            .map(|c| c.description)
            .unwrap_or_else(|| generic_description(action)),
    }
}

/// The four service-agnostic raw-API client callables, documented once.
fn generic_api_entries() -> Vec<CatalogEntry> {
    [
        ("api.<service>.get", "api_get"),
        ("api.<service>.post", "api_post"),
        ("api.<service>.put", "api_put"),
        ("api.<service>.delete", "api_delete"),
    ]
    .into_iter()
    .map(|(path, action)| CatalogEntry {
        path: path.to_string(),
        service: None,
        method: action,
        kind: "generic",
        scope: "write",
        destructive: action_is_destructive(action),
        capability: "infra".to_string(),
        required_params: vec!["path"],
        description: generic_description(action),
    })
    .collect()
}

/// Short prose for the infra verbs (curated commands carry their own).
fn generic_description(name: &str) -> &'static str {
    match name {
        "service_status" => "Call the service's default status endpoint.",
        "api_get" => "Raw GET passthrough: api.<service>.get(path).",
        "api_post" => "Raw POST passthrough (runs immediately): api.<service>.post(path, body).",
        "api_put" => "Raw PUT passthrough (runs immediately): api.<service>.put(path, body).",
        "api_delete" => "Raw DELETE passthrough. Destructive — refused inside Code Mode.",
        _ => "",
    }
}

/// The catalog serialized to a JSON array string for preamble injection.
pub fn catalog_json(services: &[(String, ServiceKind)]) -> String {
    serde_json::to_string(&build_catalog(services)).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
