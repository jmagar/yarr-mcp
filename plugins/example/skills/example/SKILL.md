---
name: example
description: TEMPLATE SKILL — Replace this description with your service's trigger phrases. This skill should be used when the user asks to interact with the Example service via MCP. Trigger phrases should describe what your service does, e.g. "query my service", "check example status", "call example API". The description is what the AI reads to decide when to invoke this skill — be specific and use the exact phrases your users will say.
---

<!-- ==========================================================================
     TEMPLATE: This is a template skill demonstrating the three-tier pattern
     for MCP server documentation skills.

     To adapt for your service:
       1. Update the YAML frontmatter: name, description
       2. Replace "example" with your tool name throughout
       3. Replace example actions (greet, echo, status, help) with your actions
       4. Update the Action Reference to document your actual response shapes
       5. Update the HTTP Fallback section with your service's curl examples

     The three tiers of this skill:
       Tier 1 (above fold) — Tool name, quick action table, critical gotchas.
                             The AI reads this first and stops here for simple queries.
       Tier 2 (middle)     — Full action reference with params and examples.
                             The AI reads this when it needs parameter details.
       Tier 3 (bottom)     — Workflows, HTTP fallback, error handling.
                             The AI reads this for complex multi-step tasks.
     ========================================================================== -->

# Example Skill

<!-- TEMPLATE: Replace this paragraph with your service description -->
Rust-based MCP server template. Exposes a single `example` MCP tool with action-based dispatch for interacting with an example remote service.

## Tool

<!-- TEMPLATE: Replace "mcp__example__example" with your tool's actual MCP name.
     Format: mcp__<server-name>__<tool-name>
     The server name comes from plugin.json "name", the tool name from src/mcp/schemas.rs -->
A single MCP tool, `mcp__example__example`, dispatches on a required `action` argument:

| action | purpose |
|--------|---------|
| `greet` | Return a greeting (with optional name parameter) |
| `echo` | Echo a message back unchanged |
| `status` | Server status and upstream connectivity info |
| `elicit_name` | Ask the MCP client to collect a name, then return a greeting |
| `scaffold_intent` | Elicit scaffold requirements and return JSON for the scaffold-project skill |
| `help` | Full in-tree action reference |

**Always prefer the MCP tool**. Fall back to HTTP curl only when MCP is unavailable.

---

## Action Reference

### `action="greet"` — Return a greeting

<!-- TEMPLATE: Document your action's parameters and response shape here.
     The AI uses this to construct correct tool calls. -->

| param | type | description |
|-------|------|-------------|
| `name` | string | Optional. Name to greet. Defaults to "World". |

**Examples:**

```
mcp__example__example(action="greet")
mcp__example__example(action="greet", name="Alice")
```

**Response shape:**
```json
{
  "greeting": "Hello, Alice!",
  "target": "Alice",
  "server": "https://api.example.com/v1"
}
```

---

### `action="echo"` — Echo a message

| param | type | description |
|-------|------|-------------|
| `message` | string | Required. Message to echo back. |

```
mcp__example__example(action="echo", message="Hello, world!")
```

**Response shape:**
```json
{
  "echo": "Hello, world!"
}
```

---

### `action="status"` — Server status

No parameters. Returns connectivity and configuration info.

```
mcp__example__example(action="status")
```

**Response shape:**
```json
{
  "status": "ok",
  "api_url": "https://api.example.com/v1",
  "note": "Replace with real health endpoint data"
}
```

---

### `action="elicit_name"` — Ask the user for a name

Uses MCP elicitation so the server can ask the client to show a small input
form to the user. Clients without elicitation support return a graceful fallback
message instead of failing the tool call.

No parameters.

```
mcp__example__example(action="elicit_name")
```

**Response shape:**
```json
{
  "greeting": "Hello, Alice! Welcome to the example MCP server.",
  "name": "Alice"
}
```

---

### `action="scaffold_intent"` — Create scaffold intent JSON

