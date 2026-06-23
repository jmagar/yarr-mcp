# rustarr — Claude Code instructions

## What this project is

Rust MCP and CLI server for a media automation fleet: Sonarr, Radarr, Prowlarr, Tautulli, Overseerr, SABnzBD, qBittorrent, Plex, Jellyfin, and related services.

The MCP surface is a single `yarr` tool that runs Code Mode (the `codemode` action). The 6 spec-backed services (sonarr/radarr/prowlarr/overseerr/jellyfin/plex) are reached through **generated** per-service callables (from vendored OpenAPI specs); download/stats keep curated commands; every service also has `service_status` + the `api_get/post/put/delete` generic passthrough. Services are declared via `RUSTARR_SERVICES` plus per-service env (see Environment variables).

## Module map

**Transport (`src/rustarr*`)**

| File | Role |
|------|------|
| `src/rustarr.rs` | `RustarrClient` — HTTP transport facade; `request_json` / `send_get` / `send_form_post` against `ServiceConfig` (dedicated cookie client for qBittorrent) |
| `src/rustarr/auth.rs` | Per-service auth application, driven by `AuthStyle` from the `KindDescriptor` table (header / query key / cookie session / Plex / Jellyfin tokens) |
| `src/rustarr/helpers.rs` | `validate_service_path` (descriptor path allowlists, S7), `query_get` (percent-encodes user text for query APIs, S6), `slim()` field selection, error-body redaction |

**Capability model**

| File | Role |
|------|------|
| `src/capability.rs` | `Capability` enum + `KindDescriptor` table (`ServiceKind::descriptor()`): api prefix, auth style, resource noun, path allowlist, `has_metadata_profiles`. SSOT for "what each kind can do" |

**Business layer (`src/app*`) — all logic lives here, never in shims**

| File | Role |
|------|------|
| `src/app.rs` | `RustarrService` — business-layer facade; `execute_service_action` shared dispatch entry |
| `src/app/openapi_ops.rs` | Generated-operation executor: one `(service, op, args)` → upstream request for the 6 spec-backed kinds (sonarr/radarr/prowlarr/overseerr/jellyfin/plex). No per-op code — see `src/openapi*` |
| `src/app/download.rs` + `app/download/{sab,qbit}.rs` | DownloadClient — per-client implementations (SAB query API, qBittorrent v2 REST/cookie) |
| `src/app/stats.rs` | Stats (tautulli) activity/history/users/libraries plus maintenance writes (refreshes run immediately; `delete_image_cache` confirm-gated); `{response}` envelope unwrap |

The 6 spec-backed kinds have **no hand-written app modules** (the old `arr`/`indexer`/
`media_server`/`requests` capability handlers were removed) — their entire API is
generated. Only `download` (sabnzbd/qbittorrent) and `stats` (tautulli) keep curated
commands; bazarr/tracearr use the generic passthrough.

**Generated OpenAPI surface (`src/openapi*` + `specs/`)**

The 6 spec-backed services are generated from the vendored OpenAPI specs under
`specs/` by `cargo xtask gen-openapi` — ~1356 operations + ~808 component types
total. Inside Code Mode they are per-service callables (`sonarr.get_series()`,
`radarr.post_movie({body})`, …) dispatched through the `op` action; component types
are surfaced via `codemode.describe`.

| File | Role |
|------|------|
| `src/openapi.rs` | `OperationSpec`/`TypeDef` runtime shapes + per-kind registry (`operations_for_kind`, `types_for_kind`, `is_generated`, `find_operation`) |
| `src/openapi/generated.rs` + `generated/<svc>.rs` | GENERATED tables (`OPERATIONS`, `TYPES`) — do not edit; regenerate with `cargo xtask gen-openapi` |
| `xtask/src/gen_openapi.rs` | The generator: parse spec `paths`+`components` → emit operation table + TS-interface type catalog |

**Code Mode (`src/codemode*` + `src/app/codemode.rs`)**

