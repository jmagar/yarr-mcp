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

# Documentation Instructions

> For the architecture / module map / auth model, see the repository-root `CLAUDE.md`.

This directory contains guides, reference material, and working records for the rustarr project and the Rust MCP server family it governs.

Both humans and LLM agents operate this codebase. Write docs, contracts, specs, rustarrs, and commands assuming both audiences. Prefer structured, runnable, and self-contained content. Avoid prose that only makes sense in context of a prior conversation.

---

## Documentation Layers

Use the right layer for the job:

- `docs/*.md` — Orientation, architecture narrative, cross-cutting guidance, and stable how-to guides. These are the map.
- `docs/references/mcp/` — Snapshots of the official MCP specification, registry, and tooling documentation. Treat as the authoritative source for MCP protocol behavior at the captured revision.

There are no `contracts/`, `specs/`, or `reports/` directories yet. If durable implementation contracts or investigation reports are added, create those directories and record their authority in this file.

---

## Files in This Directory

| File | Purpose | Update when |
|---|---|---|
| `QUICKSTART.md` | Five-minute getting-started guide | The startup sequence, CLI commands, or port changes |
| `AUTH.md` | Auth model: bearer tokens, OAuth, startup guard, gateway case | Auth behavior or env vars change |
| `PATTERNS.md` | Canonical patterns for the entire rmcp server family | The module structure, thin-shim rule, or family-wide conventions change |
| `MCP-REGISTRY-PUBLISH-GUIDE.md` | How to publish a derived server to the official MCP registry | The mcp-publisher CLI, registry schema, or CI publish workflow changes |
| `CLAUDE.md` (this file) | Instructions for agents and contributors navigating this directory | The directory structure or doc authority changes |

---

## References

`docs/references/mcp/` contains snapshots of the MCP specification, SEPs, registry docs, and tooling references. Treat these as the source-of-truth for MCP protocol behavior as captured in this repo.

- Prefer `docs/references/mcp/` before web search when implementing or verifying MCP protocol behavior.
- If the captured reference is suspected stale or ambiguous for a fast-moving spec area (elicitation, extensions, registry preview), verify against the upstream source before changing behavior.
- When upstream marks material as `preview`, `draft`, `proposal`, `RFD`, or `SEP`, mirror that status in any derived docs.

Do not treat seed transcripts or conversation context as sufficient evidence for what the spec requires. If spec behavior matters, cite the reference file.

---

## Naming

- The binary and template identifiers use `rustarr` / `Rustarr` / `RUSTARR_` as placeholders. These are renamed when the template is adapted.
- The pattern family is `rmcp-server`. Member servers include `lab`, `axon_rust`, `syslog-mcp`, `rustify`, `rustifi`, `apprise-mcp`, `rustscale`, `unrust`, and this template.
- Do not rewrite captured reference snapshots or upstream repopacks to match current naming. Those files preserve provenance.

---

## Template Adaptation

This repo is a template. Every doc in this directory contains `TEMPLATE:` markers where values must be changed when the template is adapted for a real service. When editing docs:

- Keep template markers in place unless you are explicitly adapting the template, not just editing it.
- Do not remove the `TEMPLATE:` sections from `PATTERNS.md` — they govern the entire family.
- The `PATTERNS.md` patterns are normative across all family members. Deviation requires an explicit decision recorded in that repo.

---

## Working Artifact Directories

There are none yet. If you add them:

- `docs/plans/` — durable implementation plans and task breakdowns.
- `docs/reports/` — audits, investigations, review results.
- `docs/sessions/` — saved session notes and handoff records.

Artifacts in those directories inform but do not override the stable docs in `docs/*.md`. If a working artifact contains an accepted requirement, promote it into the appropriate stable doc.

---

## Style

- Short, direct sections with clear ownership.
- Rustarrs should be runnable as written. Verify port numbers, command names, and flag names against the code before committing.
- Keep generated or historical material out of guides. If something belongs in a guide, distill it; don't paste.
- Do not move broad architecture into narrow docs only. Top-level docs should remain the map.
- Env var names are authoritative in `src/config.rs`. If a doc disagrees with the code, update the doc.
