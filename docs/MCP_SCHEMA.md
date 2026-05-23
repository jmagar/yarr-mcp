# rustarr MCP Schema Contract

The MCP schema is generated from `src/actions.rs`; `ACTION_SPECS` is the source of truth.

## Tool

| Field | Value |
|---|---|
| Tool name | `rustarr` |
| Schema resource | `rustarr://schema/mcp-tool` |
| Dispatch parameter | `action` |

## Actions

| Action | Scope | Description |
|---|---|---|
| `integrations` | `rustarr:read` | List supported and configured media services without secrets |
| `service_status` | `rustarr:read` | Fetch a service status endpoint |
| `api_get` | `rustarr:read` | Proxy a safe GET request to a configured upstream |
| `api_post` | `rustarr:write` | Proxy a safe POST request to a configured upstream |
| `help` | public | Return the action reference |

## Drift Rules

- `src/mcp/schemas.rs` derives the action enum from `action_names()`.
- `docs/API.md`, `README.md`, and `plugins/rustarr/skills/rustarr/SKILL.md` must mention every action.
- `help` remains public.
- Write operations require `rustarr:write`.
- The schema resource `rustarr://schema/mcp-tool` must return `tool_definitions()`.

## Input Validation

- `action` is always required.
- `service_status`, `api_get`, and `api_post` require `service`.
- `api_get` and `api_post` require `path`.
- `api_post` requires `body`.
- Paths must be relative and must not include query-string secrets.
