//! Generated OpenAPI operation surface (runtime side).
//!
//! For the 6 spec-backed services (Sonarr, Radarr, Prowlarr, Overseerr, Jellyfin,
//! Plex) the entire upstream API — every operation and every component type — is
//! **generated from the vendored OpenAPI specs** under `specs/` by
//! `cargo xtask gen-openapi`, not hand-written. This module holds the runtime
//! shapes the generated tables fill in ([`OperationSpec`], [`TypeDef`]), the
//! per-kind registry, and the generic executor that turns one
//! `(service, op, args)` call into an upstream request.
//!
//! Discovery (`codemode.search`/`describe`) and the per-service callable namespace
//! are driven entirely off these tables, so adding/refreshing a service is a
//! regeneration step — there is no hand-rolled curated command or model for these
//! kinds.

use serde_json::Value;

use crate::config::ServiceKind;

// Generated tables (one module per spec-backed service). Each provides
// `pub static OPERATIONS: &[OperationSpec]` and `pub static TYPES: &[TypeDef]`.
pub mod generated;

#[cfg(test)]
#[path = "openapi_tests.rs"]
mod tests;

/// One generated upstream operation — the data behind a single Code Mode callable
/// (`<service>.<name>(args)`). Path/method are spec-derived (trusted); only the
/// arg *values* are user input and are encoded at execution time.
#[derive(Debug, Clone, Copy)]
pub struct OperationSpec {
    /// Code Mode method name, e.g. `list_series`. Globally unique per service.
    pub name: &'static str,
    /// HTTP method, uppercase (`GET`/`POST`/`PUT`/`DELETE`/`PATCH`).
    pub method: &'static str,
    /// Path template with `{param}` placeholders, e.g. `/api/v3/series/{id}`.
    pub path: &'static str,
    /// Names of the `{param}` path placeholders (required).
    pub path_params: &'static [&'static str],
    /// Query parameter names this operation accepts (all optional).
    pub query_params: &'static [&'static str],
    /// Whether the operation takes a request body (passed as `args.body`).
    pub has_body: bool,
    /// Request-body component type name, if any (a key into [`TypeDef`]).
    pub request_type: Option<&'static str>,
    /// 2xx response component type name, if any (a key into [`TypeDef`]).
    pub response_type: Option<&'static str>,
    /// OpenAPI tag (used to group callables in discovery).
    pub tag: &'static str,
    /// One-line summary from the spec (operation summary/description).
    pub summary: &'static str,
}

/// One generated component type, rendered as a TypeScript interface so
/// `codemode.describe("<service>.<TypeName>")` can surface it on demand.
#[derive(Debug, Clone, Copy)]
pub struct TypeDef {
    /// Component name, e.g. `SeriesResource`.
    pub name: &'static str,
    /// The generated TypeScript interface declaration.
    pub ts: &'static str,
}

/// Whether a kind's API is generated from an OpenAPI spec (vs the doc-based,
/// hand-modeled kinds). Drives whether the per-service callable surface comes from
/// generated operations or the legacy curated commands.
pub fn is_generated(kind: ServiceKind) -> bool {
    !operations_for_kind(kind).is_empty()
}

/// The generated operations for a kind (empty for the doc-based kinds).
pub fn operations_for_kind(kind: ServiceKind) -> &'static [OperationSpec] {
    match kind {
        ServiceKind::Sonarr => generated::sonarr::OPERATIONS,
        ServiceKind::Radarr => generated::radarr::OPERATIONS,
        ServiceKind::Prowlarr => generated::prowlarr::OPERATIONS,
        ServiceKind::Overseerr => generated::overseerr::OPERATIONS,
        ServiceKind::Jellyfin => generated::jellyfin::OPERATIONS,
        ServiceKind::Plex => generated::plex::OPERATIONS,
        _ => &[],
    }
}

/// The generated component types for a kind (empty for the doc-based kinds).
pub fn types_for_kind(kind: ServiceKind) -> &'static [TypeDef] {
    match kind {
        ServiceKind::Sonarr => generated::sonarr::TYPES,
        ServiceKind::Radarr => generated::radarr::TYPES,
        ServiceKind::Prowlarr => generated::prowlarr::TYPES,
        ServiceKind::Overseerr => generated::overseerr::TYPES,
        ServiceKind::Jellyfin => generated::jellyfin::TYPES,
        ServiceKind::Plex => generated::plex::TYPES,
        _ => &[],
    }
}

/// Look up one generated operation by kind + name.
pub fn find_operation(kind: ServiceKind, name: &str) -> Option<&'static OperationSpec> {
    operations_for_kind(kind).iter().find(|op| op.name == name)
}

/// Render a value as a string for use in a path segment or query param. Strings
/// pass through; numbers/bools stringify; everything else (objects/arrays/null)
/// yields `None` so the caller can reject it rather than send `[object Object]`.
pub fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}
