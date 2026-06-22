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
last_reviewed: "2026-06-18"
---

# Tools, Actions, Params, and Endpoints

<!-- GENERATED: do not edit by hand. Run `cargo xtask tool-docs`. -->

This reference maps the Rustarr MCP/CLI action surface to the upstream HTTP
endpoints it calls. Action names, params, scopes, and mutability are read from
the Rust action registry. Endpoint mappings are rendered from the structured
generator table in `xtask/src/tool_docs/endpoints.rs`.

## MCP Tools

| Tool | Kind | Curated capability | API prefix | Path allowlist |
|---|---|---|---|---|
| `sonarr` | `sonarr` | ArrManager | `/api/v3` | `/api/v3` |
| `radarr` | `radarr` | ArrManager | `/api/v3` | `/api/v3` |
| `prowlarr` | `prowlarr` | Indexer | `/api/v1` | `/api/v1` |
| `tautulli` | `tautulli` | Stats | `/api/v2` | `/api, /api/v2` |
| `overseerr` | `overseerr` | Requests | `/api/v1` | `/api/v1` |
| `bazarr` | `bazarr` | GenericOnly | `/api` | `/api, /api/v2` |
| `tracearr` | `tracearr` | GenericOnly | `/api/v1` | `/health, /api/v1` |
| `sabnzbd` | `sabnzbd` | DownloadClient | `/api` | `/api, /api/v2` |
| `qbittorrent` | `qbittorrent` | DownloadClient | `/api/v2` | `/api/v2` |
| `plex` | `plex` | MediaServer | `(none)` | `/identity, /library, /status, /servers` |
| `jellyfin` | `jellyfin` | MediaServer | `(none)` | `/System, /Items, /Users, /Library, /Sessions` |

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


## Generic Actions

| Action | Params | Scope | Mutates | Upstream call |
|---|---|---|---:|---|
| `integrations` | none | rustarr:read | no | No upstream call; returns configured/supported service catalog. |
| `service_status` | none; service is implied by MCP tool or CLI service token | rustarr:read | no | GET the kind default status path, e.g. Sonarr/Radarr `/api/v3/system/status`, Prowlarr `/api/v1/system/status`, Overseerr `/api/v1/status`, Tautulli `/api/v2?cmd=get_server_info`, Bazarr `/api/system/status`, Tracearr `/health`, SABnzbd `/api?mode=version&output=json`, qBittorrent `/api/v2/app/version`, Plex `/identity`, Jellyfin `/System/Info/Public`. |
| `api_get` | `path` | rustarr:write | no | `GET {path}`. |
| `api_post` | `path`, optional `body`, `confirm` | rustarr:write | yes | `POST {path}` with JSON body. |
| `api_put` | `path`, optional `body`, `confirm` | rustarr:write | yes | `PUT {path}` with JSON body. |
| `api_delete` | `path`, optional `body`, `confirm` | rustarr:write | yes | `DELETE {path}` with optional JSON body. |
| `help` | none | public | no | No upstream call; returns registry-derived action help. |

## Sonarr And Radarr Actions

Tools: sonarr, radarr.

| Action | Params | Scope | Mutates | Upstream call | Notes |
|---|---|---|---:|---|---|
| `quality_profiles` | none | rustarr:read | no | sonarr/radarr: `GET /api/v3/qualityprofile` |  |
| `list` | optional `limit`, optional `offset`, optional `fields` | rustarr:read | no | sonarr: `GET /api/v3/series` | Radarr uses `GET /api/v3/movie`. |
| `wanted` | none | rustarr:read | no | sonarr/radarr: `GET /api/v3/wanted/missing?pageSize=50` |  |
| `queue` | none | rustarr:read | no | sonarr/radarr: `GET /api/v3/queue?pageSize=50` |  |
| `history` | none | rustarr:read | no | sonarr/radarr: `GET /api/v3/history?pageSize=50` |  |
| `rootfolders` | none | rustarr:read | no | sonarr/radarr: `GET /api/v3/rootfolder` |  |
| `health` | none | rustarr:read | no | sonarr/radarr: `GET /api/v3/health` |  |
| `set_quality` | `to`, optional `from`, optional `title`, optional `ids`, optional `bulk` | rustarr:write | yes | sonarr/radarr: `GET /api/v3/qualityprofile`, `GET /api/v3/{series|movie}`, then `PUT /api/v3/{series|movie}/editor` | No write without `confirm=true`. |
| `search` | optional `ids`, optional `bulk` | rustarr:write | yes | sonarr/radarr: `POST /api/v3/command` | Radarr can batch ids; Sonarr fans out one command per id. |
| `refresh` | optional `ids`, optional `bulk` | rustarr:write | yes | sonarr/radarr: `POST /api/v3/command` | Radarr can batch ids; Sonarr fans out one command per id. |
| `monitor` | optional `title`, optional `ids`, optional `bulk` | rustarr:write | yes | sonarr/radarr: `GET /api/v3/{series|movie}`, then `PUT /api/v3/{series|movie}/editor` | Sets `monitored=true`. |
| `unmonitor` | optional `title`, optional `ids`, optional `bulk` | rustarr:write | yes | sonarr/radarr: `GET /api/v3/{series|movie}`, then `PUT /api/v3/{series|movie}/editor` | Sets `monitored=false`. |
| `add` | `term`, `quality_profile`, `root_folder` | rustarr:write | yes | sonarr/radarr: `GET /api/v3/{series|movie}/lookup?term=...`, `GET /api/v3/qualityprofile`, then `POST /api/v3/{series|movie}` | No write without `confirm=true`. |
| `delete` | `id`, optional `delete_files`, optional `confirm` | rustarr:write | yes | sonarr/radarr: `DELETE /api/v3/{series|movie}/{id}?deleteFiles={true|false}` | No delete without `confirm=true`. |