Run a JS async arrow fn that calls rustarr actions — port of lab's gateway Code Mode. The `codemode` action (the single MCP `yarr` tool) / `rustarr codemode --code|--file` (CLI) take a `code` string; the script gets **per-service callables `<service>.<verb>(params)`** with the service baked in (generated OpenAPI operations for the 6 spec-backed kinds via the `op` action, curated commands for download/stats), a typed `api.<service>.get/post/put/delete(path, body)` client, `callTool(action, params)` escape hatch, `codemode.search`/`describe` discovery, `codemode.run(name, input)`/`codemode.snippets()`, and `writeArtifact(path, content, options?)`. Returns `{result, calls, logs, artifacts, artifactsRunId?}`. Engine is in-process QuickJS via `rquickjs` (no wasmtime/subprocess). It runs on a `spawn_blocking` thread; `callTool`/`writeArtifact` are synchronous native fns that block on a channel round-trip to the async dispatcher, so JS `async`/`await` is driven by a microtask pump. Requires `rustarr:write`; **destructive deletes are refused** mid-script. `RustarrService.data_dir` (set from `resolve_data_dir()` in main.rs/cli.rs) roots both artifacts and the snippet store; `None` disables both.

| File | Role |
|------|------|
| `src/codemode.rs` | Facade + limits (`CODEMODE_TIMEOUT` 30s, 64 MiB heap, stack, max code/artifact/snippet-name sizes; artifacts/snippets subdirs) |
| `src/codemode/engine.rs` | rquickjs harness: register `__rustarrEmitToolCall` + `__rustarrEmitWriteArtifact`, bind `input` JSON, eval preamble + wrapped user code, drain microtasks (outside `ctx.with`), read back `{result, logs}`. Opaque `ToolCaller`/`ArtifactWriter` (`Box<dyn Fn>`); pure of tokio/domain |
| `src/codemode/proxy.rs` | `build_preamble(services)` — `callTool`, `console`, `__rustarrRun`, **per-service callables `globalThis.<service>.<verb>`** (generated ops via the `op` action for spec-backed kinds; curated for download/stats; service baked in), `api.<service>`, injected `__codemodeCatalog` + `codemode.search`/`describe`/`run`/`snippets` |
| `src/codemode/catalog.rs` | Registry-derived discovery catalog (`catalog_json()`), one entry per action — name/kind/scope/destructive/required_params/capability/allowed_kinds |
| `src/codemode/dts.rs` | JsonSchema→TypeScript converter for the **5 doc-based** `src/models` contracts → `service.TypeName` entries; `type_catalog_json_for(services)` MERGES these with the **generated** TS for the 6 spec-backed kinds, injected as `__codemodeTypes` and surfaced ON DEMAND via `codemode.describe`/`search` (configured-service-scoped) |
| `src/codemode/artifact.rs` | Pure fail-closed artifact-path validation (`validate_artifact_path`, `resolve_under_root`) + content-type inference |
| `src/codemode/store.rs` | Snippet store: `validate_snippet_name` (allowlist), `list`/`save`/`load_source`/`delete` under `<data_dir>/codemode/snippets` |
| `src/app/codemode.rs` | `RustarrService::run_script` (shared executor; `codemode` = `run_script(code,None,false)`), dual-channel drain loop (calls + artifacts), `codemode_dispatch` (boxed recursion; refuses destructive/self/`snippet_run`-in-snippet), `snippet_list/save/run/delete`, `write_codemode_artifact` |

**Typed upstream contracts (`src/models*`)** — the **5 doc-based** services only

The 6 spec-backed services (sonarr/radarr/prowlarr/overseerr/jellyfin/plex) no longer
have hand-written models — their types are **generated** from `specs/` into
`src/openapi/generated/` (see Generated OpenAPI surface above). The 5 doc-based
services (no machine-readable spec) keep hand-modeled `Debug/Clone/PartialEq/Serialize/
Deserialize/JsonSchema` structs here. Every field is optional/defaulted, unknown fields
ignored, so partial/version-drifting payloads never hard-fail. Casing mirrors the wire
via `rename_all` + per-field renames (SABnzbd string-encoded numerics, etc.). Each
`<svc>.rs` has a colocated `<svc>_tests.rs`.

