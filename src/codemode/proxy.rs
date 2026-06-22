//! JS preamble generation.
//!
//! Builds the boilerplate injected before user code: the `callTool` bridge,
//! a `console` shim that captures output, the `__rustarrRun` driver that settles
//! the script into `globalThis.__rustarrResult`, and the auto-generated `tools`
//! namespace — one helper per registry action, the rustarr analogue of lab's
//! `codemode.<upstream>.<tool>` proxies. The catalog is the action registry, so a
//! new curated command appears as `tools.<name>(params)` automatically.

use crate::actions::all_action_names;
use crate::actions::curated_commands;

/// The fixed JS runtime injected before user code: capture-aware `console`, the
/// `callTool` bridge over the native emit, and the `__rustarrRun` driver.
const RUNTIME_JS: &str = r#"
globalThis.__rustarrLogs = [];
const __rustarrFmt = (args) => args.map((a) => {
    if (typeof a === "string") return a;
    try { return JSON.stringify(a); } catch (_) { return String(a); }
}).join(" ");
globalThis.console = {
    log: (...a) => { globalThis.__rustarrLogs.push(__rustarrFmt(a)); },
    info: (...a) => { globalThis.__rustarrLogs.push(__rustarrFmt(a)); },
    warn: (...a) => { globalThis.__rustarrLogs.push("WARN " + __rustarrFmt(a)); },
    error: (...a) => { globalThis.__rustarrLogs.push("ERROR " + __rustarrFmt(a)); },
    debug: (...a) => { globalThis.__rustarrLogs.push(__rustarrFmt(a)); },
};
globalThis.callTool = (id, params = {}) => {
    if (typeof id !== "string" || id.trim() === "") {
        throw new TypeError("callTool(id, params): id must be a non-empty string");
    }
    if (params === null || typeof params !== "object" || Array.isArray(params)) {
        throw new TypeError("callTool(id, params): params must be a JSON object");
    }
    return JSON.parse(__rustarrEmitToolCall(id, JSON.stringify(params)));
};
globalThis.writeArtifact = (path, content, options = {}) => {
    if (typeof path !== "string" || path.trim() === "") {
        throw new TypeError("writeArtifact(path, content): path must be a non-empty string");
    }
    if (typeof content !== "string") {
        throw new TypeError("writeArtifact(path, content): content must be a string");
    }
    if (options === null || typeof options !== "object" || Array.isArray(options)) {
        throw new TypeError("writeArtifact(path, content, options): options must be a JSON object");
    }
    return JSON.parse(__rustarrEmitWriteArtifact(path, content, JSON.stringify(options)));
};
globalThis.input = (typeof globalThis.__rustarrInputJson === "string")
    ? JSON.parse(globalThis.__rustarrInputJson) : null;
globalThis.__rustarrDone = false;
globalThis.__rustarrResult = "null";
globalThis.__rustarrRun = (entry) => {
    Promise.resolve()
        .then(() => (typeof entry === "function" ? entry() : entry))
        .then((value) => {
            let json;
            try { json = JSON.stringify(value === undefined ? null : value); }
            catch (e) { json = JSON.stringify({ __codemode_error: "result not serializable: " + String(e) }); }
            globalThis.__rustarrResult = json === undefined ? "null" : json;
        })
        .catch((err) => {
            const message = (err && err.message) ? err.message : String(err);
            globalThis.__rustarrResult = JSON.stringify({ __codemode_error: message });
        })
        .finally(() => { globalThis.__rustarrDone = true; });
};
"#;

/// Build the complete preamble: the fixed runtime, the generated `tools`
/// namespace (one helper per registry action), and the typed `api.<service>`
/// client (one entry per configured service, sugar over the `api_*` passthrough
/// actions). `callTool` remains available for dynamic action names.
///
/// `service_names` are the configured service names (from
/// `RustarrService::configured_service_names`); pass `&[]` when there are none.
pub fn build_preamble(service_names: &[String]) -> String {
    let mut out = String::with_capacity(RUNTIME_JS.len() + 2048);
    out.push_str(RUNTIME_JS);
    out.push_str("globalThis.tools = {};\n");
    for action in proxy_action_names() {
        // Bracket notation so reserved words (e.g. `delete`) are valid keys, and
        // `params || {}` so a zero-arg helper call still passes a JSON object.
        out.push_str(&format!(
            "globalThis.tools[{action:?}] = (params) => callTool({action:?}, params || {{}});\n"
        ));
    }
    out.push_str(&render_api_namespace(service_names));
    // Discovery: inject the registry-derived ACTION catalog + the per-service TYPE
    // catalog (response-shape TS interfaces), then the pure-JS search/describe
    // helpers. Types are surfaced ON DEMAND — only what the agent describes is
    // returned to it — so the full type surface is never dumped into its context.
    out.push_str("globalThis.__codemodeCatalog = ");
    out.push_str(&super::catalog::catalog_json());
    out.push_str(";\n");
    out.push_str("globalThis.__codemodeTypes = ");
    out.push_str(&super::dts::type_catalog_json());
    out.push_str(";\n");
    out.push_str(DISCOVERY_JS);
    out
}

