//! USAGE text, generated from the action registry + capability map.
//!
//! Rather than hand-maintaining a giant static string, the per-command lines are
//! derived from [`crate::actions::ACTION_SPECS`], [`crate::actions::CURATED_COMMANDS`],
//! and [`crate::config::ServiceKind`] so the help can never drift from the
//! grammar the router accepts. The result is cached with `OnceLock`.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::OnceLock;

use super::router::INFRA_VERBS;
use crate::actions::CURATED_COMMANDS;
use crate::capability::Capability;
use crate::config::ServiceKind;

/// Render (and cache) the full USAGE string.
pub fn usage() -> &'static str {
    static USAGE: OnceLock<String> = OnceLock::new();
    USAGE.get_or_init(build_usage)
}

fn build_usage() -> String {
    let mut out = String::new();
    out.push_str("Usage:\n");

    // Run modes (handled in main.rs, not the router).
    out.push_str("  rustarr [serve]          Start MCP HTTP server (default)\n");
    out.push_str("  rustarr mcp              Start MCP stdio transport\n\n");

    // Infra, service-less commands.
    out.push_str("Infra commands (service-less):\n");
    out.push_str("  rustarr integrations              List supported and configured services\n");
    out.push_str("  rustarr help                      Show JSON action reference\n");
    out.push_str("  rustarr doctor [--json]           Run environment pre-flight checks\n");
    out.push_str(
        "  rustarr watch [--url URL] [--interval N]  Poll server health and emit on state change\n",
    );
    out.push_str(
        "  rustarr setup check               Check plugin setup without mutating appdata\n",
    );
    out.push_str("  rustarr setup repair              Create missing appdata/env setup files\n");
    out.push_str("  rustarr setup plugin-hook [--no-repair]  Plugin hook JSON contract\n\n");

    // Service-grouped commands. Generic verbs apply to every service.
    out.push_str("Service commands (rustarr <service> <command>):\n");
    out.push_str("  rustarr <service> status                       Show upstream service status\n");
    out.push_str("  rustarr <service> get --path PATH              Passthrough GET\n");
    out.push_str(
        "  rustarr <service> post --path PATH [--body JSON] --confirm    Passthrough POST\n",
    );
    out.push_str(
        "  rustarr <service> put --path PATH [--body JSON] --confirm     Passthrough PUT\n",
    );
    out.push_str(
        "  rustarr <service> delete --path PATH [--body JSON] --confirm  Passthrough DELETE\n",
    );

    append_curated_commands(&mut out);

    // Services + infra verb inventory.
    let services = ServiceKind::ALL
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let _ = write!(out, "\nServices:\n  {services}\n");
    let _ = write!(out, "\nInfra verbs:\n  {}\n", INFRA_VERBS.join(", "));

    out.push_str("\n  rustarr --help                    Show this help\n");
    out.push_str("  rustarr --version                 Show version\n");

    out.push_str(
        "\nEnvironment:\n\
         \x20 RUSTARR_SERVICES         Comma-separated configured service names\n\
         \x20 RUSTARR_<NAME>_URL       Upstream service URL\n\
         \x20 RUSTARR_<NAME>_API_KEY   Upstream service API key\n\
         \x20 RUSTARR_MCP_HOST         Bind host (default 127.0.0.1)\n\
         \x20 RUSTARR_MCP_PORT         Bind port (default 40070)\n\
         \x20 RUSTARR_MCP_NO_AUTH      Disable auth (loopback only)\n\
         \x20 RUSTARR_MCP_TOKEN        Static bearer token\n\
         \x20 RUST_LOG                 Log filter (e.g. info,rmcp=warn)",
    );

    out
}

/// Append a per-capability section listing curated commands (empty until later
/// beads populate [`CURATED_COMMANDS`]).
fn append_curated_commands(out: &mut String) {
    if CURATED_COMMANDS.is_empty() {
        return;
    }
    out.push_str("\n\nCurated commands by capability:\n");
    // Stable ordering: iterate capabilities, then commands within each.
    let caps: BTreeSet<&'static str> = CURATED_COMMANDS
        .iter()
        .map(|c| capability_label(c.capability))
        .collect();
    for cap in caps {
        let _ = writeln!(out, "  [{cap}]");
        for cmd in CURATED_COMMANDS
            .iter()
            .filter(|c| capability_label(c.capability) == cap)
        {
            let services = services_for_capability(cmd.capability);
            let _ = writeln!(
                out,
                "    rustarr <{services}> {:<20} {}",
                cmd.name, cmd.description
            );
        }
    }
}

fn capability_label(cap: Capability) -> &'static str {
    match cap {
        Capability::ArrManager => "arr",
        Capability::Indexer => "indexer",
        Capability::DownloadClient => "download",
        Capability::MediaServer => "media",
        Capability::Requests => "requests",
        Capability::Stats => "stats",
        Capability::GenericOnly => "generic",
    }
}

/// Slash-joined list of service names sharing a capability — used as the
/// `<service>` placeholder in curated-command usage lines.
fn services_for_capability(cap: Capability) -> String {
    ServiceKind::ALL
        .iter()
        .filter(|k| k.capability() == cap)
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
