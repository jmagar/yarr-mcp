---
title: "Justfile"
doc_type: "guide"
status: "active"
owner: "rmcp-template"
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
| `just dev` | Run HTTP MCP server on loopback in no-auth dev mode (`EXAMPLE_MCP_NO_AUTH=true`). |
| `just mcp` | Run stdio MCP transport (`example mcp`). |
| `just greet` | Quick CLI smoke test. |
| `just doctor` | Pre-flight environment/connectivity checks (`example doctor`). |
| `just build` / `just build-release` | Debug/release Rust builds. |
| `just build-web` | Build static Next.js web assets (`apps/web/out`). |
| `just build-full` | Build web assets then release binary (CI use). |
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
$ example doctor

example-mcp v0.1.0 — environment check

  Config
  ──────────────────────────────────────────
  ✓ Config file:       ~/.example/config.toml
  ✓ Data directory:    ~/.example/ (writable)
  ✓ Binary in PATH:    /home/user/.local/bin/example

  Service credentials
  ──────────────────────────────────────────
  ✓ EXAMPLE_API_URL:   https://example.internal/api (set)
  ✗ EXAMPLE_API_KEY:   not set
    → Set EXAMPLE_API_KEY in ~/.example/.env

  Connectivity
  ──────────────────────────────────────────
  ✓ Upstream reachable: https://example.internal/api → 200 OK (42 ms)

  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  1 issue found. Fix it before running: example serve
```

Exit code 0 = ready. Exit code 1 = issues found.

See `docs/PATTERNS.md` §48 for the full doctor command pattern.
