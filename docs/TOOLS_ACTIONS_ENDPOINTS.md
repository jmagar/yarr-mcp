---
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

## Service Kinds

There is one published MCP tool (`yarr`). The table below lists the service
*kinds* a configured service can take — each kind's capability, upstream API
prefix, and path allowlist (from `ServiceKind::descriptor()`). The 6 spec-backed
kinds (sonarr/radarr/prowlarr/overseerr/jellyfin/plex) expose their full upstream
API as generated operations; the rest keep curated commands or generic
passthrough only.

| Kind | Curated capability | API prefix | Path allowlist |
|---|---|---|---|
| `sonarr` | ArrManager | `/api/v3` | `/api/v3` |
| `radarr` | ArrManager | `/api/v3` | `/api/v3` |
| `prowlarr` | Indexer | `/api/v1` | `/api/v1` |
| `tautulli` | Stats | `/api/v2` | `/api, /api/v2` |
| `overseerr` | Requests | `/api/v1` | `/api/v1` |
| `bazarr` | GenericOnly | `/api` | `/api, /api/v2` |
| `tracearr` | GenericOnly | `/api/v1` | `/health, /api/v1` |
| `sabnzbd` | DownloadClient | `/api` | `/api, /api/v2` |
| `qbittorrent` | DownloadClient | `/api/v2` | `/api/v2` |
| `plex` | MediaServer | `(none)` | `/identity, /library, /status, /servers` |
| `jellyfin` | MediaServer | `(none)` | `/System, /Items, /Users, /Library, /Sessions` |

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


## Generic Actions

| Action | Params | Scope | Mutates | Upstream call |
|---|---|---|---:|---|
| `service_status` | none; service is implied by MCP tool or CLI service token | rustarr:read | no | GET the kind default status path, e.g. Sonarr/Radarr `/api/v3/system/status`, Prowlarr `/api/v1/system/status`, Overseerr `/api/v1/status`, Tautulli `/api/v2?cmd=get_server_info`, Bazarr `/api/system/status`, Tracearr `/health`, SABnzbd `/api?mode=version&output=json`, qBittorrent `/api/v2/app/version`, Plex `/identity`, Jellyfin `/System/Info/Public`. |
| `api_get` | `path` | rustarr:write | no | `GET {path}`. |
| `api_post` | `path`, optional `body` | rustarr:write | yes | `POST {path}` with JSON body. Runs immediately. |
| `api_put` | `path`, optional `body` | rustarr:write | yes | `PUT {path}` with JSON body. Runs immediately. |
| `api_delete` | `path`, optional `body`, `confirm` | rustarr:write | yes | `DELETE {path}` with optional JSON body. Destructive: gated by `--confirm`. |
| `help` | none | public | no | No upstream call; returns registry-derived action help. |
| `codemode` | `code` (a JavaScript async arrow function) | rustarr:write | no | No direct upstream call; runs a Code Mode script that dispatches other actions. |
| `op` | `op` (operation name), optional `args`, `confirm` (DELETE ops) | rustarr:write | yes | Dispatches a generated OpenAPI operation for a spec-backed service. |
| `snippet_list` | none | rustarr:read | no | No upstream call; manages the Code Mode snippet store under the data dir. |
| `snippet_save` | `name`, `code`, optional `description` | rustarr:write | yes | No upstream call; manages the Code Mode snippet store under the data dir. |
| `snippet_run` | `name`, optional `input` | rustarr:write | no | No upstream call; manages the Code Mode snippet store under the data dir. |
| `snippet_delete` | `name` | rustarr:write | yes | No upstream call; manages the Code Mode snippet store under the data dir. |

## Generated Operations (spec-backed services)

