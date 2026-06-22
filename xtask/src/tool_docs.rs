use anyhow::{Result, bail};
use rustarr::{ACTION_SPECS, Capability, CommandDescriptor, ServiceKind, curated_commands};
use std::fmt::Write as _;
use std::path::Path;

mod endpoints;

use endpoints::{
    ARR_ENDPOINTS, DOWNLOAD_ENDPOINTS, EndpointRow, INDEXER_ENDPOINTS, MEDIA_ENDPOINTS,
    REQUEST_ENDPOINTS, STATS_ENDPOINTS,
};

const OUTPUT: &str = "docs/TOOLS_ACTIONS_ENDPOINTS.md";

pub fn run(args: &[String]) -> Result<()> {
    let check = args.iter().any(|arg| arg == "--check");
    let doc = render();
    let path = Path::new(OUTPUT);
    if check {
        let current = std::fs::read_to_string(path)?;
        if current != doc {
            bail!("{OUTPUT} is stale; run `cargo xtask tool-docs`");
        }
        println!("==> {OUTPUT} is current");
    } else {
        std::fs::write(path, doc)?;
        println!("==> generated {OUTPUT}");
    }
    Ok(())
}

fn render() -> String {
    let mut out = String::new();
    render_header(&mut out);
    render_mcp_tools(&mut out);
    render_schema_metadata(&mut out);
    render_generic_actions(&mut out);
    render_capabilities(&mut out);
    render_generic_only_services(&mut out);
    render_cli_verbs(&mut out);
    out
}

fn render_header(out: &mut String) {
    out.push_str(
        r#"---
title: "Tools, Actions, Params, and Endpoints"
doc_type: "reference"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "runtime"
source_of_truth: false
generated_by: "cargo xtask tool-docs"
last_reviewed: "2026-06-18"
---

# Tools, Actions, Params, and Endpoints

<!-- GENERATED: do not edit by hand. Run `cargo xtask tool-docs`. -->

This reference maps the Rustarr MCP/CLI action surface to the upstream HTTP
endpoints it calls. Action names, params, scopes, and mutability are read from
the Rust action registry. Endpoint mappings are rendered from the structured
generator table in `xtask/src/tool_docs/endpoints.rs`.

"#,
    );
}

fn render_mcp_tools(out: &mut String) {
    out.push_str(
        r#"## MCP Tools

| Tool | Kind | Curated capability | API prefix | Path allowlist |
|---|---|---|---|---|
"#,
    );
    for kind in ServiceKind::ALL {
        let descriptor = kind.descriptor();
        let prefix = if descriptor.api_prefix.is_empty() {
            "(none)"
        } else {
            descriptor.api_prefix
        };
        let allowlist = descriptor.path_allowlist.join(", ");
        let _ = writeln!(
            out,
            "| `{}` | `{}` | {:?} | `{}` | `{}` |",
            kind.as_str(),
            kind.as_str(),
            kind.capability(),
            prefix,
            allowlist,
        );
    }
}

fn render_schema_metadata(out: &mut String) {
    out.push_str(
        r#"
## MCP Schema Metadata

Every service-named MCP tool publishes registry-derived metadata in its
`inputSchema`. Clients that understand schema extensions can use these fields
instead of scraping prose:

| Extension | Source | Purpose |
|---|---|---|
| `x-rustarr-action-metadata` | `ACTION_SPECS` + `curated_commands()` | Per-action scope, params, mutability, confirm requirement, capability, and allowed service kinds. |
| `x-rustarr-service-metadata` | `ServiceKind::descriptor()` | Per-tool kind, capability, auth style, API prefix, resource noun, and path allowlist. |
| `x-rustarr-agent-guidance` | schema generator | Preferred first-pass reads, generic passthrough guidance, write confirmation rules, and response-shaping hints. |
| `properties.*.x-rustarr-actions` | curated command descriptors | Lists which curated actions consume a lifted top-level param. |

"#,
    );
}

fn render_generic_actions(out: &mut String) {
    out.push_str("\n## Generic Actions\n\n");
    out.push_str("| Action | Params | Scope | Mutates | Upstream call |\n");
    out.push_str("|---|---|---|---:|---|\n");
    for spec in ACTION_SPECS {
        let params = generic_params(spec.name);
        let endpoint = generic_endpoint(spec.name);
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} |",
            spec.name,
            params,
            scope(spec.required_scope),
            yes_no(generic_mutates(spec.name)),
            endpoint,
        );
    }
}

fn render_capabilities(out: &mut String) {
    render_capability(
        out,
        "Sonarr And Radarr Actions",
        Capability::ArrManager,
        &["sonarr", "radarr"],
        ARR_ENDPOINTS,
    );
    render_capability(
        out,
        "Prowlarr Actions",
        Capability::Indexer,
        &["prowlarr"],
        INDEXER_ENDPOINTS,
    );
    render_capability(
        out,
        "Overseerr Actions",
        Capability::Requests,
        &["overseerr"],
        REQUEST_ENDPOINTS,
    );
    render_capability(
        out,
        "Tautulli Actions",
        Capability::Stats,
        &["tautulli"],
        STATS_ENDPOINTS,
    );
    render_capability(
        out,
        "SABnzbd And qBittorrent Actions",
        Capability::DownloadClient,
        &["sabnzbd", "qbittorrent"],
        DOWNLOAD_ENDPOINTS,
    );
    render_capability(
        out,
        "Plex And Jellyfin Actions",
        Capability::MediaServer,
        &["plex", "jellyfin"],
        MEDIA_ENDPOINTS,
    );
}

