use anyhow::{Result, bail};
use rustarr::{
    ACTION_SPECS, Capability, CommandDescriptor, ServiceKind, capability_verb_tables,
    curated_commands,
};
use std::fmt::Write as _;
use std::path::Path;

mod endpoints;

use endpoints::{DOWNLOAD_ENDPOINTS, EndpointRow, STATS_ENDPOINTS};

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
    render_service_kinds(&mut out);
    render_schema_metadata(&mut out);
    render_generic_actions(&mut out);
    render_generated_operations(&mut out);
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
last_reviewed: "2026-06-23"
---

# Tools, Actions, Params, and Endpoints

<!-- GENERATED: do not edit by hand. Run `cargo xtask tool-docs`. -->

The MCP surface is a single tool, `yarr`, which runs a Code Mode script (the
`codemode` action). Inside a script the fleet is reached through per-service
callables (`sonarr.get_series()`, `qbittorrent.download_queue()`), the
`api.<service>` raw passthrough, and `callTool`. This reference maps the
underlying action surface to the upstream HTTP endpoints it calls. Action names,
params, scopes, and mutability are read from the Rust action registry; curated
endpoint mappings are rendered from `xtask/src/tool_docs/endpoints.rs`.

"#,
    );
}

fn render_service_kinds(out: &mut String) {
    out.push_str(
        r#"## Service Kinds

There is one published MCP tool (`yarr`). The table below lists the service
*kinds* a configured service can take — each kind's capability, upstream API
prefix, and path allowlist (from `ServiceKind::descriptor()`). The 6 spec-backed
kinds (sonarr/radarr/prowlarr/overseerr/jellyfin/plex) expose their full upstream
API as generated operations; the rest keep curated commands or generic
passthrough only.

| Kind | Curated capability | API prefix | Path allowlist |
|---|---|---|---|
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
            "| `{}` | {:?} | `{}` | `{}` |",
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
## Action Schema Metadata

Each service kind has a registry-derived action schema (it backs the per-service
callables and the `callTool` dispatch path; it is not published as a separate MCP
tool). Clients that understand schema extensions can read these fields instead of
scraping prose:

| Extension | Source | Purpose |
|---|---|---|
| `x-rustarr-action-metadata` | `ACTION_SPECS` + `curated_commands()` | Per-action scope, params, mutability, confirm requirement, capability, and allowed service kinds. |
| `x-rustarr-service-metadata` | `ServiceKind::descriptor()` | Per-kind capability, auth style, API prefix, resource noun, and path allowlist. |
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

fn render_generated_operations(out: &mut String) {
    out.push_str(
        r#"
## Generated Operations (spec-backed services)

`sonarr`, `radarr`, `prowlarr`, `overseerr`, `jellyfin`, and `plex` are generated
from their vendored OpenAPI specs (`cargo xtask gen-openapi` →
`src/openapi/generated/`). Every spec operation becomes a per-service callable
(`sonarr.get_series()`, `radarr.post_movie({ body })`) dispatched via the `op`
action; there are no hand-written curated commands for these kinds. Discover them
with `codemode.search(query)` and inspect signatures / response types with
`codemode.describe(path)`. DELETE operations are refused mid-script (run them via
the CLI `op` with `--confirm`, or set `RUSTARR_ALLOW_DESTRUCTIVE`).

"#,
    );
}

fn render_capabilities(out: &mut String) {
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
}

fn render_generic_only_services(out: &mut String) {
    out.push_str(
        r#"
## GenericOnly Services

`bazarr` and `tracearr` expose only the generic actions as first-class actions.
They are covered by `api_get`, `api_post`, `api_put`, and `api_delete`, with path
allowlists from `ServiceKind::descriptor()`.

| Service | Useful endpoint families |
|---|---|
| `bazarr` | `/api/system/status`, `/api/system/health`, `/api/system/jobs`, `/api/system/tasks`, `/api/movies`, `/api/series`, `/api/movies/subtitles`, `/api/episodes/subtitles`, `/api/subtitles`, `/api/movies/wanted`, `/api/episodes/wanted`, `/api/movies/history`, `/api/episodes/history`, `/api/movies/blacklist`, `/api/episodes/blacklist`, `/api/providers`, `/api/plex/oauth/pin`, `/api/plex/oauth/logout`, `/api/plex/webhook/list` |
| `tracearr` | `/health`, `/api/v1/public/health`, `/api/v1/public/stats`, `/api/v1/public/stats/today`, `/api/v1/public/activity`, `/api/v1/public/streams`, `/api/v1/public/streams/{id}/terminate`, `/api/v1/public/users`, `/api/v1/public/violations`, `/api/v1/public/history`, `/api/v1/debug/sessions`, `/api/v1/debug/violations`, `/api/v1/debug/rules`, `/api/v1/debug/library`, `/api/v1/debug/users`, `/api/v1/debug/servers`, `/api/v1/debug/reset` |

These are exercised through the generic passthrough (`rustarr <service> get|post|put|delete`)
and the live `cli` suite; the spec-backed services are covered exhaustively by the
`contract` suite (`cargo xtask live --suite contract`).
"#,
    );
}

fn render_cli_verbs(out: &mut String) {
    out.push_str(
        r#"
## CLI Verb Mapping

The CLI is service-grouped (`rustarr <service> <verb>`). Only the curated
capabilities below have friendly verbs; the spec-backed services use
`rustarr <service> op <operation>` (generated operations) or the generic
`get/post/put/delete` passthrough. Verb tables are read from the CLI registry.

| Capability | CLI verbs |
|---|---|
"#,
    );
    for (capability, verbs) in capability_verb_tables() {
        let list = verbs
            .iter()
            .map(|(verb, _action)| format!("`{verb}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "| {capability:?} | {list} |");
    }
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
        "help" => "none",
        "service_status" => "none; service is implied by MCP tool or CLI service token",
        "api_get" => "`path`",
        "api_post" | "api_put" => "`path`, optional `body`",
        "api_delete" => "`path`, optional `body`, `confirm`",
        "codemode" => "`code` (a JavaScript async arrow function)",
        "op" => "`op` (operation name), optional `args`, `confirm` (DELETE ops)",
        "snippet_save" => "`name`, `code`, optional `description`",
        "snippet_run" => "`name`, optional `input`",
        "snippet_delete" => "`name`",
        "snippet_list" => "none",
        _ => "",
    }
}

fn generic_endpoint(action: &str) -> &'static str {
    match action {
        "service_status" => {
            "GET the kind default status path, e.g. Sonarr/Radarr `/api/v3/system/status`, Prowlarr `/api/v1/system/status`, Overseerr `/api/v1/status`, Tautulli `/api/v2?cmd=get_server_info`, Bazarr `/api/system/status`, Tracearr `/health`, SABnzbd `/api?mode=version&output=json`, qBittorrent `/api/v2/app/version`, Plex `/identity`, Jellyfin `/System/Info/Public`."
        }
        "api_get" => "`GET {path}`.",
        "api_post" => "`POST {path}` with JSON body. Runs immediately.",
        "api_put" => "`PUT {path}` with JSON body. Runs immediately.",
        "api_delete" => {
            "`DELETE {path}` with optional JSON body. Destructive: gated by `--confirm`."
        }
        "help" => "No upstream call; returns registry-derived action help.",
        "codemode" => {
            "No direct upstream call; runs a Code Mode script that dispatches other actions."
        }
        "op" => "Dispatches a generated OpenAPI operation for a spec-backed service.",
        "snippet_list" | "snippet_save" | "snippet_run" | "snippet_delete" => {
            "No upstream call; manages the Code Mode snippet store under the data dir."
        }
        _ => "",
    }
}

fn generic_mutates(action: &str) -> bool {
    matches!(
        action,
        "api_post" | "api_put" | "api_delete" | "op" | "snippet_save" | "snippet_delete"
    )
}

fn scope(scope: Option<&'static str>) -> &'static str {
    scope.unwrap_or("public")
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
