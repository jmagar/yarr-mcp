---
title: "xtasks"
doc_type: "guide"
status: "active"
owner: "yarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
last_reviewed: "2026-05-15"
---

# xtasks

The `xtask/` crate contains typed repo automation invoked as `cargo xtask <command>`. Shell scripts are good for glue; xtasks are better when the check needs structured parsing, walking the repo, or cross-platform behavior.

## Repository layout

```
xtask/
  Cargo.toml    # name = "xtask", path dep on main crate
  src/
    main.rs     # cargo xtask <command>
```

## Commands

| Command | Purpose |
|---|---|
| `cargo xtask dist` | Build release binary and copy it to `bin/yarr`. |
| `cargo xtask ci` | Run local CI sequence: fmt, clippy, tests, taplo, patterns, audit when tools exist. |
| `cargo xtask symlink-docs` | Create `AGENTS.md` and `GEMINI.md` symlinks next to each `CLAUDE.md`. |
| `cargo xtask check-env` | Validate required environment before server start. |
| `cargo xtask patterns` | Check static contracts derived from `docs/PATTERNS.md`. |
| `cargo xtask live --suite all` | Run the guarded shart-only live CLI, REST, MCP, and upstream service suite. |
| `cargo xtask tool-docs` | Generate `docs/TOOLS_ACTIONS_ENDPOINTS.md` from the action registry and endpoint mapping table. |

## Justfile delegates to xtask

```just
dist:
    cargo xtask dist
symlink-docs:
    cargo xtask symlink-docs
```

## Pattern checks

`cargo xtask patterns` verifies important architecture contracts:

- required template files exist
- no `mod.rs` files
- file size warnings and hard limits
- MCP/CLI shims remain thin (no business logic)
- action surfaces stay represented in schemas, help text, tests, and CLI
- routes, plugin manifests, auth config, and tooling hooks exist

`cargo xtask patterns --strict` treats warnings as failures.

## Tool Reference Generation

`cargo xtask tool-docs` regenerates `docs/TOOLS_ACTIONS_ENDPOINTS.md`.
The generator reads action names, params, scopes, and mutability from the Rust
action registry and renders endpoint mappings from the structured table in
`xtask/src/tool_docs.rs`.

```bash
cargo xtask tool-docs
cargo xtask tool-docs --check
```

### What the pattern checker catches

```
WARN  src/mcp/tools.rs  line 42: potential business logic in MCP shim
WARN  src/cli.rs  line 87: potential business logic in CLI shim
ERROR src/app/mod.rs: mod.rs files are banned
ERROR src/mcp/tools.rs: action "new_action" in ACTION_SPECS missing from dispatch
ERROR tests/tool_dispatch.rs: action "new_action" has no test
```

## symlink-docs

`cargo xtask symlink-docs` keeps `AGENTS.md` and `GEMINI.md` in sync with `CLAUDE.md` across every directory that has a `CLAUDE.md`:

```bash
find . -name "CLAUDE.md" -not -path "./.git/*" -not -path "./target/*" | while read f; do
    dir=$(dirname "$f")
    ln -sf "CLAUDE.md" "${dir}/AGENTS.md"
    ln -sf "CLAUDE.md" "${dir}/GEMINI.md"
done
```

Run `just symlink-docs` after adding any new `CLAUDE.md` file.

## check-env

`cargo xtask check-env` reports missing or misconfigured environment before startup:

```
✓ YARR_SERVICES:  sonarr,radarr (set)
✗ YARR_SONARR_URL: not set
  → Set YARR_SONARR_URL in ~/.yarr/.env or your environment
```

See `docs/PATTERNS.md` §24 and §48 for the xtask and doctor patterns.

## live

`cargo xtask live` is the canonical full live integration harness. It refuses to
run unless the effective Yarr configuration is the dedicated shart test
environment at `/home/jmagar/.yarr-shart` and every configured service URL
points at shart.

```bash
cargo xtask live --suite guard
cargo xtask live --suite cli
cargo xtask live --suite rest
cargo xtask live --suite mcp
cargo xtask live --suite mcporter
cargo xtask live --suite services
cargo xtask live --suite all
```

The full suite covers the live guard, CLI business commands, CLI infrastructure
commands, REST health/status/auth/OAuth metadata routes, MCP initialize/tools/
resources/prompts/tool calls, every generated OpenAPI callable through
mcporter/yarr, and every configured service matrix action. It writes
`target/live-full/report.json` with one semantic check record per executed
assertion.

Unless `YARR_BIN` is set, the live xtask builds and runs
`target/debug/yarr` from the current checkout. Use `YARR_BIN` only when
intentionally testing a specific prebuilt binary.

The required high-level surface markers live in `xtask/src/live/surface.rs`.
`cargo xtask live --suite all` verifies that every marker is actually recorded
in the report, so future changes cannot accidentally drop a CLI/API/MCP surface
without failing the live run.

Use the Just aliases `just live-full-guard`, `just live-full-cli`,
`just live-full-rest`, `just live-full-mcp`, `just live-full-mcporter`,
`just live-full-services`, and `just live-full-test` for the same commands.
