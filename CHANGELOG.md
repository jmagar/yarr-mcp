# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Curated, capability-scoped WRITE/intent commands for the ArrManager kinds sonarr and radarr (C2): `set_quality`, `search`, `refresh`, `monitor`, `unmonitor`, `add`, `delete`. All are `rustarr:write` scope, mutating, and confirm-gated. The headline `set_quality` does a NAME-based bulk quality-profile change — resolve `--to` (and optional `--from`) profile names→ids via the C1 resolver, select items (by `--id`/`--title`, by current `--from` profile, or all), then `PUT /api/v3/<res>/editor` with the correct id key per resource (`seriesIds` for sonarr, `movieIds` for radarr) plus `qualityProfileId`. This replaces the original raw `PUT /api/v3/series/editor` workflow. Capability-wide safety contract (S3/AN-4): when `confirm` is absent every write command returns a structured dry-run preview (`would_do`, `target_profile`, `from_profile`, `count`, `sample_titles`) and mutates NOTHING; `confirm=true` applies and returns a concise summary (`{changed, from, to}`) rather than raw editor blobs. A count cap refuses to act on more than 100 items in one call unless `bulk=true` (CLI `--bulk`); `delete` is opt-in for file deletion (`--delete-files`), always confirm-gated and capped. `search`/`refresh` start ASYNC `/command` jobs (case-sensitive typed command-name constants `SeriesSearch`/`MoviesSearch`/`RefreshSeries`/`RefreshMovie`) and do not poll. Available on both surfaces — MCP `action=set_quality service=sonarr from=Ultra-HD to=HD-1080p` and CLI `rustarr sonarr set-quality --from "Ultra-HD" --to "HD-1080p" [--confirm]`. Business logic lives in `src/app/arr/write.rs`; descriptors extend the existing `ARR_COMMANDS` slice in `src/actions/commands/arr.rs`; CLI parse extends `src/cli/commands/arr.rs`. The curated arr `delete` verb now owns `rustarr <arr-service> delete` (the generic passthrough `delete` remains for non-arr kinds). New typed param extractors `i64_arg`/`i64_array_arg`/`string_array_arg` in `src/actions/parse.rs`; `ids`/`title`/`bulk`/`delete_files`/`id` get richer schema types in `src/mcp/schemas/properties.rs`.
- Curated, capability-scoped READ commands for the ArrManager kinds sonarr and radarr (the first real curated commands, establishing the per-capability plug-in pattern that later capability beads follow): `quality_profiles`, `list`, `wanted`, `queue`, `history`, `rootfolders`, `health`. All are `rustarr:read` scope, non-mutating. `list` is slimmed to `id,title,qualityProfileId,monitored,sizeOnDisk,status,added`. Available on both surfaces — MCP `action=list service=sonarr` and CLI `rustarr sonarr list` (kebab-case CLI verbs map to snake_case action names). Business logic lives in `src/app/arr/{read,resolve}.rs`; descriptors in `src/actions/commands/arr.rs`; CLI parse hook in `src/cli/commands/arr.rs`. The registry's curated table is now a runtime concat of per-capability const slices (`actions::registry::curated_commands()`, the single extension point); a `RustarrAction::Curated { name, params }` carrier routes curated names through the shared `execute_service_action` dispatch (and its action×kind guard) on both CLI and MCP. Curated arr commands appear in the generated schema enum/conditionals, help, and capability digest, and are rejected for non-arr kinds (e.g. `list` on plex) with a teaching valid-actions error.
- MCP tool input schema is now fully generated from the action registry + capability map. `src/mcp/schemas.rs` is a thin facade over `src/mcp/schemas/properties.rs` (property set = generic params ∪ curated-command params ∪ `verbose`/`fields`) and `src/mcp/schemas/conditionals.rs` (action→required-params and action→allowed-kind `allOf` fragments). Adding a curated-command descriptor now surfaces it in the enum, properties, conditionals, and help with no schema edits.
- `verbose` (bool) and `fields` (string array) response-verbosity opt-ins on the `rustarr` tool schema; default responses stay slim.
- Action×kind validation enforced in the SHARED dispatch path (`actions::dispatch::validate_action_for_service`, called by `execute_service_action`) so both CLI and MCP reject a curated command run against an incompatible service kind. The `ActionNotValidForKind` error carries the valid-action list. The 7 generic/infra actions remain valid for every kind.
- `integrations` action output now includes per-service `capability` and `available_actions`, supported kinds carry their capability class, and a registry-derived `capability_digest` is added when curated commands exist. The same digest is embedded in the generated tool description and help.
- Help text for the MCP `help` action is generated from the registry (`src/mcp/help.rs`), replacing the static `HELP_TEXT` const.
- `token_limit::serialize_with_limit` emits a parseable `{ "truncated": true, "reason", "partial" }` JSON envelope when a response exceeds the budget, instead of appending a notice that broke JSON.
- Startup `warn!` when `AuthPolicy::TrustedGatewayUnscoped` is active with mutating actions registered, documenting that scope checks are bypassed in that mode (`confirm=true` still gates mutations).
- `api_put` and `api_delete` passthrough actions (CLI `rustarr put` / `rustarr delete`, MCP `action=api_put` / `action=api_delete`). Both require `rustarr:write` scope and `confirm=true`, completing HTTP-method coverage so rustarr can perform upstream resource updates (e.g. Sonarr/Radarr `series`/`movie` `editor` bulk edits) and deletions. Empty upstream success bodies now return `{ "ok": true, "status": <code> }` instead of erroring.
- Transport split (`src/rustarr/{auth,helpers}.rs`) and per-service auth driven from the `KindDescriptor` capability table: descriptor-driven path allowlists (with Jellyfin `/Sessions`), `query_get` helper that percent-encodes user text for SABnzbd/Tautulli query APIs, `slim()` field-selection helper, and an optional `accept_mime` on `request_json` for JSON negotiation (Plex).

