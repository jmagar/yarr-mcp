# rustarr

Rust MCP and CLI server for a media automation fleet: Sonarr, Radarr, Prowlarr, Tautulli, Overseerr, Bazarr, Tracearr, SABnzbd, qBittorrent, Plex, and Jellyfin.

`rustarr` is an upstream-client MCP server. It does not try to replace those applications or mirror every REST endpoint as a web UI. Its job is to provide one consistent tool surface for agents and one equivalent CLI surface for operators.

## Surfaces

| Surface | Status | Purpose |
|---|---:|---|
| MCP | Required | Agent-facing tool calls through service-named tools |
| CLI | Required | Scriptable parity surface for debugging and automation |
| REST | Not shipped | Upstream-client servers do not expose a local REST action API |
| Web | Not shipped | Upstream-client servers do not serve an embedded web UI |

Every business action is implemented under `src/app/` and exposed through both MCP and CLI. `src/mcp/tools.rs` and `src/cli.rs` parse inputs and delegate only.

The CLI is **service-grouped** (`rustarr <service> <command> [flags]`); MCP exposes one tool per service kind (`sonarr`, `radarr`, etc.) dispatched by `action`. CLI ↔ MCP parity is mechanically enforced by `tests/parity.rs`.

## Generic actions

These work for every configured service kind:

| Action | Scope | CLI | Description |
|---|---|---|---|
| `integrations` | `rustarr:read` | `rustarr integrations` | List supported services and configured service names without secrets |
| `service_status` | `rustarr:read` | `rustarr <service> status` | Fetch an upstream service status endpoint |
| `api_get` | `rustarr:write` | `rustarr <service> get --path <path>` | Proxy a safe credentialed GET request to a configured service |
| `api_post` | `rustarr:write` | `rustarr <service> post --path <path> --body <json> --confirm` | Proxy a confirmed POST request to a configured service |
| `api_put` | `rustarr:write` | `rustarr <service> put --path <path> --body <json> --confirm` | Proxy a confirmed PUT request to a configured service |
| `api_delete` | `rustarr:write` | `rustarr <service> delete --path <path> --confirm` | Proxy a confirmed DELETE request to a configured service |
| `help` | public | `rustarr help` | Return action reference |

Paths must be relative API paths. Query-string secrets such as `apikey=`, `token=`, and `X-Plex-Token` are rejected; credentials belong in config or environment variables.

## Curated commands

Beyond the generic passthrough, each capability exposes curated, slimmed,
confirm-gated commands. Run `rustarr --help` for the generated, per-service list,
or `rustarr help` for the JSON action reference. Reads are `rustarr:read`; mutating
commands are `rustarr:write`. Two confirm behaviors apply:

- **Arr intent commands** (`set-quality`, `add`, `delete`, `search`, `refresh`,
  `monitor`/`unmonitor`) return a structured dry-run **preview** when `--confirm`
  is absent, and only mutate once you pass `--confirm`.
- **All other mutating commands** (download `add`/`pause`/`resume`/`remove`,
  request `create`/`approve`/`decline`, media `scan`, indexer `test`, and the
  generic `api_post`/`api_put`/`api_delete`) **require** `--confirm` and error
  without it.

| Capability (kinds) | Example commands |
|---|---|
| ArrManager (sonarr, radarr) | `rustarr sonarr list`, `rustarr sonarr set-quality --from X --to Y --confirm`, `wanted`, `queue`, `history`, `add`, `delete` |
| Indexer (prowlarr) | `rustarr prowlarr indexers`, `search --query X`, `stats`, `test --confirm` |
| DownloadClient (sabnzbd, qbittorrent) | `rustarr qbittorrent queue`, `add --url X --confirm`, `pause`, `resume`, `remove --hash H --confirm` |
| MediaServer (plex, jellyfin) | `rustarr plex sessions`, `libraries`, `search --query X`, `scan --library N --confirm` |
| Requests (overseerr) | `rustarr overseerr requests`, `request --media-type movie --media-id N --confirm`, `approve`, `decline`, `search` |
| Stats (tautulli) | `rustarr tautulli activity`, `history`, `users`, `libraries` |

Tracearr and bazarr have no curated surface yet — use the generic passthrough.

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
cargo run -- integrations
cargo run -- radarr status
cargo run -- sonarr get --path /api/v3/system/status
cargo run -- radarr post --path /api/v3/command --body '{"name":"RefreshMovie"}' --confirm

# curated commands
cargo run -- sonarr list
cargo run -- tautulli activity

cargo run -- serve
cargo run -- mcp
```

HTTP MCP endpoint:

```bash
curl -s http://127.0.0.1:40070/mcp \
  -H "Authorization: Bearer $RUSTARR_MCP_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"sonarr","arguments":{"action":"integrations"}}}'
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
