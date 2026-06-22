//! Agent-facing TypeScript surface for Code Mode.
//!
//! The JS preamble (callTool, `api.<service>`, `codemode.*`, writeArtifact) runs
//! INSIDE the sandbox and never returns to the calling agent — so it cannot teach
//! the agent the API or the response shapes. This module generates a `.d.ts` that
//! IS reachable by the agent (exposed as the `rustarr://schema/codemode` MCP
//! resource and pointed at from the `codemode` tool description): a hand-written
//! header declaring the in-sandbox API, plus per-service `declare namespace`
//! blocks of response interfaces generated from the `schemars::JsonSchema` derives
//! on the typed contracts in [`crate::models`].
//!
//! This is the home for the typed contracts on the agent side — the whole reason
//! the models derive `JsonSchema`.

use std::collections::BTreeMap;

use serde_json::Value;

/// The hand-written declaration of the in-sandbox Code Mode API.
const API_SURFACE: &str = r#"// ── Code Mode API (available inside the `code` you pass to action=codemode) ──
// Your code is `async () => { ... }`; the sandbox awaits its return value and
// returns { result, calls, logs, artifacts, artifactsRunId? } to you. Only the
// return value + these envelopes come back — nothing else leaves the sandbox.

/** Raw escape hatch: dispatch any action by name. Throws on destructive deletes. */
declare function callTool(action: string, params?: Record<string, unknown>): any;

/** One helper per registry action, e.g. tools.list({ service: "sonarr" }). */
declare const tools: Record<string, (params?: Record<string, unknown>) => any>;

/** Typed-ish client over the raw upstream API, one entry per configured service.
 *  `get`/`post`/`put` run immediately; `delete` is the destructive api_delete and
 *  is refused mid-script (it throws). Returns the upstream JSON (see the per-service
 *  namespaces below for the response shapes). */
declare const api: Record<string, {
    get(path: string): any;
    post(path: string, body?: any): any;
    put(path: string, body?: any): any;
    delete(path: string, body?: any): never;
}>;

interface CatalogEntry {
    name: string;
    kind: "generic" | "curated";
    scope: "read" | "write" | "public";
    destructive: boolean;
    capability: string;
    required_params: string[];
    allowed_kinds: string[];
    description: string;
    signature?: string;
}

interface CodeModeResult {
    result: any;
    calls: { action: string; ok: boolean; error?: string }[];
    logs: string[];
    artifacts: { path: string; ok: boolean; error?: string }[];
    artifactsRunId?: string;
}

declare const codemode: {
    /** Fuzzy-search the action catalog (no host round-trip). */
    search(query: string, limit?: number): { total: number; results: CatalogEntry[] };
    /** Describe one action by exact name, or null. */
    describe(name: string): (CatalogEntry & { signature: string }) | null;
    /** Run a saved snippet (one level deep), binding `input`. */
    run(name: string, input?: any): CodeModeResult;
    /** List saved snippets. */
    snippets(): { snippets: { name: string; description?: string; bytes: number }[] };
};

/** Write a file under this run's sandboxed artifacts dir (relative paths only). */
declare function writeArtifact(
    path: string,
    content: string,
    options?: { contentType?: string },
): { path: string; bytes: number; contentType: string };

/** Bound to the JSON passed to snippet_run / codemode.run; null otherwise. */
declare const input: any;
"#;

/// Build the complete Code Mode `.d.ts`: the API surface plus per-service response
/// types generated from the typed models' `JsonSchema`.
pub fn codemode_dts() -> String {
    let mut out = String::with_capacity(API_SURFACE.len() + 16 * 1024);
    out.push_str(API_SURFACE);
    out.push_str("\n// ── Upstream response shapes (what api.<service>.get(...) returns) ──\n");
    out.push_str(&service_namespaces());
    out
}