`sonarr`, `radarr`, `prowlarr`, `overseerr`, `jellyfin`, and `plex` are generated
from their vendored OpenAPI specs (`cargo xtask gen-openapi` →
`src/openapi/generated/`). Every spec operation becomes a per-service callable
(`sonarr.get_series()`, `radarr.post_movie({ body })`) dispatched via the `op`
action; there are no hand-written curated commands for these kinds. Discover them
with `codemode.search(query)` and inspect signatures / response types with
`codemode.describe(path)`. DELETE operations are refused mid-script (run them via
the CLI `op` with `--confirm`, or set `RUSTARR_ALLOW_DESTRUCTIVE`).


## Tautulli Actions

Tools: tautulli.

| Action | Params | Scope | Mutates | Upstream call | Notes |
|---|---|---|---:|---|---|
| `stats_activity` | none | rustarr:read | no | tautulli: `GET /api/v2?cmd=get_activity` |  |
| `stats_history` | optional `start`, optional `length`, optional `user` | rustarr:read | no | tautulli: `GET /api/v2?cmd=get_history[&start=&length=&user=]` |  |
| `stats_users` | none | rustarr:read | no | tautulli: `GET /api/v2?cmd=get_users` |  |
| `stats_libraries` | none | rustarr:read | no | tautulli: `GET /api/v2?cmd=get_library_names` |  |
| `stats_refresh_libraries` | none | rustarr:write | yes | tautulli: `GET /api/v2?cmd=refresh_libraries_list` | Runs immediately (not destructive). |
| `stats_refresh_users` | none | rustarr:write | yes | tautulli: `GET /api/v2?cmd=refresh_users_list` | Runs immediately (not destructive). |
| `stats_delete_image_cache` | optional `confirm` | rustarr:write | yes | tautulli: `GET /api/v2?cmd=delete_image_cache` | Destructive: gated by MCP elicitation / CLI `--confirm`. |

## SABnzbd And qBittorrent Actions

Tools: sabnzbd, qbittorrent.

| Action | Params | Scope | Mutates | Upstream call | Notes |
|---|---|---|---:|---|---|
| `download_queue` | none | rustarr:read | no | sabnzbd: `GET /api?mode=queue&output=json` | qBittorrent uses `GET /api/v2/torrents/info`. |
| `download_add` | `url` | rustarr:write | yes | sabnzbd: `GET /api?mode=addurl&name=<url>&output=json` | qBittorrent uses form `POST /api/v2/torrents/add` with `urls=<url>`. Runs immediately. |
| `download_pause` | optional `id`, optional `hash` | rustarr:write | yes | sabnzbd: one: `GET /api?mode=queue&name=pause&value=<id>&output=json`; all: `GET /api?mode=pause&output=json` | qBittorrent uses form `POST /api/v2/torrents/stop` with `hashes=<hash-or-all>`. Runs immediately. |
| `download_resume` | optional `id`, optional `hash` | rustarr:write | yes | sabnzbd: one: `GET /api?mode=queue&name=resume&value=<id>&output=json`; all: `GET /api?mode=resume&output=json` | qBittorrent uses form `POST /api/v2/torrents/start` with `hashes=<hash-or-all>`. Runs immediately. |
| `download_remove` | optional `id`, optional `hash`, optional `delete_files`, optional `confirm` | rustarr:write | yes | sabnzbd: `GET /api?mode=queue&name=delete&value=<id>[&del_files=1]&output=json` | qBittorrent uses form `POST /api/v2/torrents/delete` with `hashes=<hash>` and `deleteFiles={true|false}`. Destructive: gated by MCP elicitation / CLI `--confirm`. |

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

## CLI Verb Mapping

The CLI is service-grouped (`rustarr <service> <verb>`). Only the curated
capabilities below have friendly verbs; the spec-backed services use
`rustarr <service> op <operation>` (generated operations) or the generic
`get/post/put/delete` passthrough. Verb tables are read from the CLI registry.

| Capability | CLI verbs |
|---|---|
| DownloadClient | `queue`, `add`, `pause`, `resume`, `remove` |
| Stats | `activity`, `history`, `users`, `libraries`, `refresh-libraries`, `refresh-users`, `delete-image-cache` |
