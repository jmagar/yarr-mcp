# rustarr — Agent instructions

## What this project is

A Rust template for building MCP servers with the rmcp crate. The stub binary is named `rustarr`. All `Rustarr*` / `RUSTARR_*` identifiers are renamed when the template is adapted for a real service.

## Key files

| File | Role |
|------|------|
| `src/rustarr.rs` | `RustarrClient` — transport stub; replace with your HTTP/API client |
| `src/app.rs` | `RustarrService` — ALL business logic lives here |
| `src/mcp/tools.rs` | MCP dispatch shim — parse args, call service, return Value |
| `src/mcp/schemas.rs` | Tool JSON schema and action list |
| `src/mcp/rmcp_server.rs` | `ServerHandler` impl: tools, resources, prompts, scope enforcement |
| `src/mcp/routes.rs` | Axum router (`/mcp`, `/health`, OAuth routes) |
| `src/mcp/prompts.rs` | MCP prompts |
| `src/mcp.rs` | `AppState`, `AuthPolicy`, auth layer builder |
| `src/config.rs` | Config structs and env loading |
| `src/cli.rs` | CLI dispatch shim |
| `src/main.rs` | Mode dispatch: HTTP / stdio / CLI |
| `src/lib.rs` | Public API and test helpers |
| `tests/` | Integration tests (`cli_parse.rs`, `tool_dispatch.rs`) |

## Architecture

```
RustarrClient  (rustarr.rs)    ← network calls only
      ↓
RustarrService (app.rs)        ← all business logic
      ↓
  ┌─────────────────────────────┐
  │  MCP shim (mcp/tools.rs)   │  JSON args → service → Value
  │  CLI shim (cli.rs)         │  CLI args  → service → print
  └─────────────────────────────┘
```

## Surface parity policy

Every business action MUST be exposed through both MCP and CLI. Treat MCP + CLI as the minimum supported surface for every scaffolded server.

REST API and Web UI are optional surfaces based on server type:

| Server type | Required surfaces | Rustarrs |
|---|---|---|
| Upstream-client MCP server | MCP + CLI | `unrust`, `rustifi`, `rustify`, `rustscale`, `apprise` |
| Application/platform server | API + CLI + MCP + Web | `axon`, `lab`, `syslog` |

Do not add a REST/Web surface just to mirror an upstream HTTP API. For upstream-client servers, the value is the MCP tool surface plus an equivalent CLI for scripting, debugging, and parity tests.

Exception: `scaffold_intent` is MCP-only because it is specifically an MCP elicitation + plugin skill handoff workflow. There is no true CLI equivalent for exercising client-rendered elicitation and skill selection inside the user's agent/editor permission model.

## Invariant: zero logic in shims

`mcp/tools.rs` and `cli.rs` must not contain business logic. They parse inputs and delegate to `RustarrService`. All computation, validation, and transformation belongs in `app.rs`.

## How to add an action

MCP + CLI steps are mandatory for every business action:

1. `src/rustarr.rs` — add transport method returning `Result<Value>`
2. `src/app.rs` — add service method delegating to client
3. `src/actions.rs` — add action metadata to `ACTION_SPECS`
4. `src/mcp/schemas.rs` — add new parameter schema entries to `tool_definitions()`
5. `src/mcp/tools.rs` — add match arm in `dispatch_rustarr()`; update `HELP_TEXT`
6. `src/cli.rs` — add `Command` variant, parse arm, dispatch arm
7. `tests/tool_dispatch.rs` and CLI tests — add parity coverage

For application/platform servers only, also update:

8. REST API handlers/schemas for the action
9. `apps/web/lib/template.ts`, web forms, and API explorer rustarrs

## Auth policy

| State | Condition | Behavior |
|-------|-----------|----------|
| `LoopbackDev` | `no_auth=true` or host starts with `127.` | No auth, no scope checks |
| `TrustedGatewayUnscoped` | `RUSTARR_NOAUTH=true` behind an authz-enforcing gateway | No auth, no scope checks |
| `Mounted { auth_state: None }` | Default non-loopback | Static bearer token required |
| `Mounted { auth_state: Some(_) }` | `RUSTARR_MCP_AUTH_MODE=oauth` | Google OAuth + RS256 JWT |

