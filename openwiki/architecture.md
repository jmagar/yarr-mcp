# Architecture

yarr is a Rust MCP and CLI server built on `rmcp`. It follows a strict layered architecture where transports stay thin and business logic stays testable.

## Layer design

```
YarrClient    (src/yarr.rs)    → HTTP/API transport ONLY — network calls, no logic
YarrService   (src/app.rs)     → ALL business logic, validation, enrichment
MCP shim      (src/mcp/tools.rs) → parse JSON args → call service → return Value
CLI shim      (src/cli.rs)     → parse argv → call service → print
```

**Golden rule:** If you're writing business logic in `mcp/tools.rs`, `cli.rs`, or `main.rs`, you're doing it wrong. Move it to `app.rs` or an `app/` submodule.

## Module layout

```
src/
  yarr.rs           ← HTTP/API transport ONLY (no business logic)
  yarr/             ← per-service auth + transport helpers
  capability.rs     ← Capability enum + KindDescriptor table (SSOT per kind)
  app.rs            ← ALL business logic, validation, transformations
  app/              ← per-capability business modules
    openapi_ops.rs  ← generated OpenAPI operation dispatch
    download.rs     ← download client operations
    stats.rs        ← stats/analytics operations
    subtitles.rs    ← subtitle operations
    trace.rs        ← trace operations
    codemode.rs     ← Code Mode service methods
  actions/          ← action registry, dispatch, parse, help, descriptors
    registry.rs     ← ACTION_SPECS + CommandDescriptor table
    dispatch.rs     ← action routing to YarrService methods
    parse.rs        ← CLI arg parsing
    help.rs         ← help text generation
    commands.rs     ← curated command groups (arr, indexer, download, etc.)
    model.rs        ← ActionSpec, CommandDescriptor, scope types
  config.rs         ← Config structs + env override resolution
  server.rs         ← AppState, AuthPolicy, build_auth_layer
  server/
    routes.rs       ← axum router: /mcp + /health + /status + OAuth
  mcp.rs            ← MCP module entry: re-exports only
  mcp/
    tools.rs        ← thin shim: parse args → call service → return Value
    schemas.rs      ← tool JSON schema (derived from action registry)
    rmcp_server.rs  ← ServerHandler impl (tools, resources, prompts)
    prompts.rs      ← MCP prompt definitions
    transport.rs    ← Streamable HTTP transport wiring
  cli.rs            ← thin shim: parse args → call service → format/print
  cli/
    command.rs      ← Command enum (incl. Curated { action, params })
    router.rs       ← resolves token1 as infra verb or ServiceKind
    doctor.rs       ← pre-flight checks: env, connectivity, config
    setup.rs        ← interactive first-run wizard
    watch.rs        ← polls /health for plugin monitors
  codemode.rs       ← Code Mode engine, QuickJS runtime
  codemode/
    engine.rs       ← QuickJS sandbox setup, runtime lifecycle
    builtins.rs     ← codemode.search/describe, api passthrough
    semantic.rs     ← semantic search catalog, embeddings
  openapi.rs        ← OpenAPI spec loader, operation registry
  openapi/
    generated.rs    ← re-exports of generated operation modules
    generated/
      *.rs          ← ~600-900 generated operation fns per service
  token_limit.rs    ← token budget enforcement for MCP responses
  models.rs         ← public typed response structs (one set per ServiceKind)
  logging.rs        ← dual stderr + file logging setup
  run_mode.rs       ← RunMode enum (ServeHttp, ServeStdio, Cli)
  lib.rs            ← pub re-exports + test helpers
  main.rs           ← mode dispatch ONLY (HTTP server / stdio / CLI)
```

## Core files by responsibility