| File | Service / source | Notable types |
|------|------|------|
| `src/models.rs` | facade + design rules | — |
| `src/models/tautulli.rs` | Tautulli (docs) | generic `TautulliEnvelope<T>`, `GetActivityData`, `GetHistoryData`, users/libraries/server-info |
| `src/models/sabnzbd.rs` | SABnzbd (docs) | `QueueResponse`/`Queue`/`QueueSlot`, `HistoryResponse`/`HistorySlot`, `VersionResponse` (string-encoded numerics) |
| `src/models/qbittorrent.rs` | qBittorrent (docs) | `TorrentInfo`, `TorrentProperties`, `TransferInfo`, `Category`, `BuildInfo` |
| `src/models/bazarr.rs` | Bazarr (docs) | `SystemStatus`, subtitle/wanted rows, providers, languages |
| `src/models/tracearr.rs` | Tracearr (docs) | public-API resources + `Health` |

**Action registry + dispatch (`src/actions*`)**

| File | Role |
|------|------|
| `src/actions.rs` | Re-export facade over the `actions/` submodules |
| `src/actions/registry.rs` | `ACTION_SPECS` (7 generic actions) + `CommandDescriptor` table; `curated_commands()` (single extension point), `all_action_names()`, `action_allowed_for_kind`, `capability_digest()` |
| `src/actions/model.rs` | `ActionSpec`, `ActionTransport`, scopes, `RustarrAction` enum, `ValidationError` |
| `src/actions/parse.rs` | REST↔MCP arg parsing helpers (`string_arg`, `i64_arg`, `string_array_arg`, …) |
| `src/actions/dispatch.rs` | `validate_action_for_service` (action×kind guard) + curated-command dispatch shared by CLI and MCP |
| `src/actions/help.rs` | Registry-derived `help` action text |
| `src/actions/commands.rs` | Aggregates per-capability descriptor slices (`ARR_COMMANDS`, …) |
| `src/actions/commands/{arr,indexer,download,media_server,requests,stats}.rs` | Per-capability `CommandDescriptor` const slices |

**MCP protocol layer (`src/mcp*`)**

| File | Role |
|------|------|
| `src/mcp.rs` | MCP protocol layer — re-exports from `mcp/` submodules |
| `src/mcp/tools.rs` | MCP shim: parse JSON args → call service → return `Value` |
| `src/mcp/schemas.rs` | Tool JSON schema facade; enum derived from `all_action_names()` |
| `src/mcp/schemas/properties.rs` | Property set: generic + curated params + `verbose`/`fields` |
| `src/mcp/schemas/conditionals.rs` | Generated action→required-params and action→allowed-kind `allOf` fragments |
| `src/mcp/rmcp_server.rs` | `ServerHandler` impl: tools, resources, prompts, scope checks |
| `src/mcp/prompts.rs` | MCP prompts (`quick_start`) |
| `src/mcp/transport.rs` | Streamable HTTP transport wiring and session lifecycle |

**CLI layer (`src/cli*`)**

