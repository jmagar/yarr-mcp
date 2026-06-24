# rustarr

Rust MCP and CLI server for a media automation fleet: Sonarr, Radarr, Prowlarr, Tautulli, Overseerr, Bazarr, Tracearr, SABnzbd, qBittorrent, Plex, and Jellyfin.

`rustarr` is an upstream-client MCP server. It does not try to replace those applications or mirror every REST endpoint as a web UI. Its job is to provide one consistent tool surface for agents and one equivalent CLI surface for operators.

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
`sonarr.get_series()`, `radarr.post_movie({...})`); the 2 doc-based download/stats
capabilities keep curated commands; plus `api.<service>.{get,post,put,delete}` raw
passthrough and `callTool` as an escape hatch. `codemode.search`/`describe` discover
callables and response types on demand. The CLI is **service-grouped**
(`rustarr <service> <command> [flags]`, plus `rustarr codemode --code <JS>`); CLI ↔
MCP parity is mechanically enforced by `tests/parity.rs`.

## Generic actions

These work for every configured service kind:

| Action | Scope | CLI | Description |
|---|---|---|---|
| `service_status` | `rustarr:read` | `rustarr <service> status` | Fetch an upstream service status endpoint |
| `api_get` | `rustarr:write` | `rustarr <service> get --path <path>` | Proxy a safe credentialed GET request to a configured service |
| `api_post` | `rustarr:write` | `rustarr <service> post --path <path> --body <json>` | Proxy a POST request to a configured service (runs immediately) |
| `api_put` | `rustarr:write` | `rustarr <service> put --path <path> --body <json>` | Proxy a PUT request to a configured service (runs immediately) |
| `api_delete` | `rustarr:write` | `rustarr <service> delete --path <path> --confirm` | Proxy a DELETE request to a configured service (destructive; requires confirm) |
| `help` | public | `rustarr help` | Return action reference |

Paths must be relative API paths. Query-string secrets such as `apikey=`, `token=`, and `X-Plex-Token` are rejected; credentials belong in config or environment variables.

## Code Mode actions

The MCP `yarr` tool dispatches the `codemode` action, and a script reaches the rest
of the fleet through these (MCP-only) actions, also available on the CLI via
`rustarr codemode` / `rustarr snippet`:

| Action | Scope | Surface | Description |
|---|---|---|---|
| `codemode` | `rustarr:write` | `yarr` tool / `rustarr codemode --code <JS>\|--file <path>` | Run a JS arrow function over the fleet; returns `{result, calls, logs, artifacts}` |
| `op` | `rustarr:write` | inside Code Mode (`<service>.<operation>()`) | Dispatch a generated OpenAPI operation for a spec-backed service. DELETE ops are refused mid-script |
| `snippet_list` | `rustarr:read` | `rustarr snippet list` / `codemode.snippets()` | List saved Code Mode snippets |
| `snippet_save` | `rustarr:write` | `rustarr snippet save` | Save a named, reusable Code Mode snippet |
| `snippet_run` | `rustarr:write` | `rustarr snippet run` / `codemode.run(name, input)` | Run a saved snippet (one level deep) |
| `snippet_delete` | `rustarr:write` | `rustarr snippet delete` | Delete a saved snippet |

Destructive deletes are refused inside Code Mode (no confirmation channel mid-script); call them directly with `--confirm`.

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

The 2 doc-based capabilities (no machine-readable spec) keep curated, slimmed CLI
commands. Mutating download commands require `--confirm`; `stats_delete_image_cache`
is the one destructive stats command and is also confirm-gated.

| Capability (kinds) | Surface | Examples |
|---|---|---|
| Sonarr/Radarr/Prowlarr/Overseerr/Jellyfin/Plex | Generated ops (Code Mode) | `sonarr.get_series()`, `radarr.post_movie({...})`, `prowlarr.get_indexer()`, `plex.get_sessions()` |
| DownloadClient (sabnzbd, qbittorrent) | Curated commands | `rustarr qbittorrent queue`, `add --url X`, `pause`, `resume`, `remove --hash H --confirm` |
| Stats (tautulli) | Curated commands | `rustarr tautulli activity`, `history`, `users`, `libraries`, `refresh-libraries` |

Tracearr and bazarr have no spec and no curated surface — use the generic passthrough.

## Configuration

Copy `.env.example` or use `config.example.toml` as a starting point. Common variables:

```bash
RUSTARR_MCP_HOST=127.0.0.1
RUSTARR_MCP_PORT=40070
RUSTARR_MCP_TOKEN=change-me

RUSTARR_SERVICES=sonarr,radarr,prowlarr,tautulli,overseerr,bazarr,tracearr,sabnzbd,qbittorrent,plex,jellyfin
RUSTARR_SONARR_URL=http://sonarr:8989
RUSTARR_SONARR_API_KEY=...
RUSTARR_RADARR_URL=http://radarr:7878
RUSTARR_RADARR_API_KEY=...
RUSTARR_QBITTORRENT_URL=http://qbittorrent:8080
RUSTARR_QBITTORRENT_USERNAME=...
RUSTARR_QBITTORRENT_PASSWORD=...
RUSTARR_PLEX_URL=http://plex:32400
RUSTARR_PLEX_TOKEN=...
```

The `*_API_KEY` pattern covers most Arr-style services. qBittorrent uses username/password login. Plex and Jellyfin token headers are handled separately.

## Run

```bash
cargo run -- help
cargo run -- radarr status
cargo run -- sonarr get --path /api/v3/system/status
cargo run -- radarr post --path /api/v3/command --body '{"name":"RefreshMovie"}' --confirm

# generated ops (spec-backed services) + curated commands
cargo run -- sonarr op get_series
cargo run -- qbittorrent queue
cargo run -- tautulli activity

cargo run -- serve
cargo run -- mcp
```

HTTP MCP endpoint:

```bash
curl -s http://127.0.0.1:40070/mcp \
  -H "Authorization: Bearer $RUSTARR_MCP_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"yarr","arguments":{"code":"async () => await sonarr.get_system_status()"}}}'
```

## MCP Client Configuration

Streamable HTTP:

```json
{
  "mcpServers": {
    "rustarr": {
      "url": "http://127.0.0.1:40070/mcp",
      "headers": {
        "Authorization": "Bearer ${RUSTARR_MCP_TOKEN}"
      }
    }
  }
}
```

stdio:

```json
{
  "mcpServers": {
    "rustarr": {
      "command": "/path/to/rustarr/target/release/rustarr",
      "args": ["mcp"],
      "env": {
        "RUST_LOG": "info,rustarr=debug"
      }
    }
  }
}
```

## Architecture

```text
RustarrClient  (src/rustarr.rs)   network calls and auth headers
      ↓
RustarrService (src/app.rs)       validation, service lookup, response shaping
      ↓
MCP shim       (src/mcp/tools.rs) JSON args -> service -> Value
CLI shim       (src/cli.rs)       argv -> service -> stdout
```

The service layer owns business rules:

- supported service catalog and config lookup
- safe path validation
- credential redaction
- qBittorrent login flow
- response normalization

## Development

```bash
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
- `plugins/rustarr/README.md` and `plugins/rustarr/skills/rustarr/SKILL.md` for plugin packaging