## Prowlarr Actions

Tools: prowlarr.

| Action | Params | Scope | Mutates | Upstream call | Notes |
|---|---|---|---:|---|---|
| `indexers` | none | rustarr:read | no | prowlarr: `GET /api/v1/indexer` |  |
| `indexer_search` | `query`, optional `ids` | rustarr:read | no | prowlarr: `GET /api/v1/search?query=...&type=search&limit=100[&indexerIds=...]` |  |
| `indexer_stats` | none | rustarr:read | no | prowlarr: `GET /api/v1/indexerstats` |  |
| `indexer_test` | optional `id` | rustarr:write | yes | prowlarr: all: `POST /api/v1/indexer/testall`; one: `GET /api/v1/indexer/{id}` then `POST /api/v1/indexer/test` | Requires `confirm=true`. |

## Overseerr Actions

Tools: overseerr.

| Action | Params | Scope | Mutates | Upstream call | Notes |
|---|---|---|---:|---|---|
| `requests` | optional `filter`, optional `take`, optional `skip` | rustarr:read | no | overseerr: `GET /api/v1/request[?filter=&take=&skip=]` |  |
| `request_create` | `media_type`, `media_id`, optional `seasons` | rustarr:write | yes | overseerr: `POST /api/v1/request` | Body `{mediaType, mediaId, seasons?}`. Requires `confirm=true`. |
| `request_approve` | `id` | rustarr:write | yes | overseerr: `POST /api/v1/request/{id}/approve` | Requires `confirm=true` and `MANAGE_REQUESTS`. |
| `request_decline` | `id` | rustarr:write | yes | overseerr: `POST /api/v1/request/{id}/decline` | Requires `confirm=true` and `MANAGE_REQUESTS`. |
| `request_search` | `query` | rustarr:read | no | overseerr: `GET /api/v1/search?query=...` |  |

## Tautulli Actions

Tools: tautulli.

| Action | Params | Scope | Mutates | Upstream call | Notes |
|---|---|---|---:|---|---|
| `stats_activity` | none | rustarr:read | no | tautulli: `GET /api/v2?cmd=get_activity` |  |
| `stats_history` | optional `start`, optional `length`, optional `user` | rustarr:read | no | tautulli: `GET /api/v2?cmd=get_history[&start=&length=&user=]` |  |
| `stats_users` | none | rustarr:read | no | tautulli: `GET /api/v2?cmd=get_users` |  |
| `stats_libraries` | none | rustarr:read | no | tautulli: `GET /api/v2?cmd=get_library_names` |  |
| `stats_refresh_libraries` | none | rustarr:write | yes | tautulli: `GET /api/v2?cmd=refresh_libraries_list` | Requires `confirm=true`. |
| `stats_refresh_users` | none | rustarr:write | yes | tautulli: `GET /api/v2?cmd=refresh_users_list` | Requires `confirm=true`. |
| `stats_delete_image_cache` | optional `confirm` | rustarr:write | yes | tautulli: `GET /api/v2?cmd=delete_image_cache` | Requires `confirm=true`. |

## SABnzbd And qBittorrent Actions

Tools: sabnzbd, qbittorrent.

| Action | Params | Scope | Mutates | Upstream call | Notes |
|---|---|---|---:|---|---|
| `download_queue` | none | rustarr:read | no | sabnzbd: `GET /api?mode=queue&output=json` | qBittorrent uses `GET /api/v2/torrents/info`. |
| `download_add` | `url` | rustarr:write | yes | sabnzbd: `GET /api?mode=addurl&name=<url>&output=json` | qBittorrent uses form `POST /api/v2/torrents/add` with `urls=<url>`. Requires `confirm=true`. |
| `download_pause` | optional `id`, optional `hash` | rustarr:write | yes | sabnzbd: one: `GET /api?mode=queue&name=pause&value=<id>&output=json`; all: `GET /api?mode=pause&output=json` | qBittorrent uses form `POST /api/v2/torrents/stop` with `hashes=<hash-or-all>`. Requires `confirm=true`. |
| `download_resume` | optional `id`, optional `hash` | rustarr:write | yes | sabnzbd: one: `GET /api?mode=queue&name=resume&value=<id>&output=json`; all: `GET /api?mode=resume&output=json` | qBittorrent uses form `POST /api/v2/torrents/start` with `hashes=<hash-or-all>`. Requires `confirm=true`. |
| `download_remove` | optional `id`, optional `hash`, optional `delete_files`, optional `confirm` | rustarr:write | yes | sabnzbd: `GET /api?mode=queue&name=delete&value=<id>[&del_files=1]&output=json` | qBittorrent uses form `POST /api/v2/torrents/delete` with `hashes=<hash>` and `deleteFiles={true|false}`. Requires `confirm=true`. |

## Plex And Jellyfin Actions

Tools: plex, jellyfin.

| Action | Params | Scope | Mutates | Upstream call | Notes |
|---|---|---|---:|---|---|
| `media_sessions` | none | rustarr:read | no | plex: `GET /status/sessions` | Jellyfin uses `GET /Sessions`. |
| `media_libraries` | none | rustarr:read | no | plex: `GET /library/sections` | Jellyfin uses `GET /Library/VirtualFolders`. |
| `media_search` | `query` | rustarr:read | no | plex: `GET /library/search?query=...` | Jellyfin uses `GET /Items?searchTerm=...&includeItemTypes=Movie,Series,Episode&recursive=true`. |
| `media_scan` | optional `library` | rustarr:write | yes | plex: `GET /library/sections/{library}/refresh` | Jellyfin uses `POST /Library/Refresh` with `{}`. Requires `confirm=true`. |
## GenericOnly Services

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
