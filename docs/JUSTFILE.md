---
title: "Justfile"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
last_reviewed: "2026-05-15"
---

# Justfile

`Justfile` is the operator command surface for local development, CI parity, Docker, plugin packaging, and diagnostics. Run `just --list` for the complete current list.

## Core development recipes

| Recipe | Purpose |
|---|---|
| `just dev` | Run HTTP MCP server on loopback in no-auth dev mode (`RUSTARR_MCP_NO_AUTH=true`). |
| `just mcp` | Run stdio MCP transport (`rustarr mcp`). |
| `just integrations` | Quick CLI inventory smoke test. |
| `just doctor` | Pre-flight environment/connectivity checks (`rustarr doctor`). |
| `just live-read-smoke` | Shart-only live read-only CLI and upstream API `get` checks; refuses non-shart service URLs. |
| `just live-full-guard` | Validate that the effective live-test environment is the complete shart stack. |
| `just live-full-cli` | Run guarded shart live CLI business, setup, serve, stdio MCP, parser, and watch coverage. |
| `just live-full-rest` | Run guarded shart live REST health/status, bearer auth, and OAuth metadata coverage. |
| `just live-full-mcp` | Run guarded shart live MCP protocol, resource, prompt, validation, and tool-action coverage. |
| `just live-full-services` | Run guarded shart live per-service action matrix coverage. |
| `just live-full-test` | Run the complete guarded shart live suite. |
| `just build` / `just build-release` | Debug/release Rust builds. |
| `just gen-token` | Generate a random bearer token (`openssl rand -hex 32`). |

## Quality gates

| Recipe | Purpose |
|---|---|
| `just verify` | `fmt-check` + `lint` + `check` + `test`. |
| `just template-check` | Pattern/plugin/schema/template checks. |
| `just pre-release` | Full release-readiness gate (`scripts/pre-release-check.sh`). |
| `just fmt` | Format Rust and TOML. |
| `just fmt-check` | Check formatting (CI). |
| `just lint` | `cargo clippy -- -D warnings`. |
| `just test` | `cargo nextest run`. |
| `just test-ci` | `cargo nextest run --profile ci`. |
| `just fmt-toml` | `taplo format`. |
| `just check-toml` | `taplo check` (CI). |

## Deployment recipes

| Recipe | Purpose |
|---|---|
| `just docker-build` | Build Docker image. |
| `just docker-up` / `just docker-down` | Start/stop compose stack. |
| `just docker-rebuild` | Rebuild image and recreate Docker service. |
| `just docker-logs` | Follow container logs. |
| `just runtime-current` | Detect stale running runtime (Docker or systemd). |
| `just auth-smoke` | Test bearer auth path against running server. |
| `just test-mcporter` | Run live MCP integration tests. |
| `just repair` | Rebuild and restart via systemd or Docker when available. |

## Plugin and xtask recipes

| Recipe | Purpose |
|---|---|
| `just build-plugin` | Copy release binary into `bin/` and plugin `bin/`. |
| `just validate-plugin` | Validate Claude/Codex/Gemini plugin manifests and skills. |
| `just dist` | `cargo xtask dist` — build and copy release artifacts. |
| `just ci` | `cargo xtask ci` — run all checks locally. |
| `just symlink-docs` | `cargo xtask symlink-docs` — sync `AGENTS.md`/`GEMINI.md` symlinks. |
| `just check-env` | `cargo xtask check-env` — validate required environment. |
| `just patterns` | `cargo xtask patterns` — check architecture contracts. |

## Reference docs

```just
refresh-docs:           bash scripts/refresh-docs.sh
refresh-docs-repomix:   bash scripts/refresh-docs.sh --skip-crawl
refresh-docs-crawl:     bash scripts/refresh-docs.sh --skip-repomix
refresh-docs-dry:       bash scripts/refresh-docs.sh --dry-run
```

## Doctor output

```
$ rustarr doctor

rustarr-mcp v0.1.0 — environment check

  Config
  ──────────────────────────────────────────
  ✓ Config file:       ~/.rustarr/config.toml
  ✓ Data directory:    ~/.rustarr/ (writable)
  ✓ Binary in PATH:    /home/user/.local/bin/rustarr

  Service credentials
  ──────────────────────────────────────────
  ✓ RUSTARR_SERVICES:  sonarr,radarr (set)
  ✗ RUSTARR_SONARR_URL: not set
    → Set RUSTARR_SONARR_URL in ~/.rustarr/.env

  Connectivity
  ──────────────────────────────────────────
  ✓ Upstream reachable: https://rustarr.internal/api → 200 OK (42 ms)

  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  1 issue found. Fix it before running: rustarr serve
```

Exit code 0 = ready. Exit code 1 = issues found.

See `docs/PATTERNS.md` §48 for the full doctor command pattern.
