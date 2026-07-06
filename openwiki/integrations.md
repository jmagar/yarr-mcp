# Integrations

Integration points for extending yarr and connecting it with other tools.

## OpenAPI specs

yarr vendors OpenAPI specs for 6 spec-backed services in `/specs/`:

| Spec | Service | Operations | Source |
|------|---------|------------|--------|
| `sonarr.openapi.json` | Sonarr | ~235 ops | Sonarr Swagger |
| `radarr.openapi.json` | Radarr | ~235 ops | Radarr Swagger |
| `prowlarr.openapi.json` | Prowlarr | ~150 ops | Prowlarr Swagger |
| `overseerr.openapi.yml` | Overseerr | ~100 ops | Overseerr OpenAPI |
| `plex.openapi.yml` | Plex | ~80 ops | Plex Web API |
| `jellyfin.openapi.json` | Jellyfin | ~150 ops | Jellyfin OpenAPI |

### Regenerating operations

When upstream specs change:

```bash
cargo xtask gen-openapi
```

This regenerates `src/openapi/generated/*.rs` modules. Commit the generated files.

### Adding a new spec-backed service

1. Vendored the spec to `/specs/<service>.openapi.json`
2. Add `ServiceKind` variant to `src/config.rs`
3. Add `KindDescriptor` entry to `src/capability.rs`
4. Add generator case in `xtask/src/openapi/`
5. Run `cargo xtask gen-openapi`
6. Wire operations in `src/app/openapi_ops.rs`

## xtask commands

`xtask/` contains repo automation commands invoked via `cargo xtask <command>`:

| Command | Purpose | Usage |
|---------|---------|-------|
| `ci` | Run full CI pipeline | `cargo xtask ci` |
| `dist` | Build release binary to `bin/` | `cargo xtask dist` |
| `gen-openapi` | Regenerate OpenAPI operations | `cargo xtask gen-openapi` |
| `check-env` | Validate environment variables | `cargo xtask check-env` |
| `patterns` | Check architecture rules | `cargo xtask patterns` |
| `symlink-docs` | Create AGENTS.md/GEMINI.md symlinks | `cargo xtask symlink-docs` |
| `live-contracts` | Generate upstream contracts | `cargo xtask live-contracts` |

See `/xtask/README.md` for full documentation.

### just(1) aliases

The `/Justfile` provides convenient aliases:

```bash
just ci              # cargo xtask ci
just dist            # cargo xtask dist
just check           # cargo fmt + clippy
just patterns-check  # cargo xtask patterns
just check-env       # cargo xtask check-env
```

## Plugin system

yarr supports plugins that extend CLI commands and Code Mode callables. (Note: This is an emerging feature — check current implementation status in `/docs/PLUGINS.md`.)

### Plugin manifest

```toml
[plugin]
name = "my-plugin"
version = "0.1.0"
description = "My custom yarr plugin"

[[hooks]]
name = "custom_command"
command = "my-custom"
action = "custom_action"
```

### Plugin hooks

Plugins can hook into:
- **Custom commands**: New CLI verbs
- **Code Mode callables**: New per-service functions
- **Auth providers**: Custom authentication methods
- **Middleware**: Request/response processing

See `/docs/PLUGINS.md` for the current plugin API surface.

## MCP integration

### stdio mode

For local child-process MCP clients:

```bash
yarr mcp
```

stdio mode bypasses HTTP auth — the OS process boundary is the trust boundary.

### HTTP mode

For networked MCP clients:

```bash
yarr serve
```

The MCP server exposes the `yarr` tool at `/mcp`. Configure clients with:

```json
{
  "url": "http://your-server:40070/mcp",
  "transport": "http",
  "auth": {
    "type": "bearer",
    "token": "your-token"
  }
}
```

### Tool schema

The `yarr` tool takes one argument, `code`:

```json
{
  "name": "yarr",
  "inputSchema": {
    "type": "object",
    "properties": {
      "code": {
        "type": "string",
        "description": "JavaScript async arrow function to run"
      }
    },
    "required": ["code"]
  }
}
```

See `/docs/MCP_SCHEMA.md` for the full tool schema.

## Claude desktop integration

### MCP server configuration

Add to Claude Desktop MCP settings:

```json
{
  "mcpServers": {
    "yarr": {
      "command": "yarr",
      "args": ["mcp"],
      "env": {
        "YARR_SERVICES": "radarr,sonarr",
        "YARR_RADARR_URL": "http://127.0.0.1:7878",
        "YARR_RADARR_API_KEY": "...",
        "YARR_SONARR_URL": "http://127.0.0.1:8989",
        "YARR_SONARR_API_KEY": "..."
      }
    }
  }
}
```

