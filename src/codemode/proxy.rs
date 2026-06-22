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

/// Build the complete preamble: the fixed runtime plus the generated `tools`
/// namespace derived from the action registry. `callTool` remains available for
/// dynamic action names; `tools.<action>(params)` is the discoverable surface.
pub fn build_preamble() -> String {
    let mut out = String::with_capacity(RUNTIME_JS.len() + 1024);
    out.push_str(RUNTIME_JS);
    out.push_str("globalThis.tools = {};\n");
    for action in proxy_action_names() {
        // Bracket notation so reserved words (e.g. `delete`) are valid keys, and
        // `params || {}` so a zero-arg helper call still passes a JSON object.
        out.push_str(&format!(
            "globalThis.tools[{action:?}] = (params) => callTool({action:?}, params || {{}});\n"
        ));
    }
    out
}

/// Action names exposed as `tools.<name>` helpers: every registry action except
/// `codemode` itself (no self-recursion) and `help` (returns prose, not data).
fn proxy_action_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = all_action_names()
        .into_iter()
        .filter(|name| *name != "codemode" && *name != "help")
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
