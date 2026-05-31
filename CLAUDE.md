# rustarr — Claude Code instructions

## What this project is

A reusable Rust template for building MCP servers with the rmcp crate. The binary is named `rustarr`. All stub identifiers (`Rustarr*`, `RUSTARR_*`) are renamed when the template is used for a real service.

The shipped example actions wrap the *arr media stack (Sonarr, Radarr, Prowlarr, Overseerr, etc.) through a generic upstream HTTP client: `integrations`, `service_status`, `api_get`, `api_post`. Services are declared via `RUSTARR_SERVICES` + per-service env (see Environment variables). When adapting the template, replace these actions with your own.

## Module map

| File | Role |
|------|------|
| `src/rustarr.rs` | `RustarrClient` — HTTP transport to upstream services; `get_json` / `post_json` against `ServiceConfig` |
| `src/app.rs` | `RustarrService` — business layer; all logic lives here, never in shims |
| `src/actions.rs` | `ACTION_SPECS` registry, `RustarrAction` enum, scope/transport rules, REST↔MCP arg parsing, `ValidationError` |
| `src/scaffold.rs` | `ScaffoldIntent` — backs the MCP-only `scaffold_intent` elicitation wizard for spinning up a new rmcp server |
| `src/server.rs` | `AppState`, `AuthPolicy`, `build_auth_layer` — HTTP server state and auth policy |
| `src/server/routes.rs` | Axum router: `/mcp`, `/health`, `/status`, OAuth discovery routes |
| `src/api.rs` | REST API handlers: `POST /v1/rustarr`, `GET /health`, `GET /status` |
| `src/web.rs` | Serves the bundled `apps/web` static export (when present) |
| `src/mcp.rs` | MCP protocol layer — re-exports from `mcp/` submodules |
| `src/mcp/tools.rs` | MCP shim: parse JSON args → call service → return `Value` |
| `src/mcp/schemas.rs` | Tool JSON schema derived from `ACTION_SPECS` |
| `src/mcp/rmcp_server.rs` | `ServerHandler` impl: tools, resources, prompts, scope checks |
| `src/mcp/prompts.rs` | MCP prompts (`quick_start`) |
| `src/config.rs` | `Config`, `RustarrConfig`, `ServiceConfig`, `ServiceKind`, `McpConfig`, `AuthConfig`, env loading |
| `src/cli.rs` | CLI shim: parse args → call service → print |
| `src/cli/doctor.rs` | Pre-flight checks: env, connectivity, config validation |
| `src/cli/setup.rs` | Interactive first-run / plugin setup wizard |
| `src/cli/watch.rs` | Polls `/health` and emits state-change lines for plugin monitor |
| `src/logging.rs` | Log subscriber setup; `logging/aurora.rs` + `logging/formatter.rs` for human/Aurora output |
| `src/mcp/transport.rs` | Streamable HTTP transport wiring and session lifecycle |
| `src/token_limit.rs` | Token budget enforcement for MCP response payloads |
| `src/main.rs` | Mode dispatch: HTTP server / stdio / CLI |
| `src/lib.rs` | Public API + `testing` helpers for integration tests |
| `src/*_tests.rs` | Colocated unit tests — one per module, wired via `#[path = "<mod>_tests.rs"] mod tests;` (see Testing) |
| `tests/cli_parse.rs` | Integration: CLI argument parsing |
| `tests/tool_dispatch.rs` | Integration: MCP tool dispatch (service-layer, no real credentials) |
| `tests/api_routes.rs` | Integration: REST `/v1/rustarr`, `/health`, `/status` |
| `tests/plugin_contract.rs` | Integration: plugin manifest / setup-hook contract |
| `tests/template_invariants.rs` | Integration: template-adaptation invariants |

## The thin-shim rule — enforce this hard

`src/mcp/tools.rs` and `src/cli.rs` contain **zero business logic**. They only:
1. Parse their input format (JSON args or CLI flags)
2. Call the corresponding `RustarrService` method
3. Return the result

If you find yourself computing, filtering, transforming, or validating data in `tools.rs` or `cli.rs`, stop and move it to `app.rs`.

## How to add an action (checklist)

1. **`src/rustarr.rs`** — add `pub async fn your_action(&self, ...) -> Result<Value>` with the actual HTTP/API call (or stub).

2. **`src/app.rs`** — add a delegating method: `pub async fn your_action(&self, ...) -> Result<Value> { self.client.your_action(...).await }`.

3. **`src/actions.rs`** — add the action to `ACTION_SPECS`, including scope and transport.

4. **`src/mcp/schemas.rs`** — add any new parameters to `tool_definitions()`; the action enum comes from `ACTION_SPECS`.

5. **`src/mcp/tools.rs`** — add a match arm in `dispatch_rustarr()`: `"your_action" => { ... state.service.your_action(...).await }`. Also add to `HELP_TEXT`.

6. **`src/cli.rs`** — add a `Command` variant, a parse arm in `parse_args()`, and a dispatch arm in `run()`.

