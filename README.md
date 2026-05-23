# rustarr

Rust MCP and CLI server for a media automation fleet: Sonarr, Radarr, Prowlarr, Tautulli, Overseerr, Bazarr, Tracearr, Lidarr, Readarr, SABnzbd, qBittorrent, Wizarr, Notifiarr, Plex, and Jellyfin.

`rustarr` is an upstream-client MCP server. It does not try to replace those applications or mirror every REST endpoint as a web UI. Its job is to provide one consistent tool surface for agents and one equivalent CLI surface for operators.

## Surfaces

| Surface | Status | Purpose |
|---|---:|---|
| MCP | Required | Agent-facing tool calls through the `rustarr` tool |
| CLI | Required | Scriptable parity surface for debugging and automation |
| REST | Present | Thin local HTTP action endpoint from the template |
| Web | Present | Lightweight template admin shell, not the primary surface |

Every business action is implemented in `src/app.rs` and exposed through both MCP and CLI. `src/mcp/tools.rs` and `src/cli.rs` parse inputs and delegate only.

## Actions

| Action | Scope | CLI | Description |
|---|---|---|---|
| `integrations` | `rustarr:read` | `rustarr integrations` | List supported services and configured service names without secrets |
| `service_status` | `rustarr:read` | `rustarr status --service <name>` | Fetch an upstream service status endpoint |
| `api_get` | `rustarr:read` | `rustarr get --service <name> --path <path>` | Proxy a safe GET request to a configured service |
| `api_post` | `rustarr:write` | `rustarr post --service <name> --path <path> --body <json>` | Proxy a safe POST request to a configured service |
| `help` | public | `rustarr help` | Return action reference |

Paths must be relative API paths. Query-string secrets such as `apikey=`, `token=`, and `X-Plex-Token` are rejected; credentials belong in config or environment variables.

## Configuration

Copy `.env.example` or use `config.example.toml` as a starting point. Common variables:

```bash
RUSTARR_MCP_HOST=127.0.0.1
RUSTARR_MCP_PORT=3100
RUSTARR_MCP_TOKEN=change-me

RUSTARR_SERVICES=sonarr,radarr,prowlarr,tautulli,overseerr,bazarr,tracearr,lidarr,readarr,sabnzbd,qbittorrent,wizarr,notifiarr,plex,jellyfin
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
cargo run -- status --service radarr
cargo run -- get --service sonarr --path /api/v3/system/status
cargo run -- post --service radarr --path /api/v3/command --body '{"name":"RefreshMovie"}'

cargo run -- serve
cargo run -- mcp
```

HTTP MCP endpoint:

```bash
curl -s http://127.0.0.1:3100/mcp \
  -H "Authorization: Bearer $RUSTARR_MCP_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"rustarr","arguments":{"action":"integrations"}}}'
```

## MCP Client Configuration

Streamable HTTP:

```json
{
  "mcpServers": {
    "rustarr": {
      "url": "http://127.0.0.1:3100/mcp",
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
