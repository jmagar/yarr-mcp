---
name: scaffold-project
description: Use this skill when the user wants to adapt rmcp-template for a new MCP server, especially after calling the scaffold_intent elicitation action. It turns the returned JSON intent into an approval-first implementation plan without directly mutating files.
---

# Scaffold Project Skill

Use this skill to turn scaffold intent JSON into a concrete, user-approved plan for adapting `rmcp-template` into a real server.

Canonical spec: `docs/specs/scaffold-intent-handoff.md`.
Machine-readable contract: `docs/contracts/scaffold-intent.schema.json`.
Example payloads: `docs/contracts/examples/scaffold-intent-upstream-client.json` and `docs/contracts/examples/scaffold-intent-application-platform.json`.

## When to use this skill

Use this skill when the user says they want to:

- scaffold a new project from `rmcp-template`
- adapt this template for a named service
- decide whether a server should be MCP + CLI or API + CLI + MCP + Web
- turn `scaffold_intent` JSON into an implementation plan
- review what files would change before approving scaffold work

Do **not** use this skill for normal runtime interactions with a completed service. Use the service-specific skill/tool documentation for that.

`scaffold_intent` is intentionally MCP-only. It is an exception to the normal MCP + CLI parity rule because it depends on MCP elicitation and plugin skill handoff; there is no true CLI equivalent for that user-permission flow. It is not treated as a business action for MCP + CLI parity; a future CLI planner, if added, would be separately scoped and not required for parity.

## Primary workflow

1. Ask the MCP server to collect scaffold intent with elicitation if the user has not already provided intent JSON:

   ```
   mcp__example__example(action="scaffold_intent")
   ```

2. Read the returned JSON as **intent only**. It does not grant permission to mutate files.
3. Check the JSON against the expected fields in `docs/contracts/scaffold-intent.schema.json`.
4. Ask a concise clarifying question only if a required decision is missing or contradictory.
5. Draft a plan that the user can review, edit, approve, or reject.
6. Stop at an approval checkpoint. Do not apply changes while presenting the plan.
7. After explicit approval, implement only the approved steps and keep the user in control of file changes through normal tool permissions.

## Returned JSON shape

The tool returns an object like:

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

## Fallback responses

Only full success payloads should be validated against `docs/contracts/scaffold-intent.schema.json`.

Expected non-planning responses:

| `status` | Instruction |
|---|---|
| `no_input` | Stop and tell the user no scaffold intent was provided. |
| `declined` | Stop and tell the user no plan will be generated. |
| `cancelled` | Stop and offer to restart the wizard later. |
| `elicitation_not_supported` | Ask the user to provide or confirm the same intent fields manually, then draft a plan from those answers. |

## Contract notes

Validate examples and generated payloads against `docs/contracts/scaffold-intent.schema.json`.

Accepted `upstream.auth_kind` values:

| value | meaning |
|---|---|
| `none` | No upstream authentication. |
| `api-key` | API key-style upstream auth. |
| `bearer` | Bearer token-style upstream auth. |
| `oauth` | OAuth-style upstream auth. |
| `both` | Scaffold both static-token/API-key and OAuth-style paths where applicable. |
| `other` | Custom auth; call out follow-up design work in the plan. |

Current lightweight intent fields include:

| field | planning implication |
|---|---|
| `runtime.host` / `runtime.port` | Default bind settings and env var docs. |
| `runtime.mcp_transport` | `stdio`, `http`, or `dual` transport scaffolding. |
| `mcp_primitives` | Which MCP primitives to keep/scaffold: tools, resources, prompts, elicitation. |
| `deployment` | Whether to include no deployment, systemd, or Docker scaffolding. |
| `plugins` | Which plugin surfaces to scaffold: Claude, Codex, Gemini; all, none, or any subset. |
| `publish_mcp` | Whether to update `server.json` for MCP registry publishing. |
| `crawl_docs` | URLs, repos, or search topics to crawl via Axon before planning implementation details. |

## How to interpret intent

### Server category

- `upstream-client`: plan for MCP + CLI only. The server wraps an upstream API/service for agent and scripting use.
- `application-platform`: plan for API + CLI + MCP + Web. The project owns meaningful local workflows, state, dashboards, or non-MCP consumers.

If the selected `required_surfaces` conflict with `server_category`, call out the conflict and ask the user which one to keep before planning changes.

### Runtime

Use `runtime` to decide the server entrypoints and docs:

| value | planning instruction |
|---|---|
| `mcp_transport = "stdio"` | Plan child-process MCP only. HTTP routes may be omitted unless needed for health/ops. |
| `mcp_transport = "http"` | Plan Streamable HTTP MCP on `runtime.host:runtime.port`. |
| `mcp_transport = "dual"` | Plan both `mcp` stdio mode and HTTP `serve` mode. |

### MCP primitives

Only plan the primitives listed in `mcp_primitives`:

| primitive | planning instruction |
|---|---|
| `tools` | Always include business action dispatch. |
| `resources` | Include schema/status/reference resources only if selected. |
| `prompts` | Include prompt templates only if selected. |
| `elicitation` | Include elicitation examples or workflows only if selected. |