### Changed

- CLI restructured to the `rustarr <service> <command> [flags]` grammar. The generic passthrough verbs are now service-grouped (`rustarr sonarr status`, `rustarr sonarr get --path P`, `rustarr sonarr post|put|delete --path P [--body JSON] --confirm`) instead of taking `--service NAME`. Infra commands (`integrations`, `help`, `doctor`, `watch`, `setup`, `serve`, `mcp`) remain service-less. A new router resolves token1 as either an infra verb or a `ServiceKind`, and USAGE is generated from the action registry + capability map. `--yes` is accepted as an alias for `--confirm`. `src/cli.rs` is split into `src/cli/{command,router,parse,usage}.rs`; the per-capability command-parse hook (`router::parse_capability_command`) is the seam later capability beads extend.

### Security

- qBittorrent now uses a dedicated cookie-store HTTP client; the shared client is cookie-less so the qBittorrent SID can no longer bleed to other services on the same host.
- No `Authorization: Bearer` is sent for Plex (token via `X-Plex-Token` query) or Jellyfin (uses `Authorization: MediaBrowser Token="…"` with `X-Emby-Token` fallback).
- `x-emby-token=` added to the error-body redaction list.

### Removed

- Removed obsolete template/demo MCP actions (`elicit_name`, `scaffold_intent`) and their scaffold contract artifacts.

## [0.4.0] — 2026-05-14

### Added

- `.github/workflows/codeql.yml` — CodeQL SAST analysis on push to main and weekly scheduled scan; results surface in the GitHub Security tab.
- `.github/workflows/cargo-deny.yml` — license compliance, duplicate dependency, advisory, and source checks via `cargo-deny`.
- `.github/workflows/msrv.yml` — compiles against the declared `rust-version` to catch MSRV regressions early.

## [0.3.0] — 2026-05-14

### Added

- `src/cli/watch.rs` — `rustarr watch` subcommand for live file-system monitoring.
- `plugins/rustarr/monitors/` — plugin monitor definitions for event-driven automation.
- `plugins/rustarr/gemini-extension.json` — Gemini extension manifest for multi-platform plugin distribution.
- `.github/dependabot.yml` + `.github/workflows/dependabot-auto-merge.yml` — automated dependency updates with auto-merge for minor/patch bumps.
- `scripts/asciicheck.py`, `scripts/check-blob-size.py`, `scripts/check-dependency-updates.sh`, `scripts/check-file-size.sh`, `scripts/check-runtime-current.sh`, `scripts/validate-plugin-layout.sh`, `scripts/blob-size-allowlist.txt` — repository validation and quality scripts.
- `tests/plugin_contract.rs` — plugin contract integration tests.
- `docs/PLUGINS.md` — documentation for the plugin system and distribution model.
- `plugins/README.md`, `plugins/rustarr/README.md`, `plugins/rustarr/CLAUDE.md` — plugin-level documentation and agent guidance.
- `apps/web/README.md`, `xtask/README.md`, `tests/README.md`, `scripts/README.md` — README coverage for every major directory.
- `.claude/` — Claude Code project settings for agent-assisted development.

### Changed

