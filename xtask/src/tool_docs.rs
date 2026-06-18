use anyhow::{Result, bail};
use rustarr::actions::{ACTION_SPECS, CommandDescriptor, curated_commands};
use rustarr::capability::Capability;
use rustarr::config::ServiceKind;
use std::fmt::Write as _;
use std::path::Path;

const OUTPUT: &str = "docs/TOOLS_ACTIONS_ENDPOINTS.md";

#[derive(Debug, Clone, Copy)]
struct EndpointRow {
    action: &'static str,
    tools: &'static str,
    endpoint: &'static str,
    notes: &'static str,
}

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
generator table in `xtask/src/tool_docs.rs`.

## MCP Tools

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

    render_capability(
        &mut out,
        "Sonarr And Radarr Actions",
        Capability::ArrManager,
        &["sonarr", "radarr"],
        &ARR_ENDPOINTS,
    );
    render_capability(
        &mut out,
        "Prowlarr Actions",
        Capability::Indexer,
        &["prowlarr"],
        &INDEXER_ENDPOINTS,
    );
    render_capability(
        &mut out,
        "Overseerr Actions",
        Capability::Requests,
        &["overseerr"],
        &REQUEST_ENDPOINTS,
    );
    render_capability(
        &mut out,
        "Tautulli Actions",
        Capability::Stats,
        &["tautulli"],
        &STATS_ENDPOINTS,
    );
    render_capability(
        &mut out,
        "SABnzbd And qBittorrent Actions",
        Capability::DownloadClient,
        &["sabnzbd", "qbittorrent"],
        &DOWNLOAD_ENDPOINTS,
    );
    render_capability(
        &mut out,
        "Plex And Jellyfin Actions",
        Capability::MediaServer,
        &["plex", "jellyfin"],
        &MEDIA_ENDPOINTS,
    );

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

## CLI Verb Mapping

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

    out
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