| File | Role |
|------|------|
| `src/cli.rs` | CLI shim: `parse_args_from`, `run`; dispatches `Command` through the service |
| `src/cli/command.rs` | `Command` enum (incl. `Curated { action, params }`) — pure data |
| `src/cli/router.rs` | Resolves `token1` as infra verb or `ServiceKind`; `parse_capability_command` hook |
| `src/cli/parse.rs` | Shared flag parsing (`parse_passthrough_flags`, `reject_args`, …) |
| `src/cli/usage.rs` | USAGE generated from the registry + capability map; `cli_verb` renders friendly verbs |
| `src/cli/commands.rs` | Per-capability parse modules + `VERBS` verb→action tables (`capability_verb_tables`, `cli_verb_for_action`) |
| `src/cli/commands/{arr,indexer,download,media_server,requests,stats}.rs` | Per-capability CLI parse modules + their `VERBS` SSOT tables |
| `src/cli/doctor.rs` + `cli/doctor/checks.rs` | Pre-flight checks: env, connectivity, config validation |
| `src/cli/setup.rs` | Interactive first-run / plugin setup wizard |
| `src/cli/watch.rs` | Polls `/health` and emits state-change lines for plugin monitor |

**Server, config, infra**

| File | Role |
|------|------|
| `src/server.rs` | `AppState`, `AuthPolicy`, `build_auth_layer` — HTTP server state and auth policy |
| `src/server/routes.rs` | Axum router: `/mcp`, `/health`, `/status`, OAuth discovery routes |
| `src/config.rs` | `Config`, `RustarrConfig`, `ServiceConfig`, `ServiceKind`, `McpConfig`, `AuthConfig`, env loading |
| `src/logging.rs` + `logging/{aurora,formatter}.rs` | Log subscriber + human/Aurora output |
| `src/token_limit.rs` | Token budget enforcement for MCP response payloads |
| `src/main.rs` | Mode dispatch: HTTP server / stdio / CLI |
| `src/lib.rs` | Public API + `testing` helpers (`loopback_state`, `bearer_state`) for integration tests |
| `src/*_tests.rs` | Colocated unit tests — one per module, wired via `#[path = "<mod>_tests.rs"] mod tests;` (see Testing) |

**Integration tests (`tests/`)**

| File | Role |
|------|------|
| `tests/cli_parse.rs` | CLI argument parsing |
| `tests/tool_dispatch.rs` | MCP tool dispatch (service-layer, no real credentials) |
| `tests/parity.rs` | Mechanical CLI ↔ MCP parity guard (see CLI ↔ MCP action parity) |
| `tests/plugin_contract.rs` | Plugin manifest / setup-hook contract |
| `tests/template_invariants.rs` | Template-adaptation invariants + `schema_contract` doc test |

## The thin-shim rule — enforce this hard

`src/mcp/tools.rs` and `src/cli.rs` contain **zero business logic**. They only:
1. Parse their input format (JSON args or CLI flags)
2. Call the corresponding `RustarrService` method
3. Return the result

If you find yourself computing, filtering, transforming, or validating data in `tools.rs` or `cli.rs`, stop and move it to `app.rs`.

## How to add an action (checklist)

New surface for the 6 spec-backed services is added by **regenerating** from the
specs (`cargo xtask gen-openapi`), not hand-written. Curated commands remain only for
the doc-based download/stats capabilities (descriptor-table driven). The generic
`ACTION_SPECS` set (`service_status`, `api_get/post/put/delete`, `help`, `codemode`,
`op`, `snippet_*`) is closed — only extend it for new infra verbs.

**Adding a curated command:**

1. **`src/app/<cap>.rs`** — add `pub async fn your_command(&self, ...) -> Result<Value>` with the business logic and the actual HTTP call (via `RustarrClient`). All logic lives here.

2. **`src/actions/commands/<cap>.rs`** — append a `CommandDescriptor` to the capability's const slice: `name` (globally-unique snake_case action), `capability`, `description`, `required_scope`, `required_params`/`optional_params`, `destructive`, `mutates`, and the `handler`. **`destructive` marks a delete that loses hard-to-recreate data** — it is the SSOT for `action_is_destructive` and the ONLY thing still gated (MCP elicitation / CLI `--confirm`). Set `destructive: true` only for destructive deletes; every other write keeps `mutates: true, destructive: false` and runs immediately. The invariant is **`destructive => mutates`**, and `destructive` agrees with `action_is_destructive` — enforced by `tests/parity.rs`. The slice is concatenated at the single extension point in `src/actions/registry.rs::build_curated_commands` — no enum/match edits.