/// Per-service `declare namespace` blocks, generated from `schema_for!` on the
/// headline response types (each pulls in its nested types via `$defs`).
fn service_namespaces() -> String {
    // `schema_for!` is a macro over a concrete type, so the headline types are
    // listed explicitly; each root drags in all nested types via `$defs`.
    macro_rules! ty {
        ($name:literal, $t:ty) => {
            (
                $name,
                serde_json::to_value(schemars::schema_for!($t)).unwrap(),
            )
        };
    }
    use crate::models;

    let groups: Vec<(&str, Vec<(&str, Value)>)> = vec![
        (
            "sonarr",
            vec![
                ty!("SeriesResource", models::sonarr::SeriesResource),
                ty!(
                    "QualityProfileResource",
                    models::sonarr::QualityProfileResource
                ),
                ty!("QueueResource", models::sonarr::QueueResource),
                ty!("SystemResource", models::sonarr::SystemResource),
                ty!("RootFolderResource", models::sonarr::RootFolderResource),
            ],
        ),
        (
            "radarr",
            vec![
                ty!("MovieResource", models::radarr::MovieResource),
                ty!(
                    "QualityProfileResource",
                    models::radarr::QualityProfileResource
                ),
                ty!("QueueResource", models::radarr::QueueResource),
                ty!("SystemResource", models::radarr::SystemResource),
            ],
        ),
        (
            "prowlarr",
            vec![
                ty!("IndexerResource", models::prowlarr::IndexerResource),
                ty!(
                    "IndexerStatsResource",
                    models::prowlarr::IndexerStatsResource
                ),
                ty!("SystemResource", models::prowlarr::SystemResource),
            ],
        ),
        (
            "overseerr",
            vec![
                ty!("MediaRequestPage", models::overseerr::MediaRequestPage),
                ty!("MovieResult", models::overseerr::MovieResult),
                ty!("TvResult", models::overseerr::TvResult),
                ty!("Status", models::overseerr::Status),
            ],
        ),
        (
            "jellyfin",
            vec![
                ty!(
                    "BaseItemDtoQueryResult",
                    models::jellyfin::BaseItemDtoQueryResult
                ),
                ty!("SessionInfoDto", models::jellyfin::SessionInfoDto),
                ty!("VirtualFolderInfo", models::jellyfin::VirtualFolderInfo),
                ty!("SystemInfo", models::jellyfin::SystemInfo),
            ],
        ),
        (
            "plex",
            vec![ty!("MediaContainer", models::plex::MediaContainer)],
        ),
        (
            "tautulli",
            vec![
                ty!(
                    "ActivityEnvelope",
                    models::tautulli::TautulliEnvelope<models::tautulli::GetActivityData>
                ),
                ty!("GetHistoryData", models::tautulli::GetHistoryData),
            ],
        ),
        (
            "sabnzbd",
            vec![
                ty!("QueueResponse", models::sabnzbd::QueueResponse),
                ty!("HistoryResponse", models::sabnzbd::HistoryResponse),
                ty!("VersionResponse", models::sabnzbd::VersionResponse),
            ],
        ),
        (
            "qbittorrent",
            vec![
                ty!("TorrentInfo", models::qbittorrent::TorrentInfo),
                ty!("TransferInfo", models::qbittorrent::TransferInfo),
                ty!("BuildInfo", models::qbittorrent::BuildInfo),
            ],
        ),
        (
            "bazarr",
            vec![ty!("SystemStatus", models::bazarr::SystemStatus)],
        ),
        (
            "tracearr",
            vec![ty!("ServerInfo", models::tracearr::ServerInfo)],
        ),
    ];

    groups
        .iter()
        .map(|(service, roots)| namespace_dts(service, roots))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render one `declare namespace <service> { … }` from its root schemas. Each
/// root's top-level object + every `$defs` entry becomes an interface (deduped by
/// name within the namespace).
fn namespace_dts(service: &str, roots: &[(&str, Value)]) -> String {
    let mut decls: BTreeMap<String, String> = BTreeMap::new();
    for (root_name, schema) in roots {
        decls
            .entry((*root_name).to_string())
            .or_insert_with(|| declaration(root_name, schema));
        if let Some(defs) = schema.get("$defs").and_then(Value::as_object) {
            for (def_name, def_schema) in defs {
                decls
                    .entry(def_name.clone())
                    .or_insert_with(|| declaration(def_name, def_schema));
            }
        }
    }
    let body = decls
        .values()
        .map(|d| indent(d))
        .collect::<Vec<_>>()
        .join("\n");
    format!("declare namespace {service} {{\n{body}\n}}\n")
}

/// A single TS declaration for one schema node: an `interface` for an object, a
/// string-union `type` for an enum, else a `Record` alias.
fn declaration(name: &str, schema: &Value) -> String {
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        let union = values
            .iter()
            .filter_map(Value::as_str)
            .map(|v| format!("\"{v}\""))
            .collect::<Vec<_>>()
            .join(" | ");
        let union = if union.is_empty() {
            "string".into()
        } else {
            union
        };
        return format!("export type {name} = {union};");
    }
    let Some(props) = schema.get("properties").and_then(Value::as_object) else {
        // Object with no declared properties (free-form / untyped).
        return format!("export type {name} = Record<string, unknown>;");
    };
    let required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut lines = format!("export interface {name} {{\n");
    for (field, prop) in props {
        let opt = if required.contains(&field.as_str()) {
            ""
        } else {
            "?"
        };
        lines.push_str(&format!("  {field}{opt}: {};\n", ts_type(prop)));
    }
    lines.push('}');
    lines
}

/// Map a JSON Schema property node to a TypeScript type expression.
fn ts_type(prop: &Value) -> String {
    if let Some(reference) = prop.get("$ref").and_then(Value::as_str) {
        return ref_name(reference);
    }
    if let Some(branches) = prop.get("anyOf").and_then(Value::as_array) {
        let parts: Vec<String> = branches
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) != Some("null"))
            .map(ts_type)
            .collect();
        return if parts.is_empty() {
            "null".into()
        } else {
            parts.join(" | ")
        };
    }
    if let Some(values) = prop.get("enum").and_then(Value::as_array) {
        let union = values
            .iter()
            .filter_map(Value::as_str)
            .map(|v| format!("\"{v}\""))
            .collect::<Vec<_>>()
            .join(" | ");
        if !union.is_empty() {
            return union;
        }
    }
    match prop.get("type") {
        Some(Value::String(t)) if t == "array" => array_type(prop),
        Some(Value::String(t)) => scalar(t),
        Some(Value::Array(types)) => {
            // e.g. ["integer","null"] — take the first non-null scalar.
            types
                .iter()
                .filter_map(Value::as_str)
                .find(|t| *t != "null")
                .map(scalar)
                .unwrap_or_else(|| "unknown".into())
        }
        _ => "unknown".into(),
    }
}

fn array_type(prop: &Value) -> String {
    match prop.get("items") {
        Some(Value::Bool(_)) | None => "any[]".into(),
        Some(items) => format!("{}[]", ts_type(items)),
    }
}

fn scalar(t: &str) -> String {
    match t {
        "string" => "string",
        "integer" | "number" => "number",
        "boolean" => "boolean",
        "object" => "Record<string, unknown>",
        "null" => "null",
        _ => "unknown",
    }
    .to_string()
}

fn ref_name(reference: &str) -> String {
    reference
        .rsplit('/')
        .next()
        .unwrap_or(reference)
        .to_string()
}

fn indent(block: &str) -> String {
    block
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[path = "dts_tests.rs"]
mod tests;