### Using Code Mode

In Claude:

```
Use the yarr tool to search for movies in Radarr and add them to my wishlist.
```

Claude will call the `yarr` tool with generated Code Mode script:

```javascript
async () => {
  const movies = await radarr.get_movie();
  const wishlist = movies.filter(m => m.monitored);
  return { movies: wishlist };
}
```

## Reverse proxy

### Nginx

```nginx
location /yarr {
    proxy_pass http://127.0.0.1:40070/mcp;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    
    # Auth
    auth_request /auth-validator;
    auth_request_set $user $upstream_http_x_user;
    
    # Headers for RMCP
    proxy_set_header Connection "";
    proxy_http_version 1.1;
}
```

Set `YARR_NOAUTH=true` so yarr trusts the reverse proxy for auth.

### Traefik

```yaml
http:
  routers:
    yarr:
      rule: "PathPrefix(`/yarr`)"
      service: yarr
      middlewares:
        - auth-headers
  services:
    yarr:
      loadBalancer:
        servers:
          - url: "http://127.0.0.1:40070"
  middlewares:
    auth-headers:
      headers:
        customRequestHeaders:
          X-User: "{{ .User }}"
```

## Observability

### Metrics

The `/metrics` endpoint exposes Prometheus metrics:

```bash
curl http://127.0.0.1:40070/metrics
```

Metrics include:
- `mcp_requests_total{action}` — Tool calls by action
- `mcp_request_duration_seconds` — Request latency
- `upstream_requests_total{service}` — Upstream calls
- `upstream_request_errors_total{service}` — Upstream errors

### Structured logging

Logs are emitted as JSON lines to `{data_dir}/logs/yarr.log`:

```json
{"timestamp":"2024-01-01T00:00:00Z","level":"INFO","message":"Starting server","port":40070}
```

Parse with jq, elk, or your log aggregator.

See `/docs/OBSERVABILITY.md` for details.

## CI/CD integration

### GitHub Actions

```yaml
- name: Run yarr tests
  run: cargo xtask ci

- name: Build release
  run: cargo xtask dist

- name: Upload to GitHub Releases
  uses: softprops/action-gh-release@v1
  with:
    files: bin/yarr
```

### Docker

```dockerfile
FROM rust:1.90 as builder
WORKDIR /app
COPY . .
RUN cargo xtask dist

FROM alpine:latest
COPY --from=builder /app/bin/yarr /usr/local/bin/
ENTRYPOINT ["yarr", "serve"]
```

## External tooling

### just(1)

Install just for convenient commands:

```bash
cargo install just
just --help
```

### cargo-nextest

Install nextest for faster test runs:

```bash
cargo install nextest
cargo nextest run
```

### taplo

Install taplo for TOML formatting:

```bash
cargo install taplo-cli
taplo format
```

## Automation scripts

### Pre-commit hook

The repo uses lefthook for pre-commit hooks (see `/lefthook.yml`):

```yaml
pre-commit:
  commands:
    format:
      run: cargo fmt
    clippy:
      run: cargo clippy -- -D warnings
```

### Release automation

Releases are automated via `release-please` (see `/release-please-config.json`):

- Parses conventional commits
- Generates CHANGELOG.md
- Creates GitHub releases
- Bumps version numbers

Run release with:

```bash
just release
```

## Secrets management

### .env with direnv

```bash
# Install direnv
cargo install direnv

# Add to .envrc
export YARR_MCP_TOKEN=$(pass yarr/token)
export YARR_RADARR_API_KEY=$(pass radarr/key)

# Allow
direnv allow
```

### Vault integration

For production, use a secrets manager:

```bash
export YARR_MCP_TOKEN=$(vault kv get -field=token yarr/mcp)
export YARR_RADARR_API_KEY=$(vault kv get -field=api_key radarr)
```

## Further reading

- `/docs/INTEGRATIONS.md` — Integration examples and patterns
- `/docs/PLUGINS.md` — Plugin system documentation
- `/docs/MCP-REGISTRY-PUBLISH-GUIDE.md` — Publishing to MCP registry
- `/docs/OBSERVABILITY.md` — Metrics, logging, tracing
- `/docs/CI.md` — CI/CD setup
- `/docs/SCRIPTS.md` — Useful automation scripts
