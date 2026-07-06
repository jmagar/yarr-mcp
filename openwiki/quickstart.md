# yarr Quickstart

**yarr** is a Rust MCP and CLI server for media automation fleets. It provides a unified tool surface for managing Sonarr, Radarr, Prowlarr, Tautulli, Overseerr, Bazarr, Tracearr, SABnzbd, qBittorrent, Plex, and Jellyfin through a single `yarr` MCP tool and an equivalent CLI.

## What yarr does

yarr is an **upstream-client MCP server** — it does not replace media management applications or mirror their REST APIs as a web UI. Instead, it provides:

- **One consistent MCP tool** (`yarr`) that runs JavaScript (Code Mode) scripts across your entire media fleet
- **CLI parity** for scripting, debugging, and automation
- **Generated OpenAPI operations** for spec-backed services (Sonarr, Radarr, Prowlarr, Overseerr, Jellyfin, Plex)
- **Curated commands** for download clients and stats services
- **Generic passthrough** (`api_get`, `api_post`, `api_put`, `api_delete`) for any endpoint

The 6 spec-backed services expose **~235+ generated operations each** (e.g., `sonarr.get_series()`, `radarr.post_movie({...})`), all discovered at runtime through `codemode.search()` and `codemode.describe()`.

## Installation

### Recommended: Node launcher (includes binary auto-download)

```bash
# Run stdio MCP server without permanent install
npx -y yarr-mcp mcp

# Or install globally
npm i -g yarr-mcp
yarr --version
yarr mcp
```

### Alternative: One-line installer (no npm required)

```bash
curl -fsSL https://raw.githubusercontent.com/jmagar/yarr/main/scripts/install.sh | bash
```

This installs `yarr` to `~/.local/bin`.

### Development: From source

```bash
cargo build --release
cargo run --   # for development
```

## Configuration

yarr uses a split config approach:

- **config.toml** → ports, bind addresses, feature flags, timeouts, rate limits (safe to commit)
- **.env** → API URLs, API keys, tokens, OAuth credentials (NEVER commit)

```bash
# Copy templates
cp config.example.toml config.toml
cp .env.example .env

# Edit .env with your service URLs and credentials
export YARR_SERVICES=radarr,sonarr
export YARR_RADARR_URL=http://127.0.0.1:7878
export YARR_RADARR_API_KEY=your-api-key
export YARR_SONARR_URL=http://127.0.0.1:8989
export YARR_SONARR_API_KEY=your-api-key

# Optional: disable auth for local development
export YARR_MCP_HOST=127.0.0.1
export YARR_MCP_PORT=40070
export YARR_MCP_NO_AUTH=true
```

See [Configuration](configuration.md) for complete options.

## Quick test

### CLI

```bash
yarr help
yarr radarr status
yarr sonarr get --path /api/v3/system/status
```

### HTTP MCP server

```bash
# Start the server
yarr serve

# In another terminal, call the tool
curl -s http://127.0.0.1:40070/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"yarr","arguments":{"code":"async () => ({ radarr: await radarr.service_status(), sonarr: await sonarr.service_status() })"}}}'
```

### stdio MCP (local agents only)

```bash
yarr mcp
```

## Code Mode basics

The `yarr` tool takes a JavaScript async arrow function and runs it in an in-process QuickJS sandbox. Inside the script, access your entire fleet through per-service callables:

```javascript
async () => {
  // Generated OpenAPI operations (spec-backed services)
  const radarrStatus = await radarr.get_system_status();
  const movies = await radarr.get_movie();
  const series = await sonarr.get_series();
  
  // Generic passthrough (any service)
  const health = await radarr.api_get('/health');
  
  return {
    radarrVersion: radarrStatus.version,
    movieCount: movies.length,
    seriesCount: series.length
  };
}
```

Discover available operations and types at runtime:

```javascript
async () => {
  // Search for callables by semantic relevance
  const movieOps = await codemode.search('movie');
  
  // Describe a specific operation's parameters and response type
  const schema = await codemode.describe('radarr.get_movie');
  
  return { movieOps, schema };
}
```

## Running checks

From source:

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build --release
```

Via just(1):

```bash
just ci      # Run full CI pipeline locally
just check   # Quick format + clippy check
```

## Next steps

- [Architecture](architecture.md) — Layer design, module layout, AppState
- [Configuration](configuration.md) — All config options, auth modes, service setup
- [Operations](operations.md) — Deployment, Docker, systemd, observability
- [Testing](testing.md) — Test strategy, parity tests, live contracts
- [Domain concepts](domain.md) — Capabilities, Code Mode, generated operations
- [Integrations](integrations.md) — OpenAPI specs, xtask commands

For detailed patterns and conventions used across the Rust MCP server family, see the existing docs in `/docs/`, particularly [ARCHITECTURE.md](../docs/ARCHITECTURE.md) and [PATTERNS.md](../docs/PATTERNS.md).
