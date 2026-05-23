---
title: "Philosophy"
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

# Philosophy

`rustarr` exists to make new MCP servers safe, boring, and easy for agents to operate.

## Boring by design

- One binary.
- One HTTP port.
- One action-dispatch MCP tool.
- Clear layering between client, service, and transport shims.
- Repeatable scripts and release gates.

New servers from this template should be easy to understand, audit, and extend — not clever.

## Thin shims, rich service layer

MCP, REST, and CLI code should parse inputs and delegate. Validation, transformation, and business decisions belong in `RustarrService`:

```
MCP shim   → parse JSON args     → service.method()  → return Value
CLI shim   → parse argv          → service.method()  → format/print
REST shim  → parse HTTP body     → service.method()  → return JSON
```

Zero business logic in shims. If you're writing validation in `mcp/tools.rs`, move it to `app.rs`.

## Secure defaults

- `.env` is ignored and blocked from commits by `scripts/block-env-commits.sh`.
- Non-loopback HTTP requires auth unless explicitly behind a trusted gateway (`RUSTARR_NOAUTH=true`).
- Secrets in plugin settings must be marked `sensitive: true`.
- Plugin manifests do not carry version fields — marketplace versioning comes from git SHA/tags.
- Never hard-code tokens in unit files or documentation.

## Agent-first outputs

Agents have finite context windows. All outputs must be:

- **Bounded** — 10K token cap, truncation with a clear message
- **Structured** — stable JSON shapes that don't change between versions
- **Paginated** — every list action supports `limit` and `offset`
- **Self-describing** — `action="help"` always available, no auth required

Error messages must be correctable: state what failed, the bad value, why it failed, and the next command to run.

## Tests prove meaning

A good test proves the returned data is correct. Rustarrs:
- `echo` must return the exact message.
- `greet(name="Alice")` must include the name `Alice` in the response.
- Resource tests must inspect schema content, not just check that `resources/read` returned HTTP 200.

A test that only checks `is_error: false` proves nothing about the service.

## Glass house, not black box

Every server must expose its internal state:
- `/health` — fast liveness, always public
- `/status` — redacted runtime state, always public
- `action="status"` — same data via MCP for clients that can't call HTTP directly
- Structured tracing on every upstream call
- Atomic counters for requests, errors, upstream calls

Operators and agents should never have to guess what the server is doing.

## Surface parity

Every business action reachable from MCP must also be reachable from the CLI. The service layer is called identically from both surfaces — no logic is duplicated, no behavior diverges. Because both shims call the same `RustarrService` methods, parity is automatic when the shims are complete.

Allowed exceptions — documented in the parity table in `CLAUDE.md`:
- MCP-only protocol interactions (elicitation, resources, prompts) have no CLI equivalent by design.
- CLI-only operational commands (`serve`, `mcp`, `doctor`, `watch`, `setup`) are infrastructure, not business actions.

REST API and Web UI are required only for application/platform servers that own meaningful state or non-MCP consumers. A thin upstream-client MCP server does not need a local REST API.

## Thin shim rule — absolute

The prohibition on business logic in shims is the hardest rule to enforce and the most commonly violated:

| Layer | Allowed | Prohibited |
|---|---|---|
| `<service>.rs` | HTTP requests, response parsing | Defaults, validation, error messages |
| `app.rs` | Everything | Nothing |
| `mcp/tools.rs` | Parse args, call service, return Value | Any conditional logic, defaults |
| `cli.rs` | Parse flags, call service, format output | Any conditional logic, defaults |
| `main.rs` | Config loading, mode dispatch | Any domain logic |

Signs of a violation: an `if` in `tools.rs` that isn't arg parsing; a default value set in `cli.rs`; a domain error message in `tools.rs`; any `match` in `cli.rs` beyond `cmd → service.method()`.

## Graceful degradation

The MCP server must stay running and return useful responses even when the upstream service is unavailable. Never crash on upstream failures.

1. **MCP server UP, upstream DOWN** — return `CallToolResult::error()`, not panics
2. **Partial failures** — return what succeeded, mark what failed
3. **Startup with bad config** — warn, don't crash (except security violations)
4. **Upstream timeouts** — fail fast with a clear error, suggest `action=status`

MCP tool errors must use `CallToolResult::error()`, not `Err(ErrorData)`. An `Err` crashes the tool call at the protocol level; a `CallToolResult::error` gives the agent a readable, actionable message.

## Destructive action protection

Any action that can cause data loss MUST require explicit confirmation before proceeding. Use MCP elicitation when the client supports it; fall back to a `confirm=true` parameter and an `ALLOW_DESTRUCTIVE` env var.

Actions that require confirmation: any `delete_*`, `remove_*`, `destroy_*`, `wipe_*`, or any action that overwrites data without rollback or sends irreversible notifications.

## Binary owns its setup

Plugin hooks must be thin adapters. The durable setup behavior belongs in the service binary so hooks, manual repair, tests, and docs all exercise the same code path:

```
plugin-setup.sh  →  <binary> setup plugin-hook
```

The hook script maps env vars and calls the binary. The binary runs `setup check`, optionally `setup repair`, and returns a structured JSON report. Advisory failures exit 0 and don't block Claude Code SessionStart. Blocking failures exit nonzero.

## Three-tier skill fallback

Every server's Claude Code skill covers three fallback tiers in order:

1. **MCP tool** — preferred; full schema, scope enforcement, structured errors
2. **CLI binary** — when MCP is unavailable but the binary is installed
3. **Direct API** — last resort; `curl` with the upstream credentials

This ensures an agent can always reach the service regardless of which tooling is configured.

## Fail fast, validate early

Run pre-flight validation before binding the TCP listener. The `doctor` command reports every missing credential, unreachable upstream, and misconfigured path — with a one-line hint for each — before the user tries to start the server. Exit code 1 means not ready; exit code 0 means go.

Never silently start in a degraded state when a required credential is missing. Warn loudly or refuse to start.

## Small focused modules

Files should be easy to read in one sitting. When a file approaches 400 lines, split it. When a function approaches 100 lines, extract. One responsibility per file; no catch-all `utils.rs` or `helpers.rs`. `mod.rs` files are banned — use named module files.

See `docs/PATTERNS.md` for the full catalog of patterns and their implementation details.
