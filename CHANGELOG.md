# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.1](https://github.com/jmagar/yarr/compare/v1.1.0...v1.1.1) (2026-07-16)

### Changed

- Hardened authentication and destructive Code Mode dispatch, staged release
  and container promotion, immutable production Compose deployment, repository
  protections, observability, runbooks, documentation, and distribution
  contracts following the full-project review.

## [1.1.0](https://github.com/jmagar/yarr/compare/v1.0.0...v1.1.0) (2026-07-09)


### Added

* **plugins:** default yarr plugin's MCP connection to stdio ([cee8c0b](https://github.com/jmagar/yarr/commit/cee8c0bf224d0fa539f39d9f8eedb2938025bd07))
* **plugins:** wire up Gemini CLI's yarr MCP connection over stdio ([b8d3f30](https://github.com/jmagar/yarr/commit/b8d3f3043514e6fa564cc7c49064664e564f0f1d))


### Fixed

* **plugins:** repo-wide skill review fixes, README refresh, GitHub repo rename ([048bc82](https://github.com/jmagar/yarr/commit/048bc82d4874f6c8be7e0bc31d31f382e06d990a))
* unblock CI/release pipeline and correct install.sh exit code ([1047a46](https://github.com/jmagar/yarr/commit/1047a4671a1f014fff919227d1885f37159bfec7))

## [1.0.0](https://github.com/jmagar/yarr/compare/v0.5.0...v1.0.0) (2026-07-09)


### ⚠ BREAKING CHANGES

* remove the confirm=true param/--confirm flag entirely

### Fixed

* **ci:** align codeql-action/analyze to v4.36.3 ([069405d](https://github.com/jmagar/yarr/commit/069405d3b72a85b103747711ea803963247954ba))
* **ci:** align codeql-action/analyze to v4.36.3 ([c87bdce](https://github.com/jmagar/yarr/commit/c87bdce92a616788cd951bdd0a22d46929675991))
* **ci:** switch OpenWiki to local openai-compatible proxy ([19ea54e](https://github.com/jmagar/yarr/commit/19ea54ecc4cec7be49d86c4a62c01ec05f5fc35f))
* **deps:** bump crossbeam-epoch to 0.9.20 (RUSTSEC-2026-0204) ([47ef606](https://github.com/jmagar/yarr/commit/47ef606aef99abfd6201c927db0ac1b104d3ea7e))
* **mcp:** elicit generated DELETE ops dispatched via action=op ([3ffa393](https://github.com/jmagar/yarr/commit/3ffa393a44ef52ccc526e60ac8ff672e27c28237))
* **xtask:** drop dead --confirm arg from live harness, fix contract error truncation ([eec8a34](https://github.com/jmagar/yarr/commit/eec8a3404fd3fa05d8364e17b0aea1f47159d73a))


### Changed

* remove the confirm=true param/--confirm flag entirely ([fa06a8d](https://github.com/jmagar/yarr/commit/fa06a8d4202b937e86e5a7b013dc591fa052587c))

## [0.5.0](https://github.com/jmagar/yarr/compare/v0.4.0...v0.5.0) (2026-07-06)


### Added

* **codemode:** blend TEI semantic similarity into codemode.search() ([ad6fa40](https://github.com/jmagar/yarr/commit/ad6fa400bfa556066ae100bfd1748b895378fab8))
* **mcp:** add YARR_MCP_TOOL_MODE=flat ([b13c15b](https://github.com/jmagar/yarr/commit/b13c15bfa2172c5d61a3b71dccdd4c4e6b46aee6))


### Fixed

* **docs:** ASCII-hygiene fix for OpenWiki testing doc ([4ad1016](https://github.com/jmagar/yarr/commit/4ad1016f3925b92d0cc75154dc2535b0f0118810))
* **docs:** replace non-ASCII check/cross emoji in openwiki/testing.md ([99a3db3](https://github.com/jmagar/yarr/commit/99a3db326132b8b68dad3dfb5376ae5ec5b85a76))
* **live:** accept known Overseerr mcporter domain responses ([dc0b383](https://github.com/jmagar/yarr/commit/dc0b383245abf37e9c7c1bc79ad154872f55eb62))
* **live:** retry mcporter server startup ([63c4ff3](https://github.com/jmagar/yarr/commit/63c4ff3e4cb6590d004d30c9345578b371b88d1d))


### Changed

* **live:** split contract reset operations ([9276929](https://github.com/jmagar/yarr/commit/927692900de375a2a60ff2e45d1bafec84c99d4a))
* **live:** split mcporter contract helpers ([4e8977a](https://github.com/jmagar/yarr/commit/4e8977a6281716097f773da9f0813206ea3449f6))

The entries below predate the Yarr rename and intentionally preserve the
historical `rustarr` names used by those releases.

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
- `config.yarr.toml` — fully annotated config template
- `.env.rustarr` — documented secrets template
- `CHANGELOG.md` following Keep a Changelog format
- Workspace structure: root crate + `xtask/` member
- `symlink-docs` and `symlink-docs-inline` Justfile recipes
