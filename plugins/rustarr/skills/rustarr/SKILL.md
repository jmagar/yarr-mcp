---
name: rustarr
description: >
  This skill should be used when the user wants to query or automate their media
  automation stack — Sonarr (TV shows), Radarr (movies), Prowlarr (indexers),
  Tautulli (Plex stats), Overseerr (media requests), SABnzbd (Usenet downloads),
  qBittorrent (torrents), Plex (media server), or Jellyfin. Trigger phrases
  include: "what's downloading", "add a movie to Radarr", "search for a TV show",
  "Sonarr queue", "Radarr library", "what's in my download queue", "Plex status",
  "Prowlarr indexers", "check Overseerr requests", "qBittorrent torrents",
  "SABnzbd queue", "Tautulli stats", "is Sonarr healthy", "media stack status",
  "arr services", "show me what's being downloaded". Always use rustarr for these
  — do not attempt to reach service APIs directly without it.
---

# rustarr — Media Automation Stack

Rust MCP bridge to the `*arr` media stack and related services. The MCP surface is
**one tool, `yarr`**: it runs a JavaScript async arrow function (`code`) in a
sandbox (Code Mode) that reaches the whole fleet. Credentials are handled
server-side. Inside the script the fleet is reached through per-service callables
with the service baked in — generated OpenAPI operations for the 6 spec-backed
services, curated commands for download/stats, plus `api.<service>` raw passthrough
and `callTool`. Discover what's available with `codemode.search`/`codemode.describe`.

## The yarr tool + Code Mode actions

`yarr({ code })` runs the `codemode` action. Inside `code` you have:

- **Per-service callables** with the service baked in (no `service` param):
  `sonarr.get_series()`, `radarr.post_movie({ body })`, `prowlarr.get_indexer()`,
  `plex.get_sessions()`, … For the 6 spec-backed services these are generated from
  the upstream OpenAPI spec (the full API surface). DELETE operations are refused
  mid-script.
- **Raw passthrough**: `api.<service>.get/post/put/delete(path, body)`.
- **Discovery**: `codemode.search(query)` returns fully-qualified callables;
  `codemode.describe(path)` returns a callable's signature OR a response type's
  TypeScript interface (e.g. `codemode.describe("sonarr.SeriesResource")`).
- **Snippets**: `codemode.run(name, input)` and `codemode.snippets()`.
- **Artifacts**: `writeArtifact(path, content, options?)`.

The supporting actions (MCP-only; also on the CLI as `rustarr codemode` /
`rustarr snippet`): `codemode`, `op` (generated-operation dispatch),
`snippet_list`, `snippet_save`, `snippet_run`, `snippet_delete`. The generic
service actions remain: `service_status`, `api_get`, `api_post`, `api_put`,
`api_delete`, `help`.

The fleet kinds are `sonarr`, `radarr`, `prowlarr`, `overseerr`, `tautulli`,
`plex`, `tracearr`, `sabnzbd`, `qbittorrent`, `jellyfin`, and `bazarr`.

---

## Quick-Reference Examples

### Discovery and health

```js
// One yarr call: discover, then health-check across services.
async () => {
  const status = await sonarr.get_system_status();   // generated op (live)
  const found  = codemode.search("add movie").results.map(r => r.path);
  return { sonarr: status.version, found };
}
```

### Sonarr (TV shows)

```text
# List all series
mcp__rustarr__sonarr(action="api_get", path="/api/v3/series")

# Current download queue
mcp__rustarr__sonarr(action="api_get", path="/api/v3/queue")

# Recent history
mcp__rustarr__sonarr(action="api_get", path="/api/v3/history?pageSize=20")

# System status
mcp__rustarr__sonarr(action="api_get", path="/api/v3/system/status")

# Search for missing episodes of a series (replace 123 with series ID)
mcp__rustarr__sonarr(action="api_post", path="/api/v3/command",
  body={"name":"SeriesSearch","seriesId":123}, confirm=true)
```

### Radarr (movies)

```text
# List all movies
mcp__rustarr__radarr(action="api_get", path="/api/v3/movie")

# Current download queue
mcp__rustarr__radarr(action="api_get", path="/api/v3/queue")

# Recent history
mcp__rustarr__radarr(action="api_get", path="/api/v3/history?pageSize=20")

# System status
mcp__rustarr__radarr(action="api_get", path="/api/v3/system/status")

# Trigger a movie search (replace 456 with movie ID)
mcp__rustarr__radarr(action="api_post", path="/api/v3/command",
  body={"name":"MoviesSearch","movieIds":[456]}, confirm=true)

# Refresh a movie's metadata (replace 456 with movie ID)
mcp__rustarr__radarr(action="api_post", path="/api/v3/command",
  body={"name":"RefreshMovie","movieIds":[456]}, confirm=true)
```