3. **`src/cli/commands/<cap>.rs`** — add a `(friendly-verb, action)` entry to that module's `VERBS` table (SSOT for USAGE + parity), and a parse arm that marshals flags → JSON `params` into `Command::Curated { action, params }`. No business logic.

4. **Schema/help/usage are automatic** — the enum, properties, conditionals, capability digest, help, and USAGE all derive from the descriptor. Only add a NEW param *type* to `src/mcp/schemas/properties.rs` if the param isn't already declared.

5. **Tests** — add a colocated unit test in the capability's `*_tests.rs`, a dispatch test in `tests/tool_dispatch.rs`, and (for parity) nothing extra: `tests/parity.rs` mechanically asserts the new command is reachable on both surfaces from the registry + `VERBS` table.

6. **`CHANGELOG.md`** — add an entry under `[Unreleased]`.

**Security (S6) — applies to every action that puts user-controlled text into an
upstream request:** route user text through typed params and the percent-encoding
`query_get` / `append_pair` helpers (`src/rustarr/helpers.rs`). **Never `format!`
user text directly into a path or query string** — that allows query/path
injection (e.g. `cmd=delete` smuggled into a Tautulli `cmd`, a second `query`
param, or a `/api/v3/...` escape on a v1-only kind). The shared auth path injects
the API key exactly once; the app layer must not add it.

For actions with parameters, extract them with `string_arg`/`i64_arg`/`string_array_arg`
(in `src/actions/parse.rs`) from the `params` object in the shim.

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
| `RUSTARR_MCP_PORT` | `40070` | Bind port |
| `RUSTARR_MCP_NO_AUTH` | `false` | Disable auth (loopback only) |
| `RUSTARR_NOAUTH` | `false` | Trusted-gateway bypass on non-loopback (see Auth model) |
| `RUSTARR_ALLOW_DESTRUCTIVE` | `false` | Global destructive-op override: destructive `op`s/`api_delete`/curated deletes run without `--confirm` and the Code Mode mid-script delete refusal is lifted. **Disposable test stacks only** (the shart contract harness sets it); never production |
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
just health               # curl http://localhost:40070/health | jq .
```

## Testing

**Unit tests are colocated, not inline.** Each module `foo.rs` keeps its tests in a sibling `foo_tests.rs`, wired at the bottom of the module with:

```rust
#[cfg(test)]
#[path = "foo_tests.rs"]
mod tests;
```

This keeps the `mod_module_files = "deny"` workspace lint happy while avoiding giant inline `#[cfg(test)]` blocks. When adding a module, add its `*_tests.rs` sibling and the `#[path]` include — don't write inline `mod tests { ... }`.

**Integration tests** live in `tests/`: `cli_parse.rs`, `tool_dispatch.rs`, `parity.rs`, `plugin_contract.rs`, `template_invariants.rs`. Keep `tool_dispatch.rs` from growing past ~500 LOC — add new dispatch coverage thoughtfully and put cross-surface parity assertions in `parity.rs`.

`src/lib.rs` exports `testing::loopback_state()` and `testing::bearer_state(token)` (behind `features = ["test-support"]` or `cfg(test)`). Use these in integration tests — they build `AppState` without real credentials.

## CLI ↔ MCP action parity

Every action in the MCP tool must also be reachable from the CLI, and vice versa.
Both shims marshal their input into the SHARED `execute_service_action` dispatch
path, so parity is structural — not something to verify by hand.

