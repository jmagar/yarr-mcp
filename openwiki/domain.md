# Domain Concepts

Key domain concepts and terminology for understanding and working with yarr.

## Services and capabilities

### ServiceKind

`ServiceKind` is the enum of all supported upstream services:

```rust
pub enum ServiceKind {
    Radarr,
    Sonarr,
    Prowlarr,
    Overseerr,
    SABnzbd,
    Qbittorrent,
    Plex,
    Jellyfin,
    Tautulli,
    Bazarr,
    Tracearr,
}
```

Each kind has:
- A **capability class** (ArrManager, Indexer, etc.)
- An **auth style** (ApiKeyHeader, QueryApiKey, CookieSession, etc.)
- An **API prefix** (e.g., `/api/v3`)
- A **path allowlist** for generic passthrough
- A **resource noun** (e.g., `series`, `movie`) for *arr managers

### Capability

`Capability` groups service kinds by behavior:

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
    GenericOnly,     // No curated commands yet
}
```

Curated commands target a capability, not a specific kind. An `ArrManager` command works for both Sonarr and Radarr without per-kind lists.

The `KindDescriptor` table in `src/capability.rs` is the single source of truth for service topology.

## Actions

### Generic actions

Actions work for **every configured service kind**:

| Action | Scope | Description |
|--------|-------|-------------|
| `service_status` | `yarr:read` | Fetch upstream status endpoint |
| `api_get` | `yarr:write` | Proxy GET request to configured service |
| `api_post` | `yarr:write` | Proxy POST request (runs immediately) |
| `api_put` | `yarr:write` | Proxy PUT request (runs immediately) |
| `api_delete` | `yarr:write` | Proxy DELETE (destructive; runs immediately, elicited on MCP) |
| `help` | public | Return action reference |

Generic actions are defined in `ACTION_SPECS` (`src/actions/registry.rs`).

### Code Mode actions

Code Mode-specific actions:

| Action | Scope | Surface | Description |
|--------|-------|---------|-------------|
| `codemode` | `yarr:write` | MCP tool `yarr` / CLI `codemode` | Run JS arrow function over fleet |
| `op` | `yarr:write` | Inside Code Mode | Dispatch generated OpenAPI operation |
| `snippet_list` | `yarr:read` | CLI `snippet list` | List saved Code Mode snippets |
| `snippet_save` | `yarr:write` | CLI `snippet save` | Save named, reusable snippet |
| `snippet_run` | `yarr:write` | CLI `snippet run` / `codemode.run()` | Run saved snippet |
| `snippet_delete` | `yarr:write` | CLI `snippet delete` | Delete saved snippet |

### Curated commands

Curated commands are capability-targeted convenience wrappers:

- **ArrManager**: `search`, `add`, `import`, `refresh`, `unmonitor`
- **Indexer**: `test`, `sync`
- **DownloadClient**: `add`, `pause`, `resume`, `remove`
- **Stats**: `user_counts`, `top_items`

Curated commands are defined per-capability in `src/actions/commands/` and exposed through `curated_commands()`.

## Scopes

Actions require a scope for authorization:

| Scope | Meaning |
|-------|---------|
| `public` | No auth required (e.g., `help`) |
| `yarr:read` | Read-only operations (status, get, snippet_list) |
| `yarr:write` | Write operations (post, put, delete, codemode) |

Scopes are enforced by the MCP layer in `src/mcp/rmcp_server.rs`.

## Code Mode

### What it is

Code Mode is yarr's JavaScript runtime — a QuickJS sandbox that runs async arrow functions with access to the entire fleet through per-service callables.

### Interface

The `yarr` tool takes a `code` argument (a string containing a JS arrow function):

```javascript
async () => {
  const status = await radarr.get_system_status();
  return { version: status.version };
}
```

### Built-ins

Inside Code Mode, these are available:

- **Per-service callables**: `radarr`, `sonarr`, `prowlarr`, etc.
  - Generated ops: `radarr.get_movie()`, `sonarr.post_series()`
  - Generic passthrough: `radarr.api_get('/path')`, `sonarr.api_post('/path', body)`
- **Discovery**: `codemode.search('movie')`, `codemode.describe('radarr.get_movie')`
- **Snippets**: `codemode.snippets()`, `codemode.run('name', input)`
- **Artifacts**: `writeArtifact('filename.txt', 'content')`

### Discovery

`codemode.search()` uses semantic search to find relevant operations:

```javascript
async () => {
  // Find operations related to "movies"
  const movieOps = await codemode.search('movies');
  return movieOps.map(op => op.name);
}
```

`codemode.describe()` returns parameter and response type schemas:

```javascript
async () => {
  const schema = await codemode.describe('radarr.get_movie');
  return {
    params: schema.params,
    response: schema.response
  };
}
```

### Generated operations

Spec-backed services expose **all** their OpenAPI operations as callables:

- `radarr.get_system_status()`
- `radarr.get_movie({ id: 123 })`
- `radarr.post_movie({ body: {...} })`
- `sonarr.get_series()`
- `sonarr.post_series({ body: {...} })`

Operations are generated from `/specs/*.openapi.json` by `cargo xtask gen-openapi` into `src/openapi/generated/*.rs`.

### Destructive deletes

DELETE operations dispatch immediately inside Code Mode, same as any other op — there is no confirmation channel mid-script:

```bash
# Direct call
yarr radarr delete --path /api/v3/movie/123

# Inside Code Mode, DELETE dispatches the same way
async () => await radarr.delete_movie({ id: 123 })
```

On the MCP surface a destructive action gets a real interactive elicitation prompt before it dispatches.

## OpenAPI operations

### Specs

yarr vendors OpenAPI specs in `/specs/`:

- `sonarr.openapi.json` — ~235 operations, ~136 component types
- `radarr.openapi.json` — Similar coverage
- `prowlarr.openapi.json` — Indexer management operations
- `overseerr.openapi.yml` — Request management
- `plex.openapi.yml` — Media server operations
- `jellyfin.openapi.json` — Emby-based media server

### Generation

Operations are generated by `xtask/src/openapi/`:

```bash
cargo xtask gen-openapi
```

This produces `src/openapi/generated/*.rs` modules containing operation functions like:

```rust
pub async fn get_system_status(client: &Client, base_url: &str) -> Result<Value> {
    client.get_json(base_url, "/api/v3/system/status").await
}
```

### Dispatch

Generated operations are dispatched through `src/app/openapi_ops.rs` by the `op` action:

```javascript
// Inside Code Mode
async () => await radarr.get_system_status()
// ↓
// dispatched to op action with operationId="radarr.get_system_status"
// ↓
// routed to generated fn in src/openapi/generated/radarr.rs
```

## Auth styles

Each service kind authenticates differently:

| Auth style | Services | Mechanism |
|------------|----------|-----------|
| `ApiKeyHeader` | Sonarr, Radarr, Prowlarr, Overseerr, Bazarr, Tracearr | `X-Api-Key: <key>` header |
| `QueryApiKey` | SABnzbd, Tautulli | `?apikey=<key>` query param |
| `CookieSession` | qBittorrent | Login with username/password, store SID cookie |
| `PlexToken` | Plex | `?X-Plex-Token=<token>` query param |
| `JellyfinToken` | Jellyfin | `X-Emby-Token: <key>` header |

Auth is handled in `src/yarr/auth.rs`. qBittorrent's cookie session is isolated to a separate `reqwest::Client` so cookies don't leak to other services on the same host.

## Path allowlists

Generic passthrough (`api_get`, `api_post`, etc.) is restricted to configured path prefixes per service kind:

```rust
pub struct KindDescriptor {
    pub path_allowlist: &'static [&'static str],
    // e.g., Radarr: ["/api", "/api/v3"]
}
```

This prevents accidental calls to unexpected endpoints. Paths containing `apikey=`, `token=`, or `X-Plex-Token` in the query string are rejected.

## Transport model

### YarrClient

`YarrClient` (`src/yarr.rs`) is the HTTP transport layer:

- **Shared client**: One `reqwest::Client` for all services except qBittorrent
- **qBittorrent client**: Separate client with cookie jar for SID
- **Timeout**: 30s default, configurable via `YARR_HTTP_TIMEOUT_SECS`
- **No redirects**: Prevents credential leakage through malicious redirects
- **Connect timeout**: 10s bound on TCP connect phase

### YarrService

`YarrService` (`src/app.rs`) is the business logic layer:

- Validates service names
- Checks path allowlists
- Enforces destructive delete rules
- Formats responses
- Manages Code Mode artifacts and semantic cache

**Transports do NOT contain business logic.** All validation and enrichment happens in `YarrService`.

## Token budgets

MCP responses are token-budgeted to prevent runaway output:

- **Default budget**: 10,000 tokens
- **Budget per action**: Configured in `src/token_limit.rs`
- **Enforcement**: Responses exceeding budget are truncated with ellipsis

This is particularly relevant for Code Mode scripts that return large upstream responses.

## Snippets

Snippets are saved, reusable Code Mode scripts:

```bash
# Save a snippet
yarr snippet save my-script --code 'async () => ({ status: await radarr.get_system_status() })'

# List snippets
yarr snippet list

# Run a snippet
yarr snippet run my-script

# Delete a snippet
yarr snippet delete my-script
```

Snippets are stored in `{data_dir}/snippets/` as JSON files.

## Artifacts

Code Mode can write artifacts to disk:

```javascript
async () => {
  const report = generate_report();
  writeArtifact('report.json', JSON.stringify(report));
  return { artifacts: ['report.json'] };
}
```

Artifacts are written to `{data_dir}/artifacts/{run_id}/`. Disabled by default; enable with `YarrService::with_data_dir()`.

## Parity

yarr enforces **MCP ↔ CLI parity** — every action must behave identically whether called via MCP tool or CLI command. This is tested in `tests/parity.rs`.

Parity means:
- Same parameters and validation
- Same business logic and error handling
- Same output format (modulo CLI formatting)
- Elicitation gates destructive deletes on MCP only, by design — no CLI equivalent

## Further reading

- `/docs/TOOLS_ACTIONS_ENDPOINTS.md` — Complete action/tool reference
- `/docs/MCP_SCHEMA.md` — MCP tool JSON schema
- `/docs/MCPORTER.md` — Code Mode and openapi operation details
