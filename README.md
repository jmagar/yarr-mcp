# yarr

Rust MCP and CLI server for a media automation fleet: Sonarr, Radarr, Prowlarr, Tautulli, Overseerr, Bazarr, Tracearr, SABnzbd, qBittorrent, Plex, and Jellyfin.

If you run Claude Code, Codex, or Gemini CLI against a self-hosted media stack,
`yarr` gives an agent one consistent way to query and control all of it,
instead of eleven different ad hoc integrations. `yarr` is an upstream-client
MCP server — it does not try to replace those applications or mirror every
REST endpoint as a web UI. Its job is to provide one consistent tool surface
for agents and one equivalent CLI surface for operators.

**Not for:** a general-purpose REST gateway to arbitrary services, or a
scheduler/automation engine in its own right — it only talks to the 11
service kinds it knows about, and only does what you (or your agent) tell it
to, when asked.

## Two ways to install this

The [Claude Code / Codex / Gemini CLI marketplace](.claude-plugin/marketplace.json) in this repo ships **12 plugins**, not one — pick whichever shape fits. (`/plugin marketplace add`/`/plugin install` are Claude Code slash commands, run inside a chat session, not a shell — this section is Claude-Code-plugin-specific; the plain MCP server + CLI covered later in this README works standalone, with no plugin system involved.)

- **`yarr`** — the full package: the MCP server, the `yarr` CLI binary, and a
  bundled skill for every one of the 11 services that talks to that service
  directly over HTTP with `curl`. These are three independent ways to reach
  the same fleet, not an automatic runtime fallback chain — nothing detects
  the MCP server going down and silently reroutes. In practice, an agent
  reaches for the `yarr` MCP tool when the server is configured and reachable
  (its skill says to prefer it), drops to the `yarr` CLI when a human is
  scripting from a terminal, and the bundled per-service skills are there so
  an agent still has *something* to drive each service with even when
  neither the MCP server nor the `yarr` binary is installed at all.
- **One skills-only plugin per service** — `sonarr`, `radarr`, `prowlarr`,
  `overseerr`, `sabnzbd`, `qbittorrent`, `plex`, `jellyfin`, `tautulli`,
  `tracearr`, `bazarr`. Each is a standalone Claude Code/Codex/Gemini plugin
  with **no MCP server and no `yarr` CLI/binary** — just a skill whose helper
  script drives that one service's REST API directly with `curl`. Install
  only the services you actually run (e.g. just `plex` + `sonarr` +
  `radarr`) without touching Rust, Node, or this MCP server at all:

  ```
  /plugin marketplace add <this-repo-git-url>
  /plugin install sonarr@yarr
  /plugin install plex@yarr
  ```

See [`plugins/README.md`](plugins/README.md) for the full package layout,
credential bridging, and Codex/Gemini manifest details.

## Surfaces

| Surface | Status | Purpose |
|---|---:|---|
| MCP | Required | A single `yarr` tool that runs a Code Mode script over the whole fleet |
| CLI | Required | Scriptable parity surface for debugging and automation |
| REST | Not shipped | Upstream-client servers do not expose a local REST action API |
| Web | Not shipped | Upstream-client servers do not serve an embedded web UI |

The MCP surface is **one tool, `yarr`**: it takes a JavaScript async arrow function
(`code`) and runs it in an in-process QuickJS sandbox (Code Mode). Inside the
script the whole fleet is reached through per-service callables with the service
baked in — for the 6 spec-backed services (Sonarr, Radarr, Prowlarr, Overseerr,
Jellyfin, Plex) these are **generated from the upstream OpenAPI specs** (e.g.
`sonarr.get_series()`, `radarr.post_movie({...})`); the 5 doc-based kinds
(SABnzbd, qBittorrent, Tautulli, Bazarr, Tracearr — no machine-readable spec)
keep curated commands; plus `api.<service>.{get,post,put,delete}` raw
passthrough and `callTool` as an escape hatch. `codemode.search`/`describe`
discover callables and response types on demand. The CLI is **service-grouped**
(`yarr <service> <command> [flags]`, plus `yarr codemode --code <JS>`); CLI ↔
MCP parity is mechanically enforced by `tests/parity.rs`.

Set `YARR_MCP_TOOL_MODE=flat` to advertise one action-dispatched MCP tool per
configured service instead of the single Code Mode tool — useful behind a
gateway (e.g. Labby) that already provides its own dynamic-discovery/sandbox
layer, so the gateway gets real per-operation typed tools rather than one
opaque `{code: string}` tool wrapped inside its own sandbox. See
`docs/CONFIG.md` for details; `codemode` (the default) is the right choice
for a standalone MCP client with no discovery layer of its own.

## Generic actions

These work for every configured service kind:

| Action | Scope | CLI | Description |
|---|---|---|---|
| `service_status` | `yarr:read` | `yarr <service> status` | Fetch an upstream service status endpoint |
| `api_get` | `yarr:write` | `yarr <service> get --path <path>` | Proxy a safe credentialed GET request to a configured service |
| `api_post` | `yarr:write` | `yarr <service> post --path <path> --body <json>` | Proxy a POST request to a configured service (runs immediately) |
| `api_put` | `yarr:write` | `yarr <service> put --path <path> --body <json>` | Proxy a PUT request to a configured service (runs immediately) |
| `api_delete` | `yarr:write` | `yarr <service> delete --path <path>` | Proxy a DELETE request to a configured service (destructive; runs immediately, elicited on MCP) |
| `help` | public | `yarr help` | Return action reference |

Paths must be relative API paths. Query-string secrets such as `apikey=`, `token=`, and `X-Plex-Token` are rejected; credentials belong in config or environment variables.

## Code Mode actions

The MCP `yarr` tool dispatches the `codemode` action, and a script reaches the rest
of the fleet through these (MCP-only) actions, also available on the CLI via
`yarr codemode` / `yarr snippet`:

| Action | Scope | Surface | Description |
|---|---|---|---|
| `codemode` | `yarr:write` | `yarr` tool / `yarr codemode --code <JS>\|--file <path>` | Run a JS arrow function over the fleet; returns `{result, calls, logs, artifacts}` |
| `op` | `yarr:write` | inside Code Mode (`<service>.<operation>()`) / `yarr <service> op <name> [--args <JSON>]` | Dispatch a generated OpenAPI operation for a spec-backed service, including DELETE ops (MCP elicits confirmation before a generated DELETE dispatches, same as `api_delete`) |
| `snippet_list` | `yarr:read` | `yarr snippet list` / `codemode.snippets()` | List saved Code Mode snippets |
| `snippet_save` | `yarr:write` | `yarr snippet save` | Save a named, reusable Code Mode snippet |
| `snippet_run` | `yarr:write` | `yarr snippet run` / `codemode.run(name, input)` | Run a saved snippet (one level deep) |
| `snippet_delete` | `yarr:write` | `yarr snippet delete` | Delete a saved snippet |

Destructive deletes dispatch immediately in Code Mode, same as any other write — there is no confirmation channel mid-script. On the MCP surface a destructive action gets a real interactive elicitation prompt before it dispatches.

## Install

The recommended install path is the Node launcher package:

```bash
# Run the stdio MCP server without a permanent install
npx -y yarr-mcp mcp

# Or install the launcher globally
npm i -g yarr-mcp
yarr --version
yarr mcp
```

The npm package downloads the matching GitHub Release binary during install and
adds a `yarr` command to `PATH`. It does not expose legacy command aliases.

For machines without npm, use the one-line release installer:

```bash
curl -fsSL https://raw.githubusercontent.com/jmagar/yarr/main/scripts/install.sh | bash
```

That script installs `yarr` into `~/.local/bin`.

Running this against a media stack you already manage with Docker Compose?
`docker-compose.yml` / `docker-compose.prod.yml` build and run `yarr` as a
container instead — see `docs/DOCKER.md` and `docs/DEPLOYMENT.md`.

## Generated operations vs curated commands