**Parity is mechanically enforced by `tests/parity.rs`** (the table below is a
representative summary that can drift; the test is the guard). For every curated
command in `registry::curated_commands()` it asserts: (a) the name is in the MCP
action enum (`all_action_names()`), (b) `rustarr <service> <friendly-verb>` parses
into a matching `Command::Curated`, (c) the `VERBS` tables and the registry cover
exactly the same actions in both directions, (d) each verb's capability matches its
descriptor, and (e) only destructive commands are gated (`destructive => mutates`,
and each command's `destructive` flag agrees with `action_is_destructive`). MCP
resources and prompts are protocol concepts with no CLI analogue.

Grammar: the CLI is **service-grouped** (`rustarr <service> <command> [flags]`).
The **MCP surface is a single tool, `yarr`** (`schemas::yarr_tool()`), taking one
`code` param — it dispatches the `codemode` action, and the whole fleet is reached
inside the script via per-service callables `<service>.<verb>()` (generated ops for the
6 spec-backed kinds; curated for download/stats) plus `api.<service>`/`callTool` +
`codemode.search`/`describe`. So the agent carries one tool schema, not one per
service. Every action is still reachable (from inside `yarr`, and from the CLI); the per-service action
dispatch (`dispatch_service_tool`) remains as the internal/test path that a `yarr`
script's `callTool` mirrors. The MCP action name is globally unique snake_case; the
CLI verb is the short, friendly, capability-local form mapped in each
`src/cli/commands/<cap>.rs` `VERBS` table.

Representative summary (full set lives in the registry + `VERBS` tables):

| Surface area | MCP action(s) | CLI |
|---|---|---|
| Infra | `service_status`, `help`, `codemode` (the single `yarr` tool; `rustarr:write`; runs JS), `op` (generated-op dispatch; MCP/Code-Mode-only), `snippet_*` | `rustarr <service> status`, `rustarr help`, `rustarr codemode --code\|--file`, `rustarr snippet …` |
| Generic passthrough | `api_get`/`api_post`/`api_put` (writes run immediately); `api_delete` (destructive, gated) — all `rustarr:write` | `rustarr <service> get\|post\|put --path P [--body JSON]`; `rustarr <service> delete --path P [--body JSON] --confirm` |
| Sonarr/Radarr/Prowlarr/Overseerr/Jellyfin/Plex (generated) | Generated OpenAPI operations via `op`, reached in Code Mode as `<service>.<op>()` (e.g. `sonarr.get_series`, `radarr.post_movie`). DELETE ops refused mid-script | Code Mode only (no per-op CLI verbs); raw passthrough via `rustarr <service> get/post/...` |
| DownloadClient (sabnzbd/qbittorrent) | `download_queue`, `download_add`, … `download_remove` (destructive) | `rustarr qbittorrent queue \| add --url X \| remove --hash H --confirm` |
| Stats (tautulli) | `stats_activity`, `stats_history`, `stats_refresh_libraries`, … `stats_delete_image_cache` (destructive) | `rustarr tautulli activity \| history [--start N --length N --user U] \| refresh-libraries`; `delete-image-cache --confirm` |

Both `api_get` and `api_post` require `rustarr:write` (read scope is insufficient) — they are arbitrary upstream passthroughs.

## Plugin versioning

Plugin manifests (`.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `gemini-extension.json`) do **not** contain a `version` field. The marketplace derives the version from the git commit SHA on every push — adding an explicit version causes every push to be treated as a new version and creates duplicate entries. Do not add `version` to any plugin manifest and do not run `scripts/bump-version.sh` targets against plugin manifests.

## Common gotchas

- **Stdio mode suppresses logs** — `main.rs` sets log level to `warn` in stdio mode so JSON-RPC is not corrupted by log lines on stdout.
- **Scope checks run in `rmcp_server.rs`**, not in `tools.rs`. `tools.rs` only dispatches.
- **`help` action is public** — `required_scope_for_action("help")` (in `actions.rs`) returns `None`. `service_status` needs `rustarr:read`; `api_get`/`api_post`/`op`/`codemode` need `rustarr:write`. Unknown actions get `DENY_SCOPE`.
- **Default port is 40070** — set in `default_mcp_port()` in `config.rs`. Override with `RUSTARR_MCP_PORT`.
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