Uses MCP elicitation to collect what kind of project the user is building, then returns JSON for the `scaffold-project` skill. This action does **not** mutate files. The skill reads the JSON and creates an approval-first plan that the user can accept, edit, or reject.

This is intentionally MCP-only: it depends on MCP elicitation plus plugin skill handoff, which has no true CLI equivalent inside the user's agent/editor permission model.

No parameters.

```
mcp__example__example(action="scaffold_intent")
```

**Response shape:**
```json
{
  "kind": "rmcp_template_scaffold_intent",
  "schema_version": 1,
  "server_category": "upstream-client",
  "required_surfaces": ["mcp", "cli"],
  "project": {
    "display_name": "Unraid MCP",
    "crate_name": "unraid-mcp",
    "binary_name": "unraid",
    "service_name": "unraid",
    "env_prefix": "UNRAID"
  },
  "upstream": {
    "base_url_env": "UNRAID_API_URL",
    "auth_kind": "api-key"
  },
  "runtime": {
    "host": "127.0.0.1",
    "port": 3100,
    "mcp_transport": "dual"
  },
  "mcp_primitives": ["tools", "resources", "prompts", "elicitation"],
  "deployment": "none",
  "plugins": ["claude", "codex"],
  "publish_mcp": true,
  "crawl_docs": {
    "urls": ["https://docs.unraid.net/"],
    "repos": [],
    "search_topics": ["Unraid API authentication"]
  },
  "handoff": {
    "recommended_skill": "scaffold-project",
    "instructions": "Create an approval-first scaffold plan from this JSON. Do not mutate files until the user approves the plan."
  },
  "policy": {
    "business_action_minimum_surfaces": ["mcp", "cli"],
    "upstream_client_surfaces": ["mcp", "cli"],
    "application_platform_surfaces": ["api", "cli", "mcp", "web"]
  }
}
```

---

### `action="help"` — Canonical reference

Returns the authoritative in-tree action documentation. Use as ground truth if this skill document appears stale.

```
mcp__example__example(action="help")
```

---

## HTTP Fallback Mode

<!-- TEMPLATE: Update the curl examples with your service name and actions.
     The CLAUDE_PLUGIN_OPTION_* env vars are injected by the plugin runtime. -->

Use only when the MCP tool is unavailable. The plugin exports connection settings as:
- `CLAUDE_PLUGIN_OPTION_SERVER_URL` — base URL (e.g. `http://localhost:40060`)
- `CLAUDE_PLUGIN_OPTION_API_TOKEN` — bearer token

**Sensitive value handling:** `api_token` is declared `sensitive: true` in plugin.json.
It is never substituted into skill content — only the env var path above is valid.

### Health check (no auth required)

```bash
curl -s "$CLAUDE_PLUGIN_OPTION_SERVER_URL/health"
```

### Call the example tool

```bash
# Greet action
curl -s -X POST "$CLAUDE_PLUGIN_OPTION_SERVER_URL/mcp" \
  -H "Authorization: Bearer $CLAUDE_PLUGIN_OPTION_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"example","arguments":{"action":"greet","name":"Alice"}}}'

# Status action
curl -s -X POST "$CLAUDE_PLUGIN_OPTION_SERVER_URL/mcp" \
  -H "Authorization: Bearer $CLAUDE_PLUGIN_OPTION_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"example","arguments":{"action":"status"}}}'
```

---

## Example Workflows

<!-- TEMPLATE: Replace with workflows that make sense for your service.
     Good workflows show multi-step sequences that demonstrate real value. -->

### Quick health check

```
mcp__example__example(action="status")
```

### Verify the service is responding correctly

```
# 1. Check server status
mcp__example__example(action="status")

# 2. Test the API connection with a greeting
mcp__example__example(action="greet", name="test")

# 3. Verify echo round-trip
mcp__example__example(action="echo", message="ping")
```

### Scaffold a new project plan

```
# 1. Collect scaffold intent JSON through MCP elicitation
mcp__example__example(action="scaffold_intent")

# 2. Invoke/use the scaffold-project skill with the returned JSON
# 3. Review the generated plan before approving any file mutations
```
