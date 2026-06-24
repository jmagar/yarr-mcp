---
title: "Observability"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-05-15"
---

# Observability

The template exposes fast, redacted status surfaces for humans, agents, and deployment automation. Design principle: glass house, not black box.

## HTTP endpoints

| Endpoint | Auth | Purpose |
|---|---|---|
| `GET /health` | Public | Fast liveness. |
| `GET /ready` | Public | Local readiness; reports whether at least one upstream service is configured. |
| `GET /status` | Public | Local redacted runtime metadata. |
| `GET /metrics` | Bearer | Prometheus-compatible metrics (optional). |
| `/mcp` | Auth policy | MCP Streamable HTTP endpoint. |

`/health` must remain fast (no database or upstream calls). Use `/ready` for local configuration readiness and authenticated tool actions for upstream status.

## /health response shape

```json
{
  "status": "ok"
}
```

## /ready response shape

```json
{
  "status": "ready",
  "configured_services": 3
}
```

## /status response shape

```json
{
  "status": "ok",
  "server": "rustarr-mcp",
  "version": "0.1.0",
  "transport": "http"
}
```

Omit secrets, credentials, upstream URLs, and upstream health details from the public route.

## MCP status actions

`action="service_status"` is a read-scoped business action that calls one configured upstream service's status endpoint. Keep it redacted, but do not assume it has the same contract as the public `/status` route.

## Logging

Two destinations simultaneously — console and file:

| Destination | Format | Writer |
|---|---|---|
| Console (stderr) | Human-readable, Aurora colors | `tracing_subscriber::fmt` with `AuroraFormatter` |
| File (`~/.rustarr/logs/rustarr.log`) | Structured JSON | `tracing_subscriber::fmt::json()` |

Use `RUST_LOG` to control log level:

```bash
RUST_LOG=info,rmcp=warn rustarr serve
```

Log file: one file, 10 MB cap. On overflow, truncate and restart. Never multiple files.

Aurora console color palette (ANSI 256): `SERVICE_NAME=211` (pink), `ACCENT_PRIMARY=39` (blue), `SUCCESS=115` (teal), `WARN=180` (amber), `ERROR=174` (muted red). Respect `NO_COLOR`; force color with `FORCE_COLOR`.

Console log format:

```
2026-05-13T14:32:05Z  INFO  MCP tool call  tool=yarr  action=codemode  elapsed_ms=12
2026-05-13T14:32:15Z ERROR  upstream failed  action=api_get  service=sonarr  error="connection refused"
```

File log format:

```json
{"timestamp":"2026-05-13T14:32:05Z","level":"INFO","message":"MCP tool call","tool":"yarr","action":"codemode","elapsed_ms":12}
```

## Tracing spans

Wrap every external call:

```rust
async fn list_things(&self) -> Result<Value> {
    let span = tracing::info_span!("upstream.list_things");
    let _guard = span.enter();
    tracing::debug!(url = %self.base_url, "calling upstream");
    let result = self.client.list_things().await;
    match &result {
        Ok(v)  => tracing::debug!(count = v.as_array().map(|a| a.len()).unwrap_or(0), "ok"),
        Err(e) => tracing::warn!(error = %e, "upstream call failed"),
    }
    result
}
```

## Runtime freshness

`just runtime-current` checks whether the running Docker/systemd instance matches the current artifact. Use it after deploys and when debugging stale behavior.

## Agent-first diagnostics

Errors must say what failed, what was expected, and the next command to run. Avoid opaque `internal error` responses. See `docs/PATTERNS.md` §39 and §40 for error structure and token-discipline patterns.
