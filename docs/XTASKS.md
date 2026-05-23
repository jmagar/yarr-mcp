---
title: "xtasks"
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
| `cargo xtask dist` | Build release binary and copy it to `bin/example`. |
| `cargo xtask ci` | Run local CI sequence: fmt, clippy, tests, taplo, patterns, audit when tools exist. |
| `cargo xtask symlink-docs` | Create `AGENTS.md` and `GEMINI.md` symlinks next to each `CLAUDE.md`. |
| `cargo xtask check-env` | Validate required environment before server start. |
| `cargo xtask patterns` | Check static contracts derived from `docs/PATTERNS.md`. |

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
✓ EXAMPLE_API_URL:   https://example.internal/api (set)
✗ EXAMPLE_API_KEY:   not set
  → Set EXAMPLE_API_KEY in ~/.example/.env or your environment
```

See `docs/PATTERNS.md` §24 and §48 for the xtask and doctor patterns.
