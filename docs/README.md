# Documentation index

This directory contains focused guides for operating, adapting, testing, and releasing an `yarr`-derived MCP server.

## Start here

| Doc | Use when you want to... |
|---|---|
| `QUICKSTART.md` | Get the template running quickly. |
| `ARCHITECTURE.md` | Understand the Rust module layout and layering rules. |
| `PHILOSOPHY.md` | Understand the design principles behind the template. |
| `AGENTS-FIRST.md` | Build outputs and workflows that are reliable for AI agents. |
| `PATTERNS.md` | Read the canonical long-form pattern catalog. |

## Operations

| Doc | Covers |
|---|---|
| `DEPLOYMENT.md` | End-to-end deployment checklist. |
| `DOCKER.md` | Docker image and Compose operations. |
| `SYSTEMD.md` | User systemd deployment and runtime freshness checks. |
| `CONFIG.md` | Configuration structure and auth policy summary. |
| `ENV.md` | Environment variable reference. |
| `OBSERVABILITY.md` | Health/status endpoints, logging, runtime checks. |

## Development and quality

| Doc | Covers |
|---|---|
| `JUSTFILE.md` | `just` recipes and local operator commands. |
| `XTASKS.md` | `cargo xtask` automation. |
| `PRE-COMMIT.md` | Lefthook and fast local guardrails. |
| `CI.md` | Local CI parity and release gates. |
| `TESTING.md` | Rust tests, route tests, live MCP tests. |
| `MCPORTER.md` | Live MCP tool/resource testing and CLI generation. |
| `SCRIPTS.md` | Script categories and maintenance contract. |
| `DOCS.md` | Documentation generation and references. |

## Surfaces

| Doc | Covers |
|---|---|
| `API.md` | REST and HTTP endpoints. |
| `WEB.md` | Optional static Next.js web UI. |
| `AUTH.md` | Auth policies and security model. |
| `PLUGINS.md` | Claude/Codex/Gemini plugin packaging. |
| `MCP_SCHEMA.md` | Generated MCP tool schema/action contract. |
| `MCP-REGISTRY-PUBLISH-GUIDE.md` | MCP registry publishing guidance. |

## Directories

| Directory | Contents |
|---|---|
| `sessions/` | Saved session notes and agent handoff records. |
| `references/` | Snapshots of external specifications and tooling docs (MCP spec, registry, etc.). |

## Keeping docs current

- Update focused docs when changing commands, scripts, routes, deployment, or plugin behavior.
- Update `docs/PATTERNS.md` when a reusable repo-family pattern changes.
- Regenerate schema docs after action changes:
  ```bash
  just schema-docs
  ```
- Refresh ignored external references when needed:
  ```bash
  scripts/refresh-docs.sh
  ```
- Validate before pushing:
  ```bash
  scripts/pre-release-check.sh
  ```
