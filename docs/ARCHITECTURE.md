---
title: "Architecture"
doc_type: "guide"
status: "active"
owner: "yarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-05-15"
---

# Architecture

`yarr` is a Rust MCP and CLI server built on `rmcp`. It exposes **two surfaces
only — MCP and CLI**; it does not ship a local REST action API or an embedded web
UI (see the Surfaces table in `README.md`). The architecture is intentionally
layered so transports stay thin and business logic stays testable.

## Layer diagram

```
YarrClient  (src/yarr.rs)   → HTTP/API transport ONLY — network calls, no logic
YarrService (src/app.rs)       → ALL business logic, validation, enrichment
MCP shim       (src/mcp/tools.rs) → parse JSON args → call service → return Value
CLI shim       (src/cli.rs)       → parse argv → call service → print
```

**The golden rule:** If you are writing business logic in `mcp/tools.rs`, `cli.rs`, or `main.rs`, you are doing it wrong. Move it to `app.rs`.

## Module layout

```
src/
  yarr.rs        ← HTTP/API transport ONLY (no business logic)
  yarr/          ← per-service auth + transport helpers
  capability.rs     ← Capability enum + KindDescriptor table (SSOT per kind)
  app.rs            ← ALL business logic, validation, transformations
  app/              ← per-capability business modules (arr, indexer, download, …)
  actions/          ← action registry, dispatch, parse, help, command descriptors
  config.rs         ← Config structs + env overrides
  server.rs         ← AppState, AuthPolicy, build_auth_layer
  server/
    routes.rs       ← axum router: wires /mcp + /health + /status + OAuth discovery
  mcp.rs            ← MCP module entry: submodule decls + re-exports only
  mcp/
    tools.rs        ← thin shim: parse args → call service → return Value
    schemas.rs      ← tool JSON schema (enum derived from the action registry)
    rmcp_server.rs  ← ServerHandler impl (tools, resources, prompts, scopes)
    prompts.rs      ← MCP prompt definitions
    transport.rs    ← Streamable HTTP transport wiring and session lifecycle
  cli.rs            ← thin shim: parse args → call service → format/print
  cli/
    command.rs      ← Command enum (incl. Curated { action, params })
    router.rs       ← resolves token1 as infra verb or ServiceKind
    doctor.rs       ← pre-flight checks: env, connectivity, config validation
    setup.rs        ← interactive first-run / plugin setup wizard
    watch.rs        ← polls /health and emits state-change lines for plugin monitor
  token_limit.rs    ← token budget enforcement for MCP response payloads
  lib.rs            ← pub modules + test helpers (testing::*)
  main.rs           ← mode dispatch ONLY (HTTP server / stdio / CLI)
```

## Core files

| File | Responsibility |
|---|---|
| `src/yarr.rs` | Upstream/client transport stub. Replace with your service API client. |
| `src/app.rs` | Service layer. All business rules live here. |
| `src/actions.rs` | Re-export facade over the `actions/` submodules. |
| `src/actions/registry.rs` | `ACTION_SPECS` + `CommandDescriptor` table; `curated_commands()`. |
| `src/mcp/tools.rs` | MCP tool dispatch (thin shim). |
| `src/mcp/schemas.rs` | Tool input schema generated from the action registry. |
| `src/mcp/rmcp_server.rs` | `ServerHandler`, scope enforcement, tools/resources/prompts. |
| `src/server.rs` | Axum server startup, auth policy resolution, app state. |
| `src/server/routes.rs` | HTTP routes for `/mcp`, `/health`, `/status`, and OAuth discovery. |
| `src/config.rs` | Environment/config loading and safe defaults. |
| `src/main.rs` | Mode dispatch: HTTP server, stdio MCP, CLI, setup commands. |

## AppState

```rust
#[derive(Clone)]
pub struct AppState {
    pub config: McpConfig,        // MCP server config (host, port, auth settings)
    pub auth_policy: AuthPolicy,  // LoopbackDev | TrustedGatewayUnscoped | Mounted
    pub service: YarrService,  // The service layer — everything routes through here
}
```

`AppState` is cloned per-request by the RMCP framework. Keep it cheap to clone — the service wraps an `Arc`-backed `reqwest::Client` internally.

## Route composition

The MCP HTTP server is **one binary on one port**. There is no REST action API
and no embedded web UI:

