# MCP Schema Contract

Generated from `src/actions.rs` and checked against the schema, README, skill docs, help text, and scope routing.

Run:

```bash
python3 scripts/check-schema-docs.py --write
python3 scripts/check-schema-docs.py --check
```

## Tool

| Field | Value |
|---|---|
| Tool name | `rustarr` |
| Schema resource | `rustarr://schema/mcp-tool` |
| Dispatch parameter | `action` |

## Actions

| Action | Scope | Description |
|---|---|---|
| `integrations` | `rustarr:read` | List supported service kinds and configured service instances. |
| `service_status` | `rustarr:read` | Fetch the service-specific status endpoint for one configured service. |
| `api_get` | `rustarr:write` | Proxy a credentialed GET request to an allowed upstream API prefix. |
| `api_post` | `rustarr:write` | Proxy a confirmed credentialed POST request to an allowed upstream API prefix. |
| `help` | public | Return the in-tool action reference. Public; no scope required. |

## Drift Rules

- `ACTION_SPECS` in `src/actions.rs` is the canonical action and scope list.
- `src/mcp/schemas.rs` must derive its enum from `ACTION_SPECS`.
- The MCP tool schema must reject unknown top-level parameters and encode action-specific requirements that fit the single-tool dispatch model.
- `help` is intentionally public and must have no required scope.
- `src/mcp/tools.rs`, `README.md`, and `plugins/rustarr/skills/rustarr/SKILL.md` must mention every action.
- `src/mcp/rmcp_server.rs` owns stable resources and must keep `rustarr://schema/mcp-tool` wired to `tool_definitions()`.
- `src/mcp/prompts.rs` owns stable prompts and must keep `quick_start` covered by prompt tests.

## Resources

| URI | Source | Contract |
|---|---|---|
| `rustarr://schema/mcp-tool` | `src/mcp/rmcp_server.rs` | Returns `tool_definitions()` as `application/json`. |

## Prompts

| Prompt | Source | Contract |
|---|---|---|
| `quick_start` | `src/mcp/prompts.rs` | Guides a client to inspect configured integrations and, when available, fetch one service status. |

## Input Validation

- `action` is always required.
- `service_status` conditionally requires non-empty `service`.
- `api_get` conditionally requires non-empty `service` and `path`.
- `api_post` conditionally requires non-empty `service`, `path`, and `confirm=true`; `body` defaults to `{}`.
- Unknown top-level parameters are rejected by the schema.
