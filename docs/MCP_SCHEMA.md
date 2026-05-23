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
| Tool name | `example` |
| Schema resource | `example://schema/mcp-tool` |
| Dispatch parameter | `action` |

## Actions

| Action | Scope | Description |
|---|---|---|
| `greet` | `example:read` | Return a greeting. Optional `name` string. |
| `echo` | `example:read` | Echo a required `message` string. |
| `status` | `example:read` | Return server status and configuration summary. |
| `elicit_name` | `example:read` | Ask the MCP client to elicit a name and return a personalized greeting. |
| `scaffold_intent` | `example:read` | Elicit scaffold requirements and return JSON for the scaffold-project skill. Does not mutate files. |
| `help` | public | Return the in-tool action reference. Public; no scope required. |

## Drift Rules

- `ACTION_SPECS` in `src/actions.rs` is the canonical action and scope list.
- `src/mcp/schemas.rs` must derive its enum from `ACTION_SPECS`.
- The MCP tool schema must reject unknown top-level parameters and encode action-specific requirements that fit the single-tool dispatch model.
- `help` is intentionally public and must have no required scope.
- `src/mcp/tools.rs`, `README.md`, and `plugins/example/skills/example/SKILL.md` must mention every action.
- `src/mcp/rmcp_server.rs` owns stable resources and must keep `example://schema/mcp-tool` wired to `tool_definitions()`.
- `src/mcp/prompts.rs` owns stable prompts and must keep `quick_start` covered by prompt tests.

## Resources

| URI | Source | Contract |
|---|---|---|
| `example://schema/mcp-tool` | `src/mcp/rmcp_server.rs` | Returns `tool_definitions()` as `application/json`. |

## Prompts

| Prompt | Source | Contract |
|---|---|---|
| `quick_start` | `src/mcp/prompts.rs` | Guides a client to call `status` and `greet`. |

## Input Validation

- `action` is always required.
- `echo` conditionally requires non-empty `message`.
- `greet` accepts optional `name` and defaults to World.
- `elicit_name` and `scaffold_intent` collect their extra fields through MCP elicitation, not direct tool-call arguments.
- Unknown top-level parameters are rejected by the schema.