The 6 spec-backed services are served by **generated OpenAPI operations** — the
entire upstream API surface (e.g. Sonarr's ~235 operations / ~136 component types),
generated from the vendored specs in `specs/` by `cargo xtask gen-openapi` into
`src/openapi/generated/`. Inside Code Mode they are per-service callables you
discover with `codemode.search`/`describe` and invoke directly, with the service
baked in:

```js
async () => {
  const status = await sonarr.get_system_status();   // generated op
  const movie  = await radarr.post_movie({ body });   // generated op (POST)
  return { status, added: movie };
}
```

5 of the 11 service kinds have no machine-readable spec (SABnzbd, qBittorrent,
Tautulli, Tracearr, Bazarr) — all 5 keep curated, slimmed CLI commands. All
curated commands, including the destructive `download_remove`,
`stats_delete_image_cache`, and `trace_terminate_stream`, run immediately;
MCP additionally elicits confirmation for the destructive ones before
dispatch. Any endpoint not covered by a curated command is still reachable
through the generic passthrough.

| Capability (kinds) | Surface | Examples |
|---|---|---|
| Sonarr/Radarr/Prowlarr/Overseerr/Jellyfin/Plex | Generated ops (Code Mode) | `sonarr.get_series()`, `radarr.post_movie({...})`, `prowlarr.get_indexer()`, `plex.get_sessions()` |
| DownloadClient (sabnzbd, qbittorrent) | Curated commands | `yarr qbittorrent queue`, `add --url X`, `pause`, `resume`, `remove --hash H` |
| Stats (tautulli) | Curated commands | `yarr tautulli activity`, `history`, `users`, `libraries`, `refresh-libraries` |
| Subtitles (bazarr) | Curated commands | `yarr bazarr status-info`, `movies`, `wanted-movies`, `providers` |
| Trace (tracearr) | Curated commands | `yarr tracearr health`, `stats`, `streams`, `history`, `terminate-stream --id X` |

## Configuration

Copy `.env.example` or use `config.example.toml` as a starting point. Common variables:

```bash
YARR_MCP_HOST=127.0.0.1
YARR_MCP_PORT=40070
YARR_MCP_TOKEN=change-me

YARR_SERVICES=sonarr,radarr,prowlarr,tautulli,overseerr,bazarr,tracearr,sabnzbd,qbittorrent,plex,jellyfin
YARR_SONARR_URL=http://sonarr:8989
YARR_SONARR_API_KEY=...
YARR_RADARR_URL=http://radarr:7878
YARR_RADARR_API_KEY=...
YARR_QBITTORRENT_URL=http://qbittorrent:8080
YARR_QBITTORRENT_USERNAME=...
YARR_QBITTORRENT_PASSWORD=...
YARR_PLEX_URL=http://plex:32400
YARR_PLEX_TOKEN=...
```

The `*_API_KEY` pattern covers most Arr-style services. qBittorrent uses username/password login. Plex and Jellyfin token headers are handled separately.

`YARR_MCP_TOKEN` gates every request once `yarr` is reachable beyond
loopback — on `127.0.0.1`/`localhost` with no explicit token, auth is
bypassed for local dev. Set a real token (or `YARR_MCP_AUTH_MODE=oauth` for
Google OAuth) before exposing this on a network, since it holds API keys for
every configured service. See `docs/AUTH.md` for the full loopback/bearer/OAuth
trust model.

## Run

```bash
yarr help
yarr radarr status
yarr sonarr get --path /api/v3/system/status
yarr radarr post --path /api/v3/command --body '{"name":"RefreshMovie"}'

# generated ops (spec-backed services) + curated commands
yarr sonarr op get_series
yarr qbittorrent queue
yarr tautulli activity

yarr serve
yarr mcp
```

HTTP MCP endpoint:

```bash
curl -s http://127.0.0.1:40070/mcp \
  -H "Authorization: Bearer $YARR_MCP_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"yarr","arguments":{"code":"async () => await sonarr.get_system_status()"}}}'
```

## MCP Client Configuration

Streamable HTTP:

```json
{
  "mcpServers": {
    "yarr": {
      "url": "http://127.0.0.1:40070/mcp",
      "headers": {
        "Authorization": "Bearer ${YARR_MCP_TOKEN}"
      }
    }
  }
}
```

stdio:

```json
{
  "mcpServers": {
    "yarr": {
      "command": "yarr",
      "args": ["mcp"],
      "env": {
        "RUST_LOG": "info,yarr=debug"
      }
    }
  }
}
```

## Architecture

```text
MCP shim (src/mcp/tools.rs)   CLI shim (src/cli.rs)
  JSON args -> dispatch          argv -> dispatch
        \                          /
         execute_service_action (src/actions/dispatch.rs)
             shared validation + curated-command dispatch
                          ↓
         YarrService (src/app.rs + src/app/*.rs)
           validation, service lookup, response shaping
                          ↓
         YarrClient (src/yarr.rs)
           network calls and auth headers
```

Both `src/mcp.rs` and `src/cli.rs` are thin facades re-exporting their own
submodules (`src/mcp/*.rs`, `src/cli/*.rs`) — e.g. MCP scope checks live in
`src/mcp/rmcp_server.rs`, not `tools.rs`. `execute_service_action` is what
makes CLI ↔ MCP parity structural rather than something to keep in sync by
hand — both surfaces marshal into the same dispatch call.

The service layer owns business rules:

- supported service catalog and config lookup
- safe path validation
- credential redaction
- qBittorrent login flow
- response normalization

## Development

```bash
cargo run -- help
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
```

Useful docs:

- `docs/API.md` for action contracts
- `docs/CONFIG.md` for environment and config details
- `docs/QUICKSTART.md` for local smoke tests
- `docs/MCP_SCHEMA.md` for schema drift rules
- `docs/AUTH.md` for the bearer/OAuth auth model
- `docs/DEPLOYMENT.md` and `docs/DOCKER.md` for running this in production
- `docs/PATTERNS.md` for the conventions shared across the `rmcp-server` family
- `docs/PLUGINS.md` for how the marketplace plugins are built and wired up
- `plugins/README.md` for the `yarr` vs. per-service plugin layout
- `plugins/yarr/README.md` and `plugins/yarr/skills/yarr/SKILL.md` for the `yarr` plugin package specifically
- `CLAUDE.md`'s "How to add an action" checklist for extending the curated command surface

## License

[MIT](LICENSE)