- `plugins/rustarr/hooks/plugin-setup.sh` — significant simplification; reduced from ~500 to ~50 lines by extracting reusable logic and removing duplication.
- `Justfile` — expanded with additional recipes covering plugin validation, script checks, and workflow shortcuts.
- `lefthook.yml` — pre-commit hook additions aligned with new script suite.
- `AGENTS.md`, `CLAUDE.md` — updated agent and AI tooling guidance to reflect current project structure.
- `README.md`, `docs/PATTERNS.md` — documentation refreshed for new scripts and plugin layout.

## [0.2.0] — 2026-05-14

### Changed

- Split `src/mcp.rs` into three focused modules: `src/server.rs` (`AppState`, `AuthPolicy`, `build_auth_layer`), `src/server/routes.rs` (Axum router wiring), and `src/api.rs` (REST API handlers). `src/mcp/` now contains only MCP protocol concerns (tools, schemas, prompts, server handler).
- `mcp/rmcp_server.rs` and `mcp/tools.rs` now import `AppState`/`AuthPolicy` from `crate::server` instead of `super`.
- `allowed_origins` visibility widened from `pub(super)` to `pub` to support cross-module access from `server/routes.rs`.
- Updated `src/lib.rs` and `src/main.rs` to reflect new module layout (`pub mod api`, `pub mod server`).

### Added

- `deny.toml` — `cargo-deny` configuration enforcing license allowlist, banning `openssl`/`openssl-sys`, denying yanked crates, and restricting dependency sources to crates.io and `github.com/jmagar/lab.git`. RUSTSEC-2023-0071 acknowledged with rationale.
- `apps/web/CLAUDE.md` — guidance for using the Aurora design system shadcn registry in the Next.js web app: install commands, token conventions, full component catalog, and usage rules.
- `.git/hooks/pre-commit` — enforces the no-`mod.rs` rule at commit time; blocks any staged `mod.rs` file with a clear error message.
- `docs/PATTERNS.md` updated: §1/§1a module layouts reflect new `server`/`api` structure with all `mod.rs` references removed; §5 auth section headers updated; §45 No mod.rs section now includes the git hook script; §A1/§A2 advanced patterns updated to match actual file locations.

### Removed

- `src/mcp/routes.rs` — moved to `src/server/routes.rs`.
- Several obsolete scripts: `backup.sh`, `check-runtime-current.sh`, `plugin-setup.sh`, `reset-db.sh`, `smoke-test.sh`, `test-check-runtime-current.sh`, `validate-marketplace.sh`.
- `docs/server-json-guide.md` — content superseded by `docs/MCP-REGISTRY-PUBLISH-GUIDE.md`.

## [0.1.0] — 2026-05-13

### Added

- Layered architecture: `RustarrClient` (transport) → `RustarrService` (business logic) → MCP/CLI shims
- Action-based dispatch: single `rustarr` MCP tool with `action` parameter routing
- Both transports: Streamable HTTP (`rustarr serve`) and stdio (`rustarr mcp`)
- Bearer token authentication via `RUSTARR_MCP_TOKEN`
- Google OAuth authentication via `RUSTARR_MCP_AUTH_MODE=oauth` (issues RS256 JWTs)
- Loopback/no-auth mode for local development
- MCP elicitation support (`elicit_name` action, spec 2025-06-18) with graceful fallback
- MCP resources: exposes tool schema at `rustarr://schema/mcp-tool`
- MCP prompts: `quick_start` prompt
- CLI with `greet`, `echo`, and `status` subcommands
- Test helpers: `loopback_state()` and `bearer_state()` for credential-free integration tests
- `AuthPolicy` enum making auth choice explicit at construction time
- CORS, Host header validation, request body size limiting built-in
- `resolve_auth_policy_kind()` — refuses to bind `0.0.0.0` without auth (Pattern §27)
- `default_data_dir()` — detects container vs bare-metal, returns `/data` or `~/.rustarr`
- `entrypoint.sh` — Docker entrypoint with permission setup and privilege drop to UID 1000
- `xtask` crate with `dist`, `ci`, `symlink-docs`, `check-env` commands
- `.config/nextest.toml` — nextest configuration with `default` and `ci` profiles
- `taplo.toml` — TOML formatter configuration
- `lefthook.yml` — minimal pre-commit hooks (diff_check, toml_fmt, env_guard)
- `.github/workflows/ci.yml` — CI: fmt, clippy, nextest, taplo, audit, gitleaks
- `.github/workflows/docker-publish.yml` — multi-platform Docker build + Trivy scan
- `.github/workflows/release.yml` — release binaries for linux/amd64 and linux/arm64
- `config.rustarr.toml` — fully annotated config template
- `.env.rustarr` — documented secrets template
- `CHANGELOG.md` following Keep a Changelog format
- Workspace structure: root crate + `xtask/` member
- `symlink-docs` and `symlink-docs-inline` Justfile recipes
