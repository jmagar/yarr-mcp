//! Code Mode discovery catalog.
//!
//! Builds a per-action metadata list **derived from the action registry** (the
//! SSOT) — never a hand-maintained list. It is serialized to JSON once and
//! injected into the preamble as `globalThis.__codemodeCatalog`; the pure-JS
//! `codemode.search`/`codemode.describe` helpers (see [`super::proxy`]) read it
//! with zero host round-trips, mirroring lab's sandbox-only discovery globals.
//!
//! Every field comes from `crate::actions` helpers (`required_scope_for_action`,
//! `action_is_destructive`, `required_params_for_action`, `curated_command`,
//! `allowed_kind_names_for_action`); the only local prose is a short description
//! for the fixed infra verbs (curated commands carry their own `description`).

use serde::Serialize;

use crate::actions::{
    READ_SCOPE, WRITE_SCOPE, action_is_destructive, all_action_names,
    allowed_kind_names_for_action, curated_command, required_params_for_action,
    required_scope_for_action,
};

/// One catalog row, surfaced to scripts via `codemode.search`/`describe`.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub name: &'static str,
    /// `"generic"` (infra `ACTION_SPECS`) or `"curated"` (capability command).
    pub kind: &'static str,
    /// `"read"` / `"write"` / `"public"`.
    pub scope: &'static str,
    /// True only for destructive deletes — these are refused inside Code Mode.
    pub destructive: bool,
    /// Capability class for curated commands, else `"infra"`.
    pub capability: String,
    /// Required params (the implicit `service` is dropped).
    pub required_params: Vec<&'static str>,
    /// Service-kind names this action may target.
    pub allowed_kinds: Vec<&'static str>,
    pub description: &'static str,
}

/// Build the catalog from the registry. Excludes `codemode` itself (a script can't
/// usefully discover the action it is already running). Deduped + name-sorted so
/// `describe`'s exact-match lookup and the uniqueness invariant hold.
pub fn build_catalog() -> Vec<CatalogEntry> {
    let mut names = all_action_names();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .filter(|name| *name != "codemode")
        .map(catalog_entry)
        .collect()
}

fn catalog_entry(name: &'static str) -> CatalogEntry {
    let cmd = curated_command(name);
    let scope = match required_scope_for_action(name) {
        Some(WRITE_SCOPE) => "write",
        Some(READ_SCOPE) => "read",
        _ => "public",
    };
    CatalogEntry {
        name,
        kind: if cmd.is_some() { "curated" } else { "generic" },
        scope,
        destructive: action_is_destructive(name),
        capability: cmd
            .map(|c| format!("{:?}", c.capability))
            .unwrap_or_else(|| "infra".to_string()),
        required_params: required_params_for_action(name)
            .into_iter()
            .filter(|param| *param != "service")
            .collect(),
        allowed_kinds: allowed_kind_names_for_action(name),
        description: cmd
            .map(|c| c.description)
            .unwrap_or_else(|| generic_description(name)),
    }
}

/// Short prose for the fixed infra verbs (curated commands carry their own).
fn generic_description(name: &str) -> &'static str {
    match name {
        "integrations" => "List configured and supported service integrations.",
        "service_status" => "Call the default status endpoint for a service.",
        "api_get" => "Allowlisted GET passthrough against a service's upstream API.",
        "api_post" => "Allowlisted POST passthrough (runs immediately).",
        "api_put" => "Allowlisted PUT passthrough (runs immediately).",
        "api_delete" => "Allowlisted DELETE passthrough. Destructive — refused inside Code Mode.",
        "help" => "Registry-derived action help.",
        _ => "",
    }
}

/// The catalog serialized to a JSON array string for preamble injection.
pub fn catalog_json() -> String {
    serde_json::to_string(&build_catalog()).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
