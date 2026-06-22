# MCP Schema Contract

Generated from `src/actions/` and checked against the schema, README, skill docs, help text, and scope routing.

Run:

```bash
python3 scripts/check-schema-docs.py --write
python3 scripts/check-schema-docs.py --check
```

## Tool

| Field | Value |
|---|---|
| Tool names | `sonarr`, `radarr`, `prowlarr`, `overseerr`, `tautulli`, `plex`, `tracearr`, `sabnzbd`, `qbittorrent`, `jellyfin`, `bazarr` |
| Schema resource | `rustarr://schema/mcp-tool` |
| Dispatch parameter | `action`; service is implied by the tool name |

## Actions

| Action | Scope | Description |
|---|---|---|
| `integrations` | `rustarr:read` | List supported service kinds and configured service instances. |
| `service_status` | `rustarr:read` | Fetch the service-specific status endpoint for one configured service. |
| `api_get` | `rustarr:write` | Proxy a credentialed GET request to an allowed upstream API prefix. |
| `api_post` | `rustarr:write` | Proxy a credentialed POST request to an allowed upstream API prefix. |
| `api_put` | `rustarr:write` | Proxy a credentialed PUT request to an allowed upstream API prefix. |
| `api_delete` | `rustarr:write` | Proxy a credentialed DELETE request to an allowed upstream API prefix. |
| `help` | public | Return the in-tool action reference. Public; no scope required. |

## Drift Rules

- `ACTION_SPECS` in `src/actions/registry.rs` is the canonical generic action and scope list; curated commands live in `CURATED_COMMANDS`.
- `src/mcp/schemas.rs` must derive each service tool's enum from `valid_actions_for_kind()` (via the generated `properties`); `src/mcp/schemas/conditionals.rs` generates the action-specific requirements.
- The MCP tool schema must reject unknown top-level parameters and encode action-specific requirements that fit the service-named tool dispatch model.
- `help` is intentionally public and must have no required scope.
- Help text is generated in `src/actions/help.rs` from the registry; `README.md` and `plugins/rustarr/skills/rustarr/SKILL.md` must mention every action.
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
- `service_status` uses the service implied by the tool name.
- `api_get` conditionally requires non-empty `path`.
- `api_post` conditionally requires non-empty `path`; `body` defaults to `{}`. Non-destructive; runs immediately.
- `api_put` conditionally requires non-empty `path`; `body` defaults to `{}`. Non-destructive; runs immediately.
- `api_delete` conditionally requires non-empty `path`; `body` is optional (query params go in `path`). Destructive: gated by MCP elicitation / CLI `--confirm` (or an explicit `confirm=true` override), not a required schema param.
- Unknown top-level parameters are rejected by the schema.