const ARR_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "quality_profiles",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/qualityprofile`",
        notes: "",
    },
    EndpointRow {
        action: "list",
        tools: "sonarr",
        endpoint: "`GET /api/v3/series`",
        notes: "Radarr uses `GET /api/v3/movie`.",
    },
    EndpointRow {
        action: "wanted",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/wanted/missing?pageSize=50`",
        notes: "",
    },
    EndpointRow {
        action: "queue",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/queue?pageSize=50`",
        notes: "",
    },
    EndpointRow {
        action: "history",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/history?pageSize=50`",
        notes: "",
    },
    EndpointRow {
        action: "rootfolders",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/rootfolder`",
        notes: "",
    },
    EndpointRow {
        action: "health",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/health`",
        notes: "",
    },
    EndpointRow {
        action: "set_quality",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/qualityprofile`, `GET /api/v3/{series|movie}`, then `PUT /api/v3/{series|movie}/editor`",
        notes: "No write without `confirm=true`.",
    },
    EndpointRow {
        action: "search",
        tools: "sonarr/radarr",
        endpoint: "`POST /api/v3/command`",
        notes: "Radarr can batch ids; Sonarr fans out one command per id.",
    },
    EndpointRow {
        action: "refresh",
        tools: "sonarr/radarr",
        endpoint: "`POST /api/v3/command`",
        notes: "Radarr can batch ids; Sonarr fans out one command per id.",
    },
    EndpointRow {
        action: "monitor",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/{series|movie}`, then `PUT /api/v3/{series|movie}/editor`",
        notes: "Sets `monitored=true`.",
    },
    EndpointRow {
        action: "unmonitor",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/{series|movie}`, then `PUT /api/v3/{series|movie}/editor`",
        notes: "Sets `monitored=false`.",
    },
    EndpointRow {
        action: "add",
        tools: "sonarr/radarr",
        endpoint: "`GET /api/v3/{series|movie}/lookup?term=...`, `GET /api/v3/qualityprofile`, then `POST /api/v3/{series|movie}`",
        notes: "No write without `confirm=true`.",
    },
    EndpointRow {
        action: "delete",
        tools: "sonarr/radarr",
        endpoint: "`DELETE /api/v3/{series|movie}/{id}?deleteFiles={true|false}`",
        notes: "No delete without `confirm=true`.",
    },
];

const INDEXER_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "indexers",
        tools: "prowlarr",
        endpoint: "`GET /api/v1/indexer`",
        notes: "",
    },
    EndpointRow {
        action: "indexer_search",
        tools: "prowlarr",
        endpoint: "`GET /api/v1/search?query=...&type=search&limit=100[&indexerIds=...]`",
        notes: "",
    },
    EndpointRow {
        action: "indexer_stats",
        tools: "prowlarr",
        endpoint: "`GET /api/v1/indexerstats`",
        notes: "",
    },
    EndpointRow {
        action: "indexer_test",
        tools: "prowlarr",
        endpoint: "all: `POST /api/v1/indexer/testall`; one: `GET /api/v1/indexer/{id}` then `POST /api/v1/indexer/test`",
        notes: "Requires `confirm=true`.",
    },
];

const REQUEST_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "requests",
        tools: "overseerr",
        endpoint: "`GET /api/v1/request[?filter=&take=&skip=]`",
        notes: "",
    },
    EndpointRow {
        action: "request_search",
        tools: "overseerr",
        endpoint: "`GET /api/v1/search?query=...`",
        notes: "",
    },
    EndpointRow {
        action: "request_create",
        tools: "overseerr",
        endpoint: "`POST /api/v1/request`",
        notes: "Body `{mediaType, mediaId, seasons?}`. Requires `confirm=true`.",
    },
    EndpointRow {
        action: "request_approve",
        tools: "overseerr",
        endpoint: "`POST /api/v1/request/{id}/approve`",
        notes: "Requires `confirm=true` and `MANAGE_REQUESTS`.",
    },
    EndpointRow {
        action: "request_decline",
        tools: "overseerr",
        endpoint: "`POST /api/v1/request/{id}/decline`",
        notes: "Requires `confirm=true` and `MANAGE_REQUESTS`.",
    },
];

const STATS_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "stats_activity",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=get_activity`",
        notes: "",
    },
    EndpointRow {
        action: "stats_history",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=get_history[&start=&length=&user=]`",
        notes: "",
    },
    EndpointRow {
        action: "stats_users",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=get_users`",
        notes: "",
    },
    EndpointRow {
        action: "stats_libraries",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=get_library_names`",
        notes: "",
    },
    EndpointRow {
        action: "stats_refresh_libraries",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=refresh_libraries_list`",
        notes: "Requires `confirm=true`.",
    },
    EndpointRow {
        action: "stats_refresh_users",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=refresh_users_list`",
        notes: "Requires `confirm=true`.",
    },
    EndpointRow {
        action: "stats_delete_image_cache",
        tools: "tautulli",
        endpoint: "`GET /api/v2?cmd=delete_image_cache`",
        notes: "Requires `confirm=true`.",
    },
];

const DOWNLOAD_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "download_queue",
        tools: "sabnzbd",
        endpoint: "`GET /api?mode=queue&output=json`",
        notes: "qBittorrent uses `GET /api/v2/torrents/info`.",
    },
    EndpointRow {
        action: "download_add",
        tools: "sabnzbd",
        endpoint: "`GET /api?mode=addurl&name=<url>&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/add` with `urls=<url>`. Requires `confirm=true`.",
    },
    EndpointRow {
        action: "download_pause",
        tools: "sabnzbd",
        endpoint: "one: `GET /api?mode=queue&name=pause&value=<id>&output=json`; all: `GET /api?mode=pause&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/stop` with `hashes=<hash-or-all>`. Requires `confirm=true`.",
    },
    EndpointRow {
        action: "download_resume",
        tools: "sabnzbd",
        endpoint: "one: `GET /api?mode=queue&name=resume&value=<id>&output=json`; all: `GET /api?mode=resume&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/start` with `hashes=<hash-or-all>`. Requires `confirm=true`.",
    },
    EndpointRow {
        action: "download_remove",
        tools: "sabnzbd",
        endpoint: "`GET /api?mode=queue&name=delete&value=<id>[&del_files=1]&output=json`",
        notes: "qBittorrent uses form `POST /api/v2/torrents/delete` with `hashes=<hash>` and `deleteFiles={true|false}`. Requires `confirm=true`.",
    },
];

const MEDIA_ENDPOINTS: &[EndpointRow] = &[
    EndpointRow {
        action: "media_sessions",
        tools: "plex",
        endpoint: "`GET /status/sessions`",
        notes: "Jellyfin uses `GET /Sessions`.",
    },
    EndpointRow {
        action: "media_libraries",
        tools: "plex",
        endpoint: "`GET /library/sections`",
        notes: "Jellyfin uses `GET /Library/VirtualFolders`.",
    },
    EndpointRow {
        action: "media_search",
        tools: "plex",
        endpoint: "`GET /library/search?query=...`",
        notes: "Jellyfin uses `GET /Items?searchTerm=...&includeItemTypes=Movie,Series,Episode&recursive=true`.",
    },
    EndpointRow {
        action: "media_scan",
        tools: "plex",
        endpoint: "`GET /library/sections/{library}/refresh`",
        notes: "Jellyfin uses `POST /Library/Refresh` with `{}`. Requires `confirm=true`.",
    },
];