```
Port 40070
  ├── /mcp                              → Streamable HTTP MCP transport (auth-gated)
  ├── /health                          → Unauthenticated liveness probe
  ├── /ready                           → Unauthenticated readiness probe
  ├── /status                          → Runtime state (unauthenticated, secrets redacted)
  ├── /metrics                         → Prometheus metrics (unauthenticated)
  └── /mcp/.well-known/*               → OAuth discovery metadata (when auth_mode=oauth)
```

```rust
// src/server/routes.rs
pub fn router(state: AppState) -> Router {
    // Auth layer is applied to /mcp.
    let authenticated =
        Router::new().nest_service("/mcp", streamable_http_service(state.clone(), rmcp_config));

    let public = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/status", get(status));

    let mut base = Router::new().merge(authenticated).merge(public);

    // OAuth discovery routes are merged only when auth_mode = oauth.
    if let Some(oauth) = oauth_router {
        base = base.merge(oauth);
    }
    base
}
```

## CLI thin shim pattern

`src/cli.rs` follows the same shim discipline as `mcp/tools.rs`. The canonical shape:

```rust
// cli.rs — binary module (uses `rustarr_mcp::` not `crate::`)
use rustarr_mcp::app::YarrService;

pub enum CliCommand {
    Things,
    Thing { id: String },
    DeleteThing { id: String, confirm: bool },
}

impl CliCommand {
    pub fn parse(args: &[String]) -> Result<(Self, bool)> {
        let json    = args.iter().any(|a| a == "--json");
        let confirm = args.iter().any(|a| a == "--confirm");
        let rest: Vec<&str> = args.iter()
            .filter(|a| a.as_str() != "--json" && a.as_str() != "--confirm")
            .map(String::as_str).collect();

        let cmd = match rest.as_slice() {
            ["things"]         => Self::Things,
            ["thing", id, ..]  => Self::Thing { id: id.to_string() },
            ["delete", id, ..] => Self::DeleteThing { id: id.to_string(), confirm },
            other => bail!("unknown command: {}\n\nRun `yarr --help`", other.join(" ")),
        };
        Ok((cmd, json))
    }
}

pub async fn run(service: &YarrService, cmd: CliCommand, json: bool) -> Result<()> {
    let (label, data) = match cmd {
        CliCommand::Things                            => ("things", service.list_things().await?),
        CliCommand::Thing { ref id }                  => ("thing",  service.get_thing(id).await?),
        CliCommand::DeleteThing { ref id, confirm }   => ("delete", service.delete_thing(id, confirm).await?),
    };
    if json { println!("{}", serde_json::to_string_pretty(&data)?); }
    else    { print_human(label, &data); }
    Ok(())
}
```

`parse()` extracts flags and dispatches to variants — no defaults, no validation, no domain logic. `run()` calls the service and formats output. That's it.

## What "thin shim" means

`mcp/tools.rs` does exactly three things per action:
1. Extract named arguments from the `Value` args object
2. Call the corresponding `state.service.method()`
3. Return the `Value` result

`cli.rs` does exactly three things per command:
1. Parse CLI flags/positional args into typed values
2. Call the corresponding `service.method()`
3. Format and print the result (or pass `--json` through verbatim)

Zero validation, zero defaults, zero error message crafting in shims. All of that lives in `app.rs`.

## Split rules — when to make a directory vs a file

| Surface | Split into a directory when… |
|---|---|
| `yarr/` | upstream transport has ≥ 2 concerns (e.g. auth + helpers) |
| `app/` | service methods exceed one focused domain (split per capability) |
| `actions/commands/` | each capability owns a `CommandDescriptor` const slice |
| `cli/commands/` | each capability owns a parse module + `VERBS` table |

## File size targets

| Threshold | Action |
|---|---|
| ≤ 250 non-test lines | Target — ideal module size |
| > 400 non-test lines | Must add split/refactor note in PR |
| > 600 non-test lines | Requires documented exception |
| > 800 total lines | Must split unless generated/fixture/schema |

## Modern Rust requirements

- No `mod.rs` files — use named module files (`mcp.rs` + `mcp/tools.rs`)
- Rust 2024 edition
- `thiserror` for structured error types in the service layer
- `?` operator chains over nested `match`
- Avoid `unwrap()`/`expect()` in production paths

## Invariants

- Shims do not contain business logic.
- All action metadata starts in `src/actions.rs`.
- Read actions require `yarr:read`; write actions require `yarr:write`; `help` is public.
- Stdio is local trusted transport; HTTP is protected unless in loopback or explicit trusted-gateway mode.
- Plugin setup is binary-owned: hook scripts delegate to `yarr setup plugin-hook`.

See `docs/PATTERNS.md` §1, §7, §A1, §45 for full pattern details.
