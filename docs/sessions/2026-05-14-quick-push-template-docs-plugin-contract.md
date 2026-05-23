---
date: 2026-05-14 10:23:35 EDT
repo: git@github.com:jmagar/rmcp-template.git
branch: refactor/server-api-module-split
head: e8fa418
working directory: /home/jmagar/workspace/rmcp-template
---

# Quick Push: Template Docs And Plugin Setup Contract

## User Request

Run `quick-push` after the rmcp-template automation and documentation porting work.

## Session Overview

Reviewed the dirty worktree, fixed stale port and plugin setup contract drift, ran focused template verification, then committed and pushed the full intentional worktree to `refactor/server-api-module-split`.

The pushed commit was:

```text
e8fa418 fix: sync template docs and plugin setup contract
```

Remote `origin/refactor/server-api-module-split` matched local `HEAD` after the push.

## What Changed

| Area | Change |
|------|--------|
| Template port contract | Updated docs and comments from older `3000`/`3100` examples to the current `40060` template port where the reference was specific to rmcp-template. |
| Plugin manifests | Updated Claude and Gemini local server defaults to `http://localhost:40060`. |
| Plugin validator | Updated `scripts/validate-plugin-layout.sh` so it enforces the new Claude plugin server URL default. |
| Setup hook behavior | Updated `src/cli.rs` so `setup repair` writes `.env` before re-running `setup check`, and no-repair mode reports a missing `.env` as an advisory failure rather than silently succeeding. |
| Plugin contract tests | Updated `tests/plugin_contract.rs` to assert the new advisory `env_file_missing` behavior and to use port `0` for test isolation. |
| AI instruction mirrors | Added `AGENTS.md` and `GEMINI.md` symlinks next to nested `CLAUDE.md` files in `apps/web`, `docs`, and `plugins/example`. |
| Plugin distribution layout | Added `plugins/example/bin/.gitkeep` so the plugin binary directory exists in git. |

## Verification

Ran before committing:

```bash
just template-check
cargo test --test plugin_contract --test template_invariants
git diff --check
```

Results:

- `just template-check` passed after updating the plugin validator's expected `server_url`.
- `cargo test --test plugin_contract --test template_invariants` passed: 11 tests total.
- `git diff --check` passed.

## Commit And Push

Commands:

```bash
git add .
git commit -m "fix: sync template docs and plugin setup contract"
git push origin refactor/server-api-module-split
```

Push result:

```text
3b2c120..e8fa418  refactor/server-api-module-split -> refactor/server-api-module-split
```

Post-push verification:

```bash
git rev-parse HEAD
git ls-remote origin refs/heads/refactor/server-api-module-split
```

Both resolved to:

```text
e8fa41804e84f22e91689301783536e66e6aacdc
```

## Current Worktree After This Note

At the time this note was written, there was new uncommitted work created after the `e8fa418` push:

```text
 M docs/PLUGINS.md
?? scripts/check-plugin-hook-contract.py
```

Those changes were intentionally not included in the original quick-push commit. They appear to add a cross-repo plugin hook contract checker and documentation for the `no_repair` JSON field.

## Open Questions

- Should `scripts/check-plugin-hook-contract.py` become part of the template's default CI or stay as an operator-run cross-repo audit tool?
- Should the new plugin hook checker be added to `scripts/README.md`, `README.md`, `Justfile`, and `just template-check` if it remains in this repo?
- Should the checker execute binaries by default only for rmcp-template, or keep `--execute` opt-in for all family repos because it may compile and run several workspaces?
