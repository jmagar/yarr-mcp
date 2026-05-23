# rmcp-template

A reusable Rust template for building MCP servers using the [rmcp](https://crates.io/crates/rmcp) crate. Clone this, rename a handful of identifiers, drop in your API client, and you have a working MCP server with both stdio and Streamable HTTP transports, bearer token or Google OAuth authentication, elicitation support, resources, and prompts.

## Rust Server Family

This template is the reference point for the local Rust MCP/server family:

| Local path | GitHub repo | Binary |
| --- | --- | --- |
| `../lab` | `jmagar/lab` | `labby` |
| `../axon_rust` | `jmagar/axon` | `axon` |
| `../syslog-mcp` | `jmagar/syslog-mcp` | `syslog` |
| `../rustify` | `jmagar/rustify` | `gotify` |
| `../rustifi` | `jmagar/rustifi` | `unifi` |
| `../apprise-mcp` | `jmagar/apprise-mcp` | `apprise` |
| `../rustscale` | `jmagar/rustscale` | `tailscale` |
| `../rmcp-template` | `jmagar/rmcp-template` | `example` |
| `../unrust` | `jmagar/unrust` | `unraid` |

## Plugin Surfaces

The template ships Claude Code, Codex, and Gemini plugin surfaces from one shared `plugins/example/` package. See [docs/PLUGINS.md](docs/PLUGINS.md) for the manifest layout, shared MCP config, skills, hook setup contract, and per-host adaptation checklist.

## Server surface policy

Every scaffolded business action must have **MCP + CLI** parity. MCP is the agent-facing surface; CLI is the scripting/debugging/test surface.

REST API and Web UI are required only for servers that are more than a thin client over another service API:

| Server category | Required surfaces | Examples |
|---|---|---|
| Upstream-client MCP server | MCP + CLI | `unrust`, `rustifi`, `rustify`, `rustscale`, `apprise` |
| Application/platform server | API + CLI + MCP + Web | `axon`, `lab`, `syslog` |

For upstream-client servers, do not mirror the upstream HTTP API locally by default. Add REST/Web only when the server owns meaningful state, workflows, dashboards, or non-MCP consumers.

`scaffold_intent` is the template's explicit MCP-only exception: it combines MCP elicitation with plugin skill handoff, so there is no true CLI equivalent inside the user's agent/editor permission model.

## What this template gives you

- **Layered architecture** — transport client → service → MCP/CLI shims, enforced by convention
- **Action-based dispatch** — one MCP tool with an `action` parameter routes to any number of operations
- **Both transports** — `example serve` (Streamable HTTP) and `example mcp` (stdio)
- **Both auth modes** — static bearer token or full Google OAuth with RS256 JWT issuance
- **MCP elicitation** — server-asks-user mid-call (spec 2025-06-18), with graceful fallback
- **MCP resources** — exposes the tool schema as a readable resource
- **MCP prompts** — pre-canned `quick_start` prompt for clients that support them
- **CLI** — same service layer, human-readable output, mandatory MCP parity
- **Test helpers** — `loopback_state()` and `bearer_state()` for tests without real credentials

## Architecture

```
ExampleClient  (src/example.rs)    ← HTTP/GraphQL/gRPC calls to upstream
      ↓
ExampleService (src/app.rs)        ← all business logic lives here
      ↓
  ┌──────────────────────────────────┐
  │  MCP shim (src/mcp/tools.rs)    │  parse JSON args → call service → return Value
  │  CLI shim (src/cli.rs)          │  parse CLI args  → call service → print
  └──────────────────────────────────┘
```

The rule: **zero business logic in `tools.rs` or `cli.rs`**. Both are pure shims. All logic belongs in `app.rs` (or `example.rs` for transport concerns). For business actions, MCP + CLI parity is mandatory; REST/Web are project-type dependent.

## Quickstart — run the stub

```bash
git clone https://github.com/jmagar/rmcp-template
cd rmcp-template
cargo run -- serve          # Streamable HTTP on :40060
# or
cargo run -- mcp            # stdio transport
# or
cargo run -- greet --name Alice
```

Health check:

```bash
curl http://localhost:40060/health
# {"status":"ok"}
```

Call the MCP tool directly:

```bash
curl -s -X POST http://localhost:40060/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"example","arguments":{"action":"greet","name":"Alice"}}}'
```

## Step-by-step: build your own MCP server from this template

### 1. Clone and rename

```bash
git clone https://github.com/jmagar/rmcp-template myservice-mcp
cd myservice-mcp
```

Find and replace these identifiers across the project:

| Find | Replace with |
|------|-------------|
| `rmcp-template` | `myservice-mcp` (Cargo.toml package name) |
| `example` (binary name) | `myservice` (Cargo.toml `[[bin]] name`) |
| `ExampleClient` | `MyServiceClient` |
| `ExampleService` | `MyServiceService` |
| `ExampleConfig` | `MyServiceConfig` |
| `ExampleRmcpServer` | `MyServiceRmcpServer` |
| `EXAMPLE_API_URL` | `MYSERVICE_API_URL` |
| `EXAMPLE_MCP_*` | `MYSERVICE_MCP_*` |
| `example:read` | `myservice:read` |
| `example://schema/mcp-tool` | `myservice://schema/mcp-tool` |

### 2. Replace ExampleClient with your API client

Edit `src/example.rs`. This is the only file that makes network calls.

```rust
pub struct MyServiceClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl MyServiceClient {
    pub fn new(cfg: &MyServiceConfig) -> Result<Self> {
        if cfg.api_url.is_empty() { anyhow::bail!("MYSERVICE_API_URL is not set"); }
        let client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self { client, base_url: cfg.api_url.clone(), api_key: cfg.api_key.clone() })
    }

    pub async fn get_things(&self) -> Result<Value> {
        let resp = self.client
            .get(format!("{}/things", self.base_url))
            .bearer_auth(&self.api_key)
            .send().await?
            .json::<Value>().await?;
        Ok(resp)
    }
}
```

### 3. Add service methods

Edit `src/app.rs`. Delegate to the client; add caching, retries, or transformation here:

```rust
pub async fn get_things(&self) -> Result<Value> {
    self.client.get_things().await
}
```

### 4. Add tool actions

For each new action:

**a. `src/actions.rs`** — add one entry to `ACTION_SPECS`:

```rust
ActionSpec {
    name: "get_things",
    required_scope: Some(READ_SCOPE),
    transport: ActionTransport::Any,
}
```

Then add any new parameters to `tool_definitions()` in `src/mcp/schemas.rs`.

**b. `src/mcp/tools.rs`** — add a match arm in `dispatch_example()`:

```rust
"get_things" => state.service.get_things().await,
```

Scope rules are derived from `ACTION_SPECS`.

**c. `src/cli.rs`** — add a `Command` variant and dispatch arm:

```rust
pub enum Command { ..., GetThings }

// in parse_args():
"get-things" => Some(Command::GetThings),

// in run():
Command::GetThings => service.get_things().await?,
```

**d. Add a test** in `tests/tool_dispatch.rs`.

### 5. Update config

Edit `src/config.rs` to rename `ExampleConfig` fields and env var names. Edit `config.toml` and `.env.example`.

## Command modes

```
example [serve]          Start Streamable HTTP MCP server (default)
example mcp              Start stdio MCP transport
example greet [--name]   CLI: greet
example echo --message   CLI: echo
example status           CLI: server status
example --help           Usage
example --version        Version
```

## MCP tool actions

The single `example` tool dispatches on the `action` parameter:

| Action | Description | Parameters |
|--------|-------------|------------|
| `greet` | Return a greeting | `name` (optional string) |
| `echo` | Echo a message back | `message` (required string) |
| `status` | Server status info | none |
| `elicit_name` | Ask user for name via elicitation, return greeting | none |
| `scaffold_intent` | Elicit scaffold requirements and return JSON for the `scaffold-project` skill | none |
| `help` | Full action reference | none |

## Authentication

### Bearer token (default)

Set `EXAMPLE_MCP_TOKEN`. All `/mcp` requests must include `Authorization: Bearer <token>`.

### No auth (loopback only)

Set `EXAMPLE_MCP_NO_AUTH=true` or bind to `127.*`. Only legal for local development.

### OAuth (Google)

Set `EXAMPLE_MCP_AUTH_MODE=oauth` and the OAuth env vars below. The server issues RS256 JWTs after Google authentication. OAuth and bearer can coexist (OAuth mode disables the static token by default; set `disable_static_token_with_oauth = false` to keep both active).

`/health` is always unauthenticated.

## Environment variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `EXAMPLE_API_URL` | no | — | Upstream service base URL |
| `EXAMPLE_API_KEY` | no | — | Upstream service API key |
| `EXAMPLE_MCP_HOST` | no | `0.0.0.0` | Bind host |
| `EXAMPLE_MCP_PORT` | no | `40060` | Bind port |
| `EXAMPLE_MCP_NO_AUTH` | no | `false` | Disable auth (loopback only; 1/true/yes) |
| `EXAMPLE_MCP_TOKEN` | no | — | Static bearer token for `/mcp` |
| `EXAMPLE_MCP_ALLOWED_HOSTS` | no | — | Extra comma-separated Host header values |
| `EXAMPLE_MCP_ALLOWED_ORIGINS` | no | — | Extra comma-separated CORS origins |
| `EXAMPLE_MCP_PUBLIC_URL` | OAuth | — | Public URL (e.g. `https://myservice.example.com`) |
| `EXAMPLE_MCP_AUTH_MODE` | no | `bearer` | `bearer` or `oauth` |
| `EXAMPLE_MCP_GOOGLE_CLIENT_ID` | OAuth | — | Google OAuth client ID |
| `EXAMPLE_MCP_GOOGLE_CLIENT_SECRET` | OAuth | — | Google OAuth client secret |
| `EXAMPLE_MCP_AUTH_ADMIN_EMAIL` | OAuth | — | Admin email address |
| `RUST_LOG` | no | `info` | Log filter (e.g. `info,rmcp=warn`) |

## Development commands

```bash
cargo build           # debug build
cargo build --release # release build
cargo test            # run tests
cargo clippy -- -D warnings  # lint
cargo fmt             # format

just dev              # EXAMPLE_MCP_HOST=127.0.0.1 EXAMPLE_MCP_NO_AUTH=true cargo run -- serve mcp (loopback only, no auth)
just test             # cargo test
just lint             # cargo clippy -- -D warnings
just fmt              # cargo fmt
just build            # cargo build
just release          # cargo build --release
just gen-token        # openssl rand -hex 32
just health           # curl http://localhost:40060/health | jq .
```

## Portable automation

This template includes reusable automation pulled from the local Rust server
family and generalized for new MCP services:

| Command | Purpose |
|---|---|
| `just install-tools` / `just bootstrap` | Install common local tools (`cargo-nextest`, `taplo`, `cargo-deny`, `bacon`, `cargo-llvm-cov`, `lefthook`, `cargo-audit`) |
| `just install-hooks` | Enable the fast lefthook pre-commit checks |
| `just deps-check` | Report lockfile-compatible and latest direct dependency updates |
| `just blob-size-check` | Block oversized changed blobs before they land in git |
| `just file-size-check` | Check staged source files against line-count budgets |
| `just ascii-check` / `just ascii-fix` | Find or rewrite unexpected non-ASCII characters in tracked source/config/docs |
| `just test-cov` | Generate an HTML Rust coverage report with `cargo llvm-cov` |
| `just watch` | Run interactive Rust checks with `bacon` |
| `just validate-plugin` | Validate plugin manifests, shared MCP config, hook config, and skills |
| `just runtime-current` | Detect whether Docker/systemd is running the current built artifact |
| `just schema-docs` / `just schema-docs-check` | Generate or verify [docs/MCP_SCHEMA.md](docs/MCP_SCHEMA.md) from the MCP action schema |
| `just openapi` / `just openapi-check` | Generate or verify [docs/generated/openapi.json](docs/generated/openapi.json) for the REST API surface |
| `just scaffold-contract-check` | Validate scaffold intent JSON Schema and examples in `docs/contracts/` |
| `just template-check` | Run plugin layout, schema drift, OpenAPI, scaffold contract, and template feature checks |
| `just auth-smoke` | Smoke-test bearer-token MCP HTTP auth against a running server |
| `just pre-release` | Run the release-readiness gate |
| `just up` / `just down` | Short aliases for Docker Compose start/stop |

See [scripts/README.md](scripts/README.md) for script-level options and
template adaptation notes.

## Documentation map

When changing template automation or generated surfaces, update the matching
docs in the same change:

| Surface | Documentation |
|---|---|
| Just recipes and portable commands | This README's portable automation table |
| Script options and environment variables | [scripts/README.md](scripts/README.md) |
| MCP actions, scopes, and schema resource | [docs/MCP_SCHEMA.md](docs/MCP_SCHEMA.md), generated by `just schema-docs` |
| REST OpenAPI schema | [docs/generated/openapi.json](docs/generated/openapi.json), generated by `just openapi` |
| Claude/Codex/Gemini plugin manifests, skills, and hook contract | [docs/PLUGINS.md](docs/PLUGINS.md) |
| Scaffold setup wizard handoff | [docs/specs/scaffold-intent-handoff.md](docs/specs/scaffold-intent-handoff.md) and [docs/contracts/scaffold-intent.schema.json](docs/contracts/scaffold-intent.schema.json) |
| Test layers and template checks | [tests/README.md](tests/README.md) |
| MCP registry publishing | [docs/MCP-REGISTRY-PUBLISH-GUIDE.md](docs/MCP-REGISTRY-PUBLISH-GUIDE.md) |

`just template-check` and CI enforce the highest-risk drift points: plugin
layout, schema docs, shell template smoke tests, and coupled file changes.

## MCP client configuration

### Streamable HTTP (Claude.app, mcpx, etc.)

```json
{
  "mcpServers": {
    "example": {
      "url": "http://localhost:40060/mcp",
      "headers": { "Authorization": "Bearer YOUR_TOKEN" }
    }
  }
}
```

### stdio (Claude Desktop, local clients)

```json
{
  "mcpServers": {
    "example": {
      "command": "/path/to/example",
      "args": ["mcp"],
      "env": { "RUST_LOG": "warn" }
    }
  }
}
```

## Using this template

This checklist covers everything you need to adapt rmcp-template for a real service. Work through it top-to-bottom; each step is independent.

### Checklist

#### Core: rename and implement

1. **Replace all occurrences of `example`/`Example`/`EXAMPLE` with your service name**

   Global search-replace across the entire project:

   | Find | Replace with |
   |------|-------------|
   | `rmcp-template` | `myservice-mcp` (Cargo.toml package name) |
   | `example` (binary name) | `myservice` (Cargo.toml `[[bin]] name`) |
   | `ExampleClient` | `MyServiceClient` |
   | `ExampleService` | `MyServiceService` |
   | `ExampleConfig` | `MyServiceConfig` |
   | `ExampleRmcpServer` | `MyServiceRmcpServer` |
   | `EXAMPLE_API_URL` | `MYSERVICE_API_URL` |
   | `EXAMPLE_MCP_*` | `MYSERVICE_MCP_*` |
   | `EXAMPLE_NOAUTH` | `MYSERVICE_NOAUTH` |
   | `example:read` | `myservice:read` |
   | `example://schema/mcp-tool` | `myservice://schema/mcp-tool` |
   | `.example` (data dir) | `.myservice` (in `config.rs` and `docker-compose.yml`) |

2. **Implement your API client in `src/example.rs`**

   Replace the stub methods with real HTTP/GraphQL/gRPC calls. See the inline comments for the `reqwest::Client` pattern.

3. **Add service methods to `src/app.rs`**

   Each public method on `ExampleService` corresponds to one MCP action. Business logic, caching, and retries go here — not in `tools.rs`.

4. **Add MCP actions to `src/actions.rs`, `src/mcp/tools.rs`, and `src/mcp/schemas.rs`**

   - `actions.rs`: add action metadata to `ACTION_SPECS`
   - `schemas.rs`: add any new action parameters to the schema
   - `tools.rs`: add match arms in `dispatch_example()`

5. **Add CLI commands to `src/cli.rs`**

   One `Command` enum variant and one `fmt_*` formatter per action. Keep CLI output human-readable; the MCP layer handles machine-readable JSON.

6. **Update `src/config.rs`** with service-specific config fields

   Rename `ExampleConfig` and add any fields your service needs. Update env prefixes throughout.

7. **Add required env vars to `check-env` in `xtask/src/main.rs`**

   Uncomment the `REQUIRED_VARS` entries (or add your own) so `cargo xtask check-env` catches missing credentials.

#### Docker and deployment

8. **Update `config/Dockerfile` binary name, port, and cache IDs**

   Replace every occurrence of `example` (binary copy, cache IDs, CMD, LABEL) with your binary name. Update `EXPOSE` to your port.

9. **Update `docker-compose.yml`**

   - Change `40060` to your service's port (must match `config.toml [mcp] port`)
   - The `${HOME}/.example:/data` volume is already set; rename `.example` to your service

10. **Update `entrypoint.sh`**

    Uncomment the `REQUIRED_VARS` check block and add your service's required env vars. Replace `EXAMPLE_API_KEY` references with your prefix.

11. **Update `config/Dockerfile` to use `entrypoint.sh`**

    Already wired in the template (ENTRYPOINT + CMD split). Verify the gosu/su-exec choice matches your base image.

#### Infrastructure

12. **Choose a binary distribution path**

    GitHub release tags build Linux artifacts and attach them to the release. Local `just dist` is an operator convenience for preparing files under `dist/`; it does not push generated binaries back to `main`.

13. **Run `just symlink-docs`** after any new CLAUDE.md

    Creates `AGENTS.md` + `GEMINI.md` symlinks next to every `CLAUDE.md` in the repo.

14. **Update GitHub workflow files** (`.github/workflows/`)

    In all three workflows, replace:
    - `rmcp-template` → your repo name (cache keys)
    - `example-mcp` → your Docker image name
    - `example` → your binary name
    - `jmagar` → your GitHub org/username (image registry path)

15. **Update `.env.example`** with your service's actual variable names and descriptions

16. **Update `config.example.toml`** with your service's actual config fields

#### Plugin and skills

17. **Update plugin.json userConfig for your service's credentials**

    Edit `plugins/example/.claude-plugin/plugin.json`. Replace the `example_api_url` / `example_api_key` fields with your service's actual credential names and descriptions.

18. **Update `plugins/example/hooks/plugin-setup.sh`**

    Replace `EXAMPLE_*` env var names, `example-mcp` service references, and add any service-specific credentials your binary needs.

19. **Update `plugins/example/skills/`**

    Replace the action table in `plugins/example/skills/example/SKILL.md` with your actual actions and documented response shapes. Keep or adapt `plugins/example/skills/scaffold-project/SKILL.md` if you want the elicitation setup wizard to generate approval-first scaffold plans. Good skill docs drive better AI tool use.

20. **Update `plugins/example/.codex-plugin/plugin.json`** for Codex plugin registry

    Every field marked `TEMPLATE:` must be replaced. Key fields:
    - `name` — `<your-service>-mcp`
    - `interface.displayName` — human-readable name
    - `interface.shortDescription` — 50-char tagline
    - `interface.capabilities` — `["Read"]` or `["Read", "Write"]` based on your server
    - `interface.defaultPrompt` — 3 sample prompts demonstrating your actions
    - `interface.brandColor` — hex color matching your service's brand

    See `plugins/example/.codex-plugin/README.md` for the full field reference.

21. **Write `server.json`** for MCP registry publishing

    Update every `TEMPLATE:` field in `server.json` at the repo root:
    - `name` — your reverse-DNS namespace (e.g. `yourdomain.com/myservice-mcp`)
    - `description` — one-sentence description
    - `repository.url` — your GitHub repo URL
    - `packages[0].identifier` — your OCI image ref
    - `environmentVariables` — your service's actual env vars

    See `docs/MCP-REGISTRY-PUBLISH-GUIDE.md` for step-by-step publishing instructions.

#### Tests

22. **Update `tests/mcporter/test-mcp.sh`**

    Add semantic checks for your actions. Validate actual field values, not just key existence.

23. **Run all checks**

    ```bash
    cargo check               # must compile clean
    cargo nextest run         # all tests pass
    taplo check               # TOML format valid
    cargo xtask check-env     # required env vars set
    ```

### After renaming

```bash
# Verify it compiles
cargo check

# Run tests with nextest
cargo nextest run

# Check environment variables
cargo xtask check-env

# Start the server in dev mode
just dev       # no-auth mode on :40060

# Start the static web UI after `pnpm build`
cd apps/web && pnpm start

# Symlink docs for all AI systems
just symlink-docs

# In another terminal, run integration tests
just test-mcporter
```

## License

MIT