### Deployment

| deployment | planning instruction |
|---|---|
| `none` | Do not add systemd units, Dockerfiles, compose files, or deployment docs beyond local build/run. |
| `systemd` | Plan systemd unit/env file docs only if explicitly selected. |
| `docker` | Plan container build/runtime config only if explicitly selected. |

### Plugins

`plugins` may be empty or contain any subset of `claude`, `codex`, and `gemini`.

- Empty array: plan to remove, ignore, or leave plugin scaffolding disabled.
- Subset: plan only the selected plugin manifests/skills/config.
- All three: plan Claude, Codex, and Gemini plugin surfaces.

### Publishing

If `publish_mcp` is `true`, include `server.json` in the plan and map the project metadata into the MCP registry manifest. If `false`, avoid registry publishing work unless the user asks for it later.

### Docs crawling

If `crawl_docs` contains `urls`, `repos`, or `search_topics`, include a proposed research/crawl step in the plan. Do not crawl external URLs/repos before the user approves the plan or explicitly authorizes research. Do not invent API shapes from the crawl request alone.

## Surface policy

Always enforce the project surface policy:

| Server category | Required surfaces | Examples |
|---|---|---|
| `upstream-client` | MCP + CLI | `unrust`, `rustifi`, `rustify`, `rustscale`, `apprise` |
| `application-platform` | API + CLI + MCP + Web | `axon`, `lab`, `syslog` |

For upstream-client servers, do **not** add or preserve REST/Web just because the upstream has an HTTP API. Recommend removing, ignoring, or feature-gating `apps/web` and REST handlers unless the user explicitly wants local dashboards/workflows/non-MCP consumers.

## Plan format

Present the plan in this order and keep it concrete enough for approval:

1. **Summary** — one paragraph describing the scaffold target.
2. **Surface decision** — explain why the selected surfaces are required.
3. **Rename map** — identifiers, env vars, scopes, plugin names, binary/crate names.
4. **Runtime/plugin/deployment choices** — summarize `host`, `port`, `mcp_transport`, `mcp_primitives`, `deployment`, `plugins`, `publish_mcp`, and `crawl_docs`.
5. **Files to change** — grouped by Rust service, MCP, CLI, API/Web, plugins, deployment, publishing, tests, docs.
6. **Deferred decisions** — list anything that still needs user input, API docs, or crawl results.
7. **Tests/validation** — exact commands to run.
8. **Approval checkpoint** — ask the user to approve before any mutation.

Use this approval wording:

> Please review this plan. I will not modify files until you approve it. Reply with approval, edits to the plan, or anything you want removed from scope.

## Output template

```md
## Scaffold Plan: <display_name>

### 1. Summary
<one paragraph>

### 2. Surface Decision
- Server category: `<server_category>`
- Required surfaces: `<required_surfaces>`
- Rationale: <why this matches the policy>

### 3. Rename Map
| Template identifier | New identifier |
|---|---|
| `example` | `<binary_name>` |
| `rmcp-template` | `<crate_name>` |
| `ExampleService` | `<ServiceName>Service` |
| `EXAMPLE_*` | `<ENV_PREFIX>_*` |
| `example:read` | `<service_name>:read` |

Keep `scaffold_intent` MCP-only in scaffolded projects and rename its scope from `example:read` to `<service_name>:read`.

### 4. Runtime / Plugins / Deployment
- Host/port: `<host>:<port>`
- MCP transport: `<stdio|http|dual>`
- MCP primitives: `<tools/resources/prompts/elicitation>`
- Deployment: `<none|systemd|docker>`
- Plugins: `<claude/codex/gemini or none>`
- MCP registry publishing: `<true|false>`
- Docs crawl inputs: `<urls/repos/search_topics or none>`

### 5. Files to Change
- Rust service/client:
- MCP:
- CLI:
- API/Web:
- Plugins:
- Deployment/publishing:
- Tests/docs:

### 6. Deferred Decisions
- <items needing approval, docs, or research>

### 7. Validation
- `<command>`

### 8. Approval Checkpoint
Please review this plan. I will not modify files until you approve it. Reply with approval, edits to the plan, or anything you want removed from scope.
```

## Approval handling

- If the user approves the full plan, implement only that plan.
- If the user edits the plan, restate the revised scope before implementing.
- If the user approves only part of the plan, implement only the approved subset.
- If the user asks for a change that conflicts with the surface policy, explain the conflict and ask for confirmation before proceeding.
- If the user provides new scaffold intent JSON later, treat it as a new planning input and regenerate the plan.

## Safety rules

- Do not treat scaffold intent JSON as permission to mutate files; it is intent only.
- Do not commit, push, delete, or overwrite unrelated work without explicit approval.
- Preserve the user's surface decision. If it conflicts with the policy, call that out before proceeding.
- For destructive/write actions, require explicit confirmation gates in the service layer and document scopes.
- Do not add Docker, systemd, REST, Web, or plugin surfaces unless the intent JSON or user approval explicitly includes them.
- Do not invent upstream API details. If docs are missing, add a crawl/research step or ask the user.
