---
title: "Documentation"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: true
last_reviewed: "2026-05-15"
---

# Documentation

This repo keeps documentation close to the automation it describes. Every file in `docs/` carries YAML frontmatter that describes its role, audience, and authority.

## Directory tree

```
docs/
  ├── DOCS.md                         ← this file; doc directory index
  ├── CLAUDE.md                       ← agent/contributor instructions (source)
  ├── AGENTS.md                       ← symlink → CLAUDE.md (Codex CLI)
  ├── GEMINI.md                       ← symlink → CLAUDE.md (Gemini CLI)
  ├── README.md                       ← docs index listing all files
  │
  ├── PATTERNS.md                     ← canonical pattern catalog (normative)
  ├── MCP_SCHEMA.md                   ← MCP action/scope/schema contract
  ├── MCP-REGISTRY-PUBLISH-GUIDE.md  ← how to publish to the MCP registry
  │
  ├── QUICKSTART.md                   ← 5-minute getting-started guide
  ├── PHILOSOPHY.md                   ← design principles
  ├── AGENTS-FIRST.md                 ← agent-first output and error design
  │
  ├── ARCHITECTURE.md                 ← module layout, layers, AppState
  ├── API.md                          ← HTTP endpoints, REST dispatch, errors
  ├── AUTH.md                         ← bearer tokens, OAuth, auth policy
  ├── CONFIG.md                       ← config.toml vs .env split, loading
  ├── ENV.md                          ← environment variable reference
  ├── OBSERVABILITY.md                ← /health, /status, logging, tracing
  │
  ├── DEPLOYMENT.md                   ← deployment modes overview
  ├── DOCKER.md                       ← Dockerfile, compose, entrypoint
  ├── SYSTEMD.md                      ← user-level systemd service
  │
  ├── PLUGINS.md                      ← Claude/Codex/Gemini plugin packaging
  ├── WEB.md                          ← embedded Next.js web UI
  │
  ├── CI.md                           ← GitHub workflows, nextest, taplo
  ├── PRE-COMMIT.md                   ← lefthook hooks, taplo, env guard
  ├── TESTING.md                      ← test strategy, sidecars, mcporter
  ├── MCPORTER.md                     ← live MCP integration testing
  ├── RUST.md                         ← Rust build setup: system tools, global config, per-repo rules
  ├── XTASKS.md                       ← cargo xtask commands
  ├── JUSTFILE.md                     ← just recipes reference
  ├── SCRIPTS.md                      ← scripts/ directory reference
  │
  ├── plans/                          ← durable implementation plans (transient)
  ├── reports/                        ← audits, investigations, reviews (transient)
  ├── research/                       ← research notes (transient)
  ├── sessions/                       ← saved session logs (transient)
  └── references/                     ← auto-fetched upstream docs (gitignored)
      ├── INDEX.md
      ├── CHANGES.md
      └── mcp/
```

## What goes where

| Location | What belongs there |
|---|---|
| `docs/*.md` | Stable orientation, architecture narrative, and how-to guides. The map, not the territory. |
| `docs/PATTERNS.md` | Normative patterns for the entire rmcp server family. Deviation requires an explicit recorded decision. |
| `docs/plans/` | Durable task breakdowns for in-progress work. Transient — clean up when work lands. |
| `docs/reports/` | Audits, investigations, and review results. Transient. |
| `docs/research/` | Research notes and evidence gathered during investigations. Transient. |
| `docs/sessions/` | Saved session logs and handoff records written by `just save-session`. Transient. |
| `docs/references/` | Auto-fetched upstream docs (MCP spec, registry, upstream repos). **Gitignored.** Run `just refresh-docs` to populate. |

Working artifacts (plans, reports, research, sessions) inform but do not override the stable docs in `docs/*.md`. Accepted requirements from working artifacts should be promoted into the appropriate stable guide.

## Frontmatter schema

Every `docs/*.md` file opens with YAML frontmatter:

```yaml
---
title: "Human-readable title"
doc_type: "guide"          # guide | contract | spec | session | report
status: "active"           # active | draft | deprecated
owner: "rustarr"     # repo name or team
audience:
  - "contributors"
  - "agents"
scope: "template"          # template | service | family
source_of_truth: false     # true only when this file IS the canonical record
upstream_refs:             # optional: where authoritative info lives
  - "src/config.rs"
last_reviewed: "2026-05-15"
---
```

### Field meanings

| Field | Values | Purpose |
|---|---|---|
| `doc_type` | `guide`, `contract`, `spec`, `session`, `report` | Classifies the file's role in the doc hierarchy. |
| `status` | `active`, `draft`, `deprecated` | `active` = current and maintained; `draft` = in progress; `deprecated` = superseded by another file. |
| `source_of_truth` | `true` / `false` | `true` only when this file IS the authoritative record. Most guides are `false` — they summarize the code or reference `PATTERNS.md`. When a doc disagrees with `source_of_truth: true` code, update the doc. |
| `upstream_refs` | file paths | Where to go when this doc and reality diverge. Code files beat docs. |
| `scope` | `template`, `family`, `service` | `template` = this repo only; `family` = normative across all rmcp servers; `service` = only relevant after template adaptation. |

### CLAUDE.md / AGENTS.md / GEMINI.md

`docs/CLAUDE.md` carries agent and contributor instructions for navigating this directory. It is `source_of_truth: false` because the code itself is authoritative; the file explains structure and conventions, not behavior.

`docs/AGENTS.md` and `docs/GEMINI.md` are symlinks to `docs/CLAUDE.md` — they exist so Codex CLI and Gemini CLI find the same instructions Claude Code does. Their frontmatter is identical to `docs/CLAUDE.md` because they are the same file:

```yaml
---
title: "Documentation Instructions"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/references/mcp/"
last_reviewed: "2026-05-14"
---
```

After adding any new `CLAUDE.md` anywhere in the repo, regenerate the symlinks:

```bash
just symlink-docs
# or: cargo xtask symlink-docs
```

## Generated and checked docs

### Schema docs

Regenerate and check MCP schema docs with:

```bash
just schema-docs
just schema-docs-check
```

The checker treats `src/actions.rs::ACTION_SPECS` as canonical. `docs/MCP_SCHEMA.md` must stay in sync with it.

### Reference docs

`docs/references/` is populated by `scripts/refresh-docs.sh` and excluded from git:

```bash
just refresh-docs              # full refresh (crawl + repomix)
just refresh-docs-dry          # dry run, no mutations
just refresh-docs-repomix      # skip crawl, repomix only
just refresh-docs-crawl        # skip repomix, crawl only
```

Run when starting work on a new feature touching the service API, when the upstream service releases a new API version, or monthly to keep reference material current.