/// Pure-JS `codemode.search`/`codemode.describe` over the injected action +
/// type catalogs. No host round-trip. `describe` resolves either an action or a
/// `service.TypeName` response type (returning its TS interface).
const DISCOVERY_JS: &str = r#"
globalThis.codemode = globalThis.codemode || {};
globalThis.codemode.search = (query, limit) => {
    const q = String(query == null ? "" : query).toLowerCase().trim();
    const lim = (typeof limit === "number" && limit > 0) ? limit : 20;
    const toks = q.split(/\s+/).filter(Boolean);
    const actionHits = globalThis.__codemodeCatalog.map((e) => {
        const hay = (e.name + " " + (e.description || "") + " " + (e.capability || "")).toLowerCase();
        let score = 0;
        if (e.name.toLowerCase() === q) score += 100;
        else if (e.name.toLowerCase().indexOf(q) !== -1) score += 50;
        for (const tok of toks) { if (hay.indexOf(tok) !== -1) score += 5; }
        return { e: { name: e.name, kind: e.kind, scope: e.scope, destructive: e.destructive, description: e.description }, score: score };
    });
    const typeHits = globalThis.__codemodeTypes.map((t) => {
        const hay = (t.name + " " + t.type_name).toLowerCase();
        let score = 0;
        if (t.name.toLowerCase() === q || t.type_name.toLowerCase() === q) score += 100;
        else if (hay.indexOf(q) !== -1) score += 40;
        for (const tok of toks) { if (hay.indexOf(tok) !== -1) score += 4; }
        return { e: { name: t.name, kind: "type", service: t.service }, score: score };
    });
    const scored = actionHits.concat(typeHits).filter((x) => q === "" || x.score > 0);
    scored.sort((a, b) => b.score - a.score || a.e.name.localeCompare(b.e.name));
    const results = scored.slice(0, lim).map((x) => x.e);
    return { total: results.length, results: results };
};
globalThis.codemode.describe = (name) => {
    // An action?
    const action = globalThis.__codemodeCatalog.find((e) => e.name === name);
    if (action) {
        const sig = action.name + "(" + (action.required_params || []).join(", ") + ")";
        return Object.assign({}, action, { signature: sig });
    }
    // A response type, by "service.TypeName" or an unambiguous bare "TypeName".
    let type = globalThis.__codemodeTypes.find((t) => t.name === name);
    if (!type) {
        const bare = globalThis.__codemodeTypes.filter((t) => t.type_name === name);
        if (bare.length === 1) type = bare[0];
    }
    if (type) return { name: type.name, kind: "type", service: type.service, dts: type.dts };
    return null;
};
globalThis.codemode.snippets = () => callTool("snippet_list", {});
globalThis.codemode.run = (name, input) =>
    callTool("snippet_run", { name: name, input: (input === undefined ? null : input) });
"#;

/// Render the `api.<service>` client: per configured service, `get/post/put/delete`
/// helpers that are thin sugar over the generic `api_*` passthrough actions
/// (`api.sonarr.get("/series")` → `callTool("api_get", {service:"sonarr", path})`).
/// `delete` resolves to the destructive `api_delete`, which `codemode_dispatch`
/// already refuses mid-script — so it throws in-sandbox, by design.
fn render_api_namespace(service_names: &[String]) -> String {
    let mut out = String::from("globalThis.api = {};\n");
    for name in service_names {
        // `{name:?}` emits a quoted, escaped JS string literal — never raw
        // interpolation. `body` is forwarded as-is (undefined drops out of the
        // JSON object, so the server-side body default applies).
        out.push_str(&format!(
            "globalThis.api[{name:?}] = {{\n  \
               get: (path) => callTool(\"api_get\", {{ service: {name:?}, path: path }}),\n  \
               post: (path, body) => callTool(\"api_post\", {{ service: {name:?}, path: path, body: body }}),\n  \
               put: (path, body) => callTool(\"api_put\", {{ service: {name:?}, path: path, body: body }}),\n  \
               delete: (path, body) => callTool(\"api_delete\", {{ service: {name:?}, path: path, body: body }}),\n\
             }};\n"
        ));
    }
    out
}

/// Action names exposed as `tools.<name>` helpers: every registry action except
/// `codemode` itself (no self-recursion), `help` (returns prose), and the
/// `snippet_*` store verbs (reachable only via the explicit `codemode.run`/
/// `codemode.snippets` helpers — never as an incidental in-script side effect
/// that could persist or delete files).
fn proxy_action_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = all_action_names()
        .into_iter()
        .filter(|name| *name != "codemode" && *name != "help" && !name.starts_with("snippet_"))
        .collect();
    // `all_action_names()` already includes curated commands, but assert coverage
    // so a future registry refactor that drops them from that list is caught here.
    for cmd in curated_commands() {
        debug_assert!(
            names.contains(&cmd.name),
            "curated command {} missing from codemode proxy",
            cmd.name
        );
    }
    names.sort_unstable();
    names.dedup();
    names
}

#[cfg(test)]
#[path = "proxy_tests.rs"]
mod tests;