`help` action requires no scope. Read actions require `rustarr:read`; mutating actions require `rustarr:write`, which satisfies read.

## Environment variables

```
RUSTARR_API_URL              Upstream service base URL
RUSTARR_API_KEY              Upstream service API key
RUSTARR_MCP_HOST             Bind host (default 0.0.0.0)
RUSTARR_MCP_PORT             Bind port (default 3100)
RUSTARR_MCP_NO_AUTH          Disable auth — loopback only (1/true/yes)
RUSTARR_MCP_TOKEN            Static bearer token
RUSTARR_MCP_ALLOWED_HOSTS    Comma-separated extra Host header values
RUSTARR_MCP_ALLOWED_ORIGINS  Comma-separated extra CORS origins
RUSTARR_MCP_PUBLIC_URL       Public URL for OAuth metadata
RUSTARR_MCP_AUTH_MODE        bearer (default) or oauth
RUSTARR_MCP_GOOGLE_CLIENT_ID     Google OAuth client ID (OAuth mode)
RUSTARR_MCP_GOOGLE_CLIENT_SECRET  Google OAuth client secret (OAuth mode)
RUSTARR_MCP_AUTH_ADMIN_EMAIL  OAuth admin email (OAuth mode)
RUST_LOG                     Log filter (e.g. info,rmcp=warn)
```

## Transports

- `rustarr serve` (or no args) — Streamable HTTP on `RUSTARR_MCP_PORT` (default 3100)
- `rustarr mcp` — stdio transport for child-process MCP clients
- `rustarr greet / echo / status` — direct CLI

## MCP tool actions

Single tool `rustarr`, dispatched by `action` parameter:

| Action | Scope | Description |
|--------|-------|-------------|
| `greet` | `rustarr:read` | Greeting; optional `name` string |
| `echo` | `rustarr:read` | Echo; required `message` string |
| `status` | `rustarr:read` | Server status |
| `elicit_name` | `rustarr:read` | Elicitation demo — asks user for name mid-call |
| `scaffold_intent` | `rustarr:read` | Elicitation setup wizard — returns JSON for the scaffold-project skill |
| `help` | none (public) | Full action reference |

## MCP features implemented

- **Tools** — `rustarr` tool with action dispatch
- **Resources** — `rustarr://schema/mcp-tool` (JSON schema for the tool)
- **Prompts** — `quick_start` prompt
- **Elicitation** — `elicit_name` and `scaffold_intent` actions use `peer.elicit::<...>(...)` (spec 2025-06-18)
- **Scaffold handoff** — `scaffold_intent` returns JSON only; the `scaffold-project` plugin skill turns it into an approval-first plan

## Plugin versioning

Plugin manifests (`.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `gemini-extension.json`) do **not** contain a `version` field. The marketplace derives version from the git commit SHA — an explicit version causes every push to create a duplicate entry. Never add `version` to a plugin manifest.

## Build and test

```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
cargo fmt
```

## Test helpers

`rustarr::testing::loopback_state()` builds `AppState` with no auth — use in all integration tests. `bearer_state(token)` builds a bearer-only state.

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->

## Plugin setup hooks

Plugin setup is owned by the binary. Keep `plugins/rustarr/hooks/plugin-setup.sh` as a thin adapter that maps `CLAUDE_PLUGIN_OPTION_*` values to environment variables, prepares appdata, ensures `rustarr` is on `PATH`, and then calls `rustarr setup plugin-hook "$@"`.

`rustarr setup check` is read-only, `rustarr setup repair` is idempotent, and `rustarr setup plugin-hook --no-repair` is audit mode. Do not add Docker Compose, systemd, or service bootstrap logic back into the hook script. Use `scripts/check-plugin-hook-contract.py` to audit this pattern across the Rust servers.