### Prowlarr (indexers)

```text
# List indexers
mcp__rustarr__prowlarr(action="api_get", path="/api/v1/indexer")

# System status
mcp__rustarr__prowlarr(action="api_get", path="/api/v1/system/status")

# Search across indexers
mcp__rustarr__prowlarr(action="api_get", path="/api/v1/search?query=ubuntu&type=search")
```

### Tautulli (Plex stats)

```text
# Currently playing on Plex
mcp__rustarr__tautulli(action="api_get", path="/api/v2?cmd=get_activity")

# Recent history
mcp__rustarr__tautulli(action="api_get", path="/api/v2?cmd=get_history&length=20")

# Home stats
mcp__rustarr__tautulli(action="api_get", path="/api/v2?cmd=get_home_stats")
```

### Download clients

```text
# SABnzbd queue
mcp__rustarr__sabnzbd(action="api_get", path="/api?mode=queue&output=json")

# qBittorrent torrent list
mcp__rustarr__qbittorrent(action="api_get", path="/api/v2/torrents/info")

# qBittorrent transfer info
mcp__rustarr__qbittorrent(action="api_get", path="/api/v2/transfer/info")
```

### Overseerr (requests)

```text
# Pending media requests
mcp__rustarr__overseerr(action="api_get", path="/api/v1/request?filter=pending")

# All requests
mcp__rustarr__overseerr(action="api_get", path="/api/v1/request?take=20")
```

### Plex

```text
# Server status
mcp__rustarr__plex(action="api_get", path="/identity")

# Active sessions (who's watching)
mcp__rustarr__plex(action="api_get", path="/status/sessions")

# Libraries
mcp__rustarr__plex(action="api_get", path="/library/sections")
```

---

## Common Workflows

### "What's downloading right now?"

```text
# Check both arr queues
mcp__rustarr__sonarr(action="api_get", path="/api/v3/queue")
mcp__rustarr__radarr(action="api_get", path="/api/v3/queue")

# And the download clients
mcp__rustarr__sabnzbd(action="api_get", path="/api?mode=queue&output=json")
mcp__rustarr__qbittorrent(action="api_get", path="/api/v2/torrents/info?filter=downloading")
```

### "Is everything healthy?"

```text
mcp__rustarr__sonarr(action="integrations")
mcp__rustarr__sonarr(action="service_status")
mcp__rustarr__radarr(action="service_status")
mcp__rustarr__prowlarr(action="service_status")
```

### "Who's watching Plex right now?"

```text
mcp__rustarr__tautulli(action="api_get", path="/api/v2?cmd=get_activity")
# or via Plex directly:
mcp__rustarr__plex(action="api_get", path="/status/sessions")
```

---

## Gotchas

1. **`api_get` requires write scope.** Despite being read-only from the user's
   perspective, `api_get` dispatches generic HTTP requests and is gated at
   write-scope to prevent credential leakage via crafted paths. Your MCP token
   must have write scope.

2. **All three mutating generic actions need `confirm=true`.** `api_post`,
   `api_put`, and `api_delete` each require the `confirm` boolean explicitly. The
   server rejects the call without it — no service is touched.

3. **Never include credentials in `path`.** The configured service credentials
   live in server environment variables. Do not append `?apikey=...` or
   `&X-Api-Key=...` to paths — the server injects auth headers automatically.

4. **`service` names are exact.** Use `sonarr`, `radarr`, `prowlarr`,
   `tautulli`, `overseerr`, `sabnzbd`, `qbittorrent`, `plex`, `jellyfin`.
   Passing an unknown name returns a structured `unknown_service` error with
   the configured names listed.

5. **API paths are service-version-specific.** Sonarr/Radarr use `/api/v3/`;
   Prowlarr uses `/api/v1/`; Tautulli uses query params (`/api/v2?cmd=...`).
   Call `path="/api/v3/system/status"` first to confirm the API version.

6. **`integrations` is the fastest diagnostic.** Always call it first when
   something isn't working — it shows which services are configured and which
   are actually reachable.