fn render_generic_only_services(out: &mut String) {
    out.push_str(
        r#"## GenericOnly Services

`bazarr` and `tracearr` currently expose only the generic actions as first-class
actions. They are still covered by `api_get`, `api_post`, `api_put`, and
`api_delete`, with path allowlists from `ServiceKind::descriptor()`.

| Service | Useful endpoint families |
|---|---|
| `bazarr` | `/api/system/status`, `/api/system/health`, `/api/system/jobs`, `/api/system/tasks`, `/api/movies`, `/api/series`, `/api/movies/subtitles`, `/api/episodes/subtitles`, `/api/subtitles`, `/api/movies/wanted`, `/api/episodes/wanted`, `/api/movies/history`, `/api/episodes/history`, `/api/movies/blacklist`, `/api/episodes/blacklist`, `/api/providers`, `/api/plex/oauth/pin`, `/api/plex/oauth/logout`, `/api/plex/webhook/list` |
| `tracearr` | `/health`, `/api/v1/public/health`, `/api/v1/public/stats`, `/api/v1/public/stats/today`, `/api/v1/public/activity`, `/api/v1/public/streams`, `/api/v1/public/streams/{id}/terminate`, `/api/v1/public/users`, `/api/v1/public/violations`, `/api/v1/public/history`, `/api/v1/debug/sessions`, `/api/v1/debug/violations`, `/api/v1/debug/rules`, `/api/v1/debug/library`, `/api/v1/debug/users`, `/api/v1/debug/servers`, `/api/v1/debug/reset` |

Live mcporter coverage currently validates Bazarr seeded blacklist deletion via
`api_delete /api/movies/blacklist?all=true` and Tracearr seeded debug-session
deletion via `api_delete /api/v1/debug/sessions`.

"#,
    );
}

fn render_cli_verbs(out: &mut String) {
    out.push_str(
        r#"## CLI Verb Mapping

The CLI uses service-grouped friendly verbs. These map to the MCP action names:

| Capability | CLI verbs |
|---|---|
| ArrManager | `quality-profiles`, `list`, `wanted`, `queue`, `history`, `rootfolders`, `health`, `set-quality`, `search`, `refresh`, `monitor`, `unmonitor`, `add`, `delete` |
| Indexer | `indexers`, `search`, `stats`, `test` |
| Requests | `requests`, `search`, `request`, `approve`, `decline` |
| Stats | `activity`, `history`, `users`, `libraries`, `refresh-libraries`, `refresh-users`, `delete-image-cache` |
| DownloadClient | `queue`, `add`, `pause`, `resume`, `remove` |
| MediaServer | `sessions`, `libraries`, `search`, `scan` |
"#,
    );
}

fn render_capability(
    out: &mut String,
    title: &str,
    capability: Capability,
    tools: &[&str],
    endpoints: &[EndpointRow],
) {
    let _ = writeln!(out, "\n## {title}\n");
    let _ = writeln!(out, "Tools: {}.\n", tools.join(", "));
    out.push_str("| Action | Params | Scope | Mutates | Upstream call | Notes |\n");
    out.push_str("|---|---|---|---:|---|---|\n");
    for command in curated_commands()
        .iter()
        .filter(|command| command.capability == capability)
    {
        let endpoint = endpoints
            .iter()
            .find(|row| row.action == command.name)
            .copied()
            .unwrap_or(EndpointRow {
                action: command.name,
                tools: "",
                endpoint: "MISSING ENDPOINT MAPPING",
                notes: "",
            });
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} |",
            command.name,
            params(command),
            command.required_scope,
            yes_no(command.mutates),
            endpoint_for_tools(endpoint),
            endpoint.notes,
        );
    }
}

fn endpoint_for_tools(row: EndpointRow) -> String {
    if row.tools.is_empty() {
        row.endpoint.to_string()
    } else {
        format!("{}: {}", row.tools, row.endpoint)
    }
}

fn params(command: &CommandDescriptor) -> String {
    let mut parts: Vec<String> = Vec::new();
    for param in command
        .required_params
        .iter()
        .copied()
        .filter(|param| *param != "service")
    {
        parts.push(format!("`{param}`"));
    }
    for param in command.optional_params {
        parts.push(format!("optional `{param}`"));
    }
    if parts.is_empty() {
        "none".into()
    } else {
        parts.join(", ")
    }
}

fn generic_params(action: &str) -> &'static str {
    match action {
        "integrations" | "help" => "none",
        "service_status" => "none; service is implied by MCP tool or CLI service token",
        "api_get" => "`path`",
        "api_post" | "api_put" => "`path`, optional `body`, `confirm`",
        "api_delete" => "`path`, optional `body`, `confirm`",
        _ => "",
    }
}

fn generic_endpoint(action: &str) -> &'static str {
    match action {
        "integrations" => "No upstream call; returns configured/supported service catalog.",
        "service_status" => {
            "GET the kind default status path, e.g. Sonarr/Radarr `/api/v3/system/status`, Prowlarr `/api/v1/system/status`, Overseerr `/api/v1/status`, Tautulli `/api/v2?cmd=get_server_info`, Bazarr `/api/system/status`, Tracearr `/health`, SABnzbd `/api?mode=version&output=json`, qBittorrent `/api/v2/app/version`, Plex `/identity`, Jellyfin `/System/Info/Public`."
        }
        "api_get" => "`GET {path}`.",
        "api_post" => "`POST {path}` with JSON body.",
        "api_put" => "`PUT {path}` with JSON body.",
        "api_delete" => "`DELETE {path}` with optional JSON body.",
        "help" => "No upstream call; returns registry-derived action help.",
        _ => "",
    }
}

fn generic_mutates(action: &str) -> bool {
    matches!(action, "api_post" | "api_put" | "api_delete")
}

fn scope(scope: Option<&'static str>) -> &'static str {
    scope.unwrap_or("public")
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
