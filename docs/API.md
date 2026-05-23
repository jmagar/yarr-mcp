---
title: "API Reference"
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

# API

The server exposes HTTP endpoints alongside MCP. All surfaces (MCP, REST, CLI) call the same `RustarrService` methods — no logic is duplicated.

## Endpoints

| Endpoint | Method | Auth | Purpose |
|---|---|---|---|
| `/health` | GET | Public | Fast liveness. Returns minimal status. |
| `/status` | GET | Public | Local-only redacted runtime status; see `docs/OBSERVABILITY.md`. |
| `/openapi.json` | GET | Public | Generated REST OpenAPI schema. |
| `/mcp` | POST/stream | Auth policy | Streamable HTTP MCP endpoint. |
| `/v1/rustarr` | POST | Auth policy | REST action dispatch. |

## REST action request

The REST API uses the same `action` + `params` pattern as MCP tools:

```json
{
  "action": "echo",
  "params": {
    "message": "hello"
  }
}
```

`params` may be omitted or empty for no-argument actions.

## REST handler

```rust
// src/api.rs
async fn api_dispatch(
    State(state): State<AppState>,
    auth: Option<Extension<AuthContext>>,
    Json(body): Json<ActionRequest>,
) -> impl IntoResponse {
    let result = match RustarrAction::from_rest(&body.action, &body.params) {
        Ok(action) => {
            if let Some(response) = enforce_rest_scope(
                &state,
                auth.as_ref().map(|Extension(auth)| auth),
                &body.action,
            ) {
                return response;
            }
            execute_service_action(&state.service, &action).await
        }
        Err(error) => Err(error),
    };

    match result {
        Ok(value) => Json(cap_rest_response(value)).into_response(),
        Err(e) if crate::actions::is_validation_error(&e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        ).into_response(),
        Err(e) => {
            tracing::error!(error = %e, action = %body.action, "REST action execution failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal server error"})),
            ).into_response()
        }
    }
}
```

## Surface parity

| Surface | Call pattern |
|---|---|
| MCP | `rustarr(action="greet", name="Alice")` |
| REST | `POST /v1/rustarr {"action":"greet","params":{"name":"Alice"}}` |
| CLI | `rustarr greet --name Alice` |

All three call `state.service.greet(Some("Alice"))`.

## Response shapes

```json
{"status":"ok"}
```

```json
{"echo":"hello"}
```

Responses are JSON values produced by `RustarrService` via `src/actions.rs`.
If a REST action result exceeds the response cap, the route returns a valid JSON
truncation envelope instead of raw truncated JSON.

## MCP-only actions

Some actions require MCP client capabilities and are excluded from REST action lists. Elicitation-based actions require a live MCP client interaction. REST `help` returns both `actions` and `mcp_only_actions` so clients can discover the split.

## Agent-first output rules

- No single response may return more than ~10,000 tokens (~40 KB). REST returns a JSON truncation envelope; MCP truncates the serialized tool text.
- List actions MUST support `limit` and `offset` (or `cursor`).
- List actions that return heterogeneous data MUST support `filter` and `state` parameters.
- Every CLI command that outputs data MUST support `--json`.

```rust
const MAX_RESPONSE_BYTES: usize = 40_000; // ~10K tokens

fn truncate_response(text: &str) -> String {
    if text.len() <= MAX_RESPONSE_BYTES {
        return text.to_string();
    }
    let boundary = text
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= MAX_RESPONSE_BYTES)
        .last()
        .unwrap_or(0);
    let truncated = &text[..boundary];
    format!("{truncated}\n\n[TRUNCATED: response exceeded 10K token limit. Use limit/offset or more specific filters.]")
}
```

## Error messages

Errors must be actionable. Every error must say what failed, the bad value, why it failed, and how to fix it:

```rust
Ok(CallToolResult::error(vec![Content::text(format!(
    "ERROR: {action} failed\n\
     Reason: {reason}\n\
     Hint: {how_to_fix}\n\
     See: action=help for full documentation"
))]))
```

Validation errors return HTTP 400 with an `error` field. Never leak secrets in error text.

Common error shapes:
- Missing required arg: `` "`id` is required for docker_logs — pass id=<container_id>" ``
- Unknown action: `"unknown action: \"florp\" — valid actions: greet, echo, status, help"`
- API unreachable: `"RUSTARR_URL unreachable: connection refused — is the service running?"`

See `docs/PATTERNS.md` §A2, §39, §40 for the full REST pattern, error structure, and token discipline.