| File | Purpose |
|------|---------|
| `src/yarr.rs` | Upstream HTTP client transport. Replace this for different APIs. |
| `src/yarr/auth.rs` | Per-service auth (ApiKeyHeader, QueryApiKey, CookieSession, tokens). |
| `src/yarr/helpers.rs` | URL building, query assembly, path validation, response slimming. |
| `src/app.rs` | Service facade — all business rules, validation, orchestration. |
| `src/app/openapi_ops.rs` | Generated OpenAPI operation dispatch (`sonarr.get_series()` etc.). |
| `src/capability.rs` | Capability enum + KindDescriptor table (SSOT for service topology). |
| `src/actions/registry.rs` | `ACTION_SPECS` + `curated_commands()` table. |
| `src/actions/dispatch.rs` | Routes action names to `YarrService` methods. |
| `src/mcp/tools.rs` | MCP tool shim — parse JSON → call service → return Value. |
| `src/mcp/schemas.rs` | Tool input schema derived from action registry. |
| `src/mcp/rmcp_server.rs` | rmcp `ServerHandler` impl, scope enforcement. |
| `src/cli.rs` | CLI shim — parse argv → call service → format/print. |
| `src/config.rs` | TOML + environment loading, defaults, validation. |
| `src/server.rs` | Axum server startup, auth policy, AppState. |
| `src/main.rs` | Mode selection (HTTP/stdio/CLI). |

## AppState

The `AppState` struct is cloned per-request by the RMCP framework:

```rust
#[derive(Clone)]
pub struct AppState {
    pub config: McpConfig,        // MCP server config (host, port, auth)
    pub auth_policy: AuthPolicy,  // LoopbackDev | TrustedGatewayUnscoped | Mounted
    pub service: YarrService,      // Service layer — everything routes through here
}
```

Keep `AppState` cheap to clone — `YarrService` wraps an `Arc`-backed `reqwest::Client`.

## Surfaces

yarr exposes exactly two surfaces:

| Surface | Purpose | Entry point |
|---------|---------|-------------|
| **MCP** | Primary integration surface for agents | `src/mcp/tools.rs` |
| **CLI** | Scripting, debugging, regression tests | `src/cli.rs` |

No REST API and no Web UI — as an upstream-client server, yarr does not duplicate the upstream HTTP APIs as a local REST endpoint. Add REST/Web only when the server owns meaningful workflows or state not present upstream.

## Capability model

The `Capability` enum groups `ServiceKind` by behavior:

```rust
pub enum Capability {
    ArrManager,      // Sonarr, Radarr — /api/v3 resource managers
    Indexer,         // Prowlarr
    DownloadClient,  // SABnzbd, qBittorrent
    MediaServer,     // Plex, Jellyfin
    Requests,        // Overseerr
    Stats,           // Tautulli
    Subtitles,       // Bazarr
    Trace,           // Tracearr
    GenericOnly,     // Kinds with no curated commands yet
}
```

Curated commands target a `Capability`, not a specific kind. An `ArrManager` command works for both Sonarr and Radarr without per-kind lists.

See `src/capability.rs` for the `KindDescriptor` table — the single source of truth for API versioning, auth style, path allowlists, and resource nouns.

## Code Mode

Code Mode is yarr's JavaScript runtime:

- **Engine**: QuickJS (in-process via `rquickjs`) — no subprocess/wasmtime overhead
- **Interface**: The `yarr` tool takes an async arrow function string (`code`)
- **Builtins**: Per-service callables, `codemode.search()`, `codemode.describe()`, `api.*`
- **Discovery**: Runtime operation/type discovery via semantic search
- **Artifacts**: Optional `writeArtifact()` output to `{data_dir}/artifacts/`

Code Mode scripts run inside the rmcp `tools/call` handler. The generated OpenAPI operations are dispatched through the `op` action, which validates the operation ID, deserializes params, and calls the upstream API.

## OpenAPI integration

yarr vendored OpenAPI specs for 6 services in `/specs/`:

- `sonarr.openapi.json` → `src/openapi/generated/sonarr.rs` (~235 operations)
- `radarr.openapi.json` → `src/openapi/generated/radarr.rs`
- `prowlarr.openapi.json` → `src/openapi/generated/prowlarr.rs`
- `overseerr.openapi.yml` → `src/openapi/generated/overseerr.rs`
- `plex.openapi.yml` → `src/openapi/generated/plex.rs`
- `jellyfin.openapi.json` → `src/openapi/generated/jellyfin.rs`

Regenerate with `cargo xtask gen-openapi`. The generated modules export operation fns that `src/app/openapi_ops.rs` routes to.

## Action dispatch

Actions flow through these layers:

1. **Registry** (`src/actions/registry.rs`): `ACTION_SPECS` defines generic actions; `curated_commands()` defines per-capability commands
2. **Parse** (`src/actions/parse.rs`): CLI arg parsing into structured params
3. **Dispatch** (`src/actions/dispatch.rs`): Routes action name → `YarrService` method
4. **Service** (`src/app.rs`): Business logic, validation, upstream calls
5. **Transport** (`src/yarr.rs`): HTTP client with auth, headers, timeouts

The MCP and CLI shims are thin — they parse args, call `dispatch`, and format results.

## Auth policy

The `AuthPolicy` enum determines how the MCP HTTP server enforces auth:

| Policy | When it applies | Enforcement |
|--------|-----------------|-------------|
| `LoopbackDev` | `host = "127.0.0.1"` | No auth — OS process boundary is the trust boundary |
| `TrustedGatewayUnscoped` | `host != "127.0.0.1"` and `no_auth = false` | Bearer token or OAuth required |
| `Mounted` | When `YARR_NO_AUTH=true` is set on non-loopback | Trust upstream gateway (reverse proxy handles auth) |

Bearer mode is static (`YARR_MCP_TOKEN`). OAuth mode uses Google OAuth 2.0 + JWT (`lab-auth` crate) — see `/docs/AUTH.md`.

## Error handling

- **Upstream errors**: `UpstreamError` enum in `src/yarr.rs` (HTTP status, non-JSON, login rejected)
- **Config errors**: `ConfigError` in `src/config.rs` (missing vars, invalid URLs)
- **Service errors**: `anyhow::Result` throughout `src/app.rs`

Errors are rendered as JSON error responses in MCP and formatted messages in CLI.

## Testing strategy

- **Unit tests**: `_tests.rs` files alongside source (e.g., `config_tests.rs`, `actions_tests.rs`)
- **Integration tests**: `tests/` directory with full-stack scenarios
- **Parity tests**: `tests/parity.rs` enforces MCP ↔ CLI behavioral parity
- **Live contracts**: `xtask/src/live/` calls real upstream services; generates fixture contracts
- **Pattern checks**: `cargo xtask patterns` enforces architecture rules (file sizes, thin shims, etc.)

See [Testing](testing.md) for details.

## xtask automation

`xtask/` contains repo automation commands:

- `cargo xtask ci` — Run full CI pipeline (format, clippy, tests, patterns, audit)
- `cargo xtask dist` — Build release binary and copy to `bin/` for Git LFS distribution
- `cargo xtask gen-openapi` — Regenerate OpenAPI operation modules from specs
- `cargo xtask check-env` — Validate environment variables before starting server
- `cargo xtask patterns` — Check static contracts from PATTERNS.md
- `cargo xtask symlink-docs` — Create AGENTS.md/GEMINI.md symlinks

These are available via `just` aliases too — see `/Justfile` or `/docs/JUSTFILE.md`.

## Further reading

- `/docs/ARCHITECTURE.md` — Original architecture documentation (more detailed on some topics)
- `/docs/PATTERNS.md` — Patterns shared across the Rust MCP server family
- `/docs/TOOLS_ACTIONS_ENDPOINTS.md` — Detailed action/tool/endpoint reference
- `/docs/PHILOSOPHY.md` — Design philosophy and constraints