7. **Tests** — add a colocated unit test in the relevant `*_tests.rs` (e.g. `src/actions_tests.rs` for parsing/scope) and a dispatch test in `tests/tool_dispatch.rs`. If the action is REST-reachable, also cover it in `tests/api_routes.rs`.

8. **`CHANGELOG.md`** — add an entry under `[Unreleased]` describing the new action.

For actions with parameters, extract them with `string_arg(&args, "param_name")` in `tools.rs`.

## Auth model

`AuthPolicy` is an enum with three states:

| Variant | When | Effect |
|---------|------|--------|
| `AuthPolicy::LoopbackDev` | `no_auth=true` or host is loopback (`localhost`, `127.*`, `::1`) via `McpConfig::is_loopback()` | No auth middleware; scope checks bypassed |
| `AuthPolicy::TrustedGatewayUnscoped` | `RUSTARR_NOAUTH=true` on non-loopback behind an authz-enforcing gateway | No auth middleware; scope checks bypassed |
| `AuthPolicy::Mounted { auth_state: None }` | Default non-loopback | Static bearer token required |
| `AuthPolicy::Mounted { auth_state: Some(_) }` | `auth_mode = "oauth"` | Full Google OAuth + RS256 JWT issuance |

Auth is selected in `build_auth_policy()` in `main.rs`. Scopes are `rustarr:read` and `rustarr:write` (write satisfies read). `help` requires no scope. Unknown actions get `DENY_SCOPE`.

## Environment variables

Upstream services are configured as a set, not a single endpoint. `RUSTARR_SERVICES` lists the service names; each name expands to a `RUSTARR_<NAME>_*` env group (name uppercased, non-alphanumerics → `_`). Loaded by `load_services_from_env()` in `config.rs`.

| Variable | Default | Description |
|----------|---------|-------------|
| `RUSTARR_SERVICES` | — | Comma-separated service names, e.g. `sonarr,radarr,overseerr` |
| `RUSTARR_<NAME>_URL` | — | Per-service base URL |
| `RUSTARR_<NAME>_API_KEY` | — | Per-service API key |
| `RUSTARR_<NAME>_KIND` | _(name)_ | Service kind (`ServiceKind`); defaults to the name. Determines status path |
| `RUSTARR_<NAME>_USERNAME` | — | Per-service basic-auth username (where applicable) |
| `RUSTARR_<NAME>_PASSWORD` | — | Per-service basic-auth password |
| `RUSTARR_<NAME>_TOKEN` | — | Per-service token (where applicable) |
| `RUSTARR_SERVER_NAME` / `RUSTARR_MCP_SERVER_NAME` | `rustarr` | MCP server name advertised to clients |
| `RUSTARR_CONFIG` | — | Path to a config file (overrides default lookup) |
| `RUSTARR_HOME` | — | Base dir for appdata/config resolution |
| `RUSTARR_MCP_HOST` | `127.0.0.1` | Bind host |
| `RUSTARR_MCP_PORT` | `40060` | Bind port |
| `RUSTARR_MCP_NO_AUTH` | `false` | Disable auth (loopback only) |
| `RUSTARR_NOAUTH` | `false` | Trusted-gateway bypass on non-loopback (see Auth model) |
| `RUSTARR_MCP_TOKEN` | — | Static bearer token |
| `RUSTARR_MCP_ALLOWED_HOSTS` | — | Extra comma-separated Host header values |
| `RUSTARR_MCP_ALLOWED_ORIGINS` | — | Extra comma-separated CORS origins |
| `RUSTARR_MCP_PUBLIC_URL` | — | Public URL for OAuth metadata endpoints |
| `RUSTARR_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth` |
| `RUSTARR_MCP_GOOGLE_CLIENT_ID` | — | Google OAuth client ID |
| `RUSTARR_MCP_GOOGLE_CLIENT_SECRET` | — | Google OAuth client secret |
| `RUSTARR_MCP_AUTH_ADMIN_EMAIL` | — | OAuth admin email |
| `RUST_LOG` | `info` | Log filter |

`ServiceKind` (15 known kinds): `sonarr`, `radarr`, `prowlarr`, `tautulli`, `overseerr`, `bazarr`, `tracearr`, `lidarr`, `readarr`, `sabnzbd`, `qbittorrent`, `wizarr`, `notifiarr`, `plex`, `jellyfin`. Additional OAuth tuning vars (`RUSTARR_MCP_AUTH_*` TTLs, RPM limits, key/sqlite paths, allowed emails/redirect URIs) are defined in `config.rs`.

## Elicitation

The `elicit_name` action demonstrates MCP elicitation (spec 2025-06-18). The server calls `peer.elicit::<T>()` to ask the MCP client for user input mid-call. The type `T` must:
- Derive `JsonSchema`, `Serialize`, `Deserialize`
- Be an object (struct), not a primitive
- Be registered with `rmcp::elicit_safe!(T)`

`ElicitationError::CapabilityNotSupported` is handled gracefully — clients that don't support it get a fallback message instead of an error.

## Build commands

```bash
cargo build --release     # produces target/release/rustarr
cargo test                # all tests
cargo clippy -- -D warnings  # lint (must pass)
cargo fmt                 # format

just dev                  # RUSTARR_MCP_HOST=127.0.0.1 RUSTARR_MCP_NO_AUTH=true cargo run -- serve mcp (loopback only, no auth)
just test                 # cargo test
just lint                 # cargo clippy -- -D warnings
just fmt                  # cargo fmt
just gen-token            # openssl rand -hex 32
just health               # curl http://localhost:40060/health | jq .
```

## Testing

**Unit tests are colocated, not inline.** Each module `foo.rs` keeps its tests in a sibling `foo_tests.rs`, wired at the bottom of the module with:

```rust
#[cfg(test)]
#[path = "foo_tests.rs"]
mod tests;
```

This keeps the `mod_module_files = "deny"` workspace lint happy while avoiding giant inline `#[cfg(test)]` blocks. When adding a module, add its `*_tests.rs` sibling and the `#[path]` include — don't write inline `mod tests { ... }`.

**Integration tests** live in `tests/`: `cli_parse.rs`, `tool_dispatch.rs`, `api_routes.rs`, `plugin_contract.rs`, `template_invariants.rs`.

`src/lib.rs` exports `testing::loopback_state()` and `testing::bearer_state(token)` (behind `features = ["test-support"]` or `cfg(test)`). Use these in integration tests — they build `AppState` without real credentials.

## CLI ↔ MCP action parity

Every action in the MCP tool must also be reachable from the CLI, and vice versa.
Both shims call the same `RustarrService` methods, so parity is automatic when the
shims are complete.

**Exception — MCP-only features:** `elicit_name` and MCP resources/prompts have no
CLI equivalent. Elicitation requires a live MCP client interaction (the server asks
the user for input mid-call via `peer.elicit()`); that interaction model does not
translate to a one-shot CLI call. Resources and prompts are MCP protocol concepts
with no CLI analogue.

| Service Method | MCP Action | CLI Command | Notes |
|---|---|---|---|
| `service.integrations()` | `rustarr(action="integrations")` | `rustarr integrations` | Lists supported + configured services. Sync |
| `service.service_status(service)` | `rustarr(action="service_status", service="sonarr")` | `rustarr status --service NAME` | Upstream status for one service |
| `service.api_get(service, path)` | `rustarr(action="api_get", service="...", path="...")` | `rustarr get --service NAME --path PATH` | Passthrough GET; requires `rustarr:write` |
| `service.api_post(service, path, body, confirm)` | `rustarr(action="api_post", service="...", path="...", body={...}, confirm=true)` | `rustarr post --service NAME --path PATH --body JSON --confirm` | Passthrough POST; `confirm` gates mutation; requires `rustarr:write` |
| _(MCP client interaction)_ | `rustarr(action="elicit_name")` | _(MCP-only — no CLI equivalent)_ | Requires elicitation-capable client |
| _(MCP elicitation wizard)_ | `rustarr(action="scaffold_intent")` | _(MCP-only — no CLI equivalent)_ | Combines elicitation + skill handoff; no one-shot CLI equivalent |
| `rest_help()` | `rustarr(action="help")` | `rustarr help` / `rustarr --help` | MCP + `rustarr help` return structured JSON; `--help` prints usage |

Both `api_get` and `api_post` require `rustarr:write` (read scope is insufficient) — they are arbitrary upstream passthroughs.

**TEMPLATE:** Replace this table with your service's actual actions when you adapt
the template. The rule is: one row per service method, with both the MCP action name
and the CLI subcommand/flag documented.

## Plugin versioning

Plugin manifests (`.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `gemini-extension.json`) do **not** contain a `version` field. The marketplace derives the version from the git commit SHA on every push — adding an explicit version causes every push to be treated as a new version and creates duplicate entries. Do not add `version` to any plugin manifest and do not run `scripts/bump-version.sh` targets against plugin manifests.

## Common gotchas

- **Stdio mode suppresses logs** — `main.rs` sets log level to `warn` in stdio mode so JSON-RPC is not corrupted by log lines on stdout.
- **`config.toml` is a template file** — it still contains `unraid-mcp` values; update it when adapting this template.
- **Scope checks run in `rmcp_server.rs`**, not in `tools.rs`. `tools.rs` only dispatches.
- **`help` action is public** — `required_scope_for_action("help")` (in `actions.rs`) returns `None`. `integrations` and `service_status` need `rustarr:read`; `api_get` and `api_post` need `rustarr:write`. Unknown actions get `DENY_SCOPE`.
- **Default port is 40060** — set in `default_mcp_port()` in `config.rs`. Override with `RUSTARR_MCP_PORT`.
- **`elicit_name` is MCP-only** — elicitation requires a live client connection; it cannot be invoked from the CLI. This is the one intentional parity exception.
- **`watch`, `serve`, and `doctor` are CLI infrastructure** — they are not MCP actions and have no parity requirement. `watch` polls `/health` and emits state-change lines to stdout (used by the plugin monitor). `serve` starts the HTTP server. `doctor` runs pre-flight checks. None belong in the MCP parity table.


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
