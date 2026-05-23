# plugins

Claude Code and Codex plugin packages for the MCP server. Both platforms share the same skills and MCP connection config — only the manifests differ.

## Structure

```
plugins/rustarr/
├── .claude-plugin/
│   └── plugin.json       # Claude Code manifest
├── .codex-plugin/
│   ├── plugin.json       # Codex manifest
│   └── README.md         # Codex manifest field reference
├── .mcp.json             # Shared MCP server connection config
├── hooks/
│   ├── hooks.json        # Lifecycle hook definitions
│   └── plugin-setup.sh  # Deployment and validation script
└── skills/
    ├── rustarr/
    │   └── SKILL.md      # Tool documentation for Claude and Codex
    └── scaffold-project/
        └── SKILL.md      # Turns scaffold_intent JSON into an approval-first plan
```

---

## Manifests

### `.claude-plugin/plugin.json`

Claude Code plugin manifest. Defines the plugin identity, MCP server connection, lifecycle hooks, and user-configurable options.

**User config fields** (set via Claude Code plugin settings):

| Field | Type | Description |
|---|---|---|
| `server_url` | string | MCP HTTP server base URL (default: `http://localhost:3000`) |
| `api_token` | string (sensitive) | Bearer token for auth |
| `no_auth` | boolean | Disable auth (loopback dev only; non-loopback requires an upstream gateway) |
| `auth_mode` | string | `bearer` or `oauth` |
| `public_url` | string | Public URL for OAuth callbacks |
| `google_client_id` | string (sensitive) | Google OAuth client ID |
| `google_client_secret` | string (sensitive) | Google OAuth client secret |
| `auth_admin_email` | string | OAuth admin email |
| `rustarr_api_url` | string | Upstream service URL |
| `rustarr_api_key` | string (sensitive) | Upstream service API key |

**TEMPLATE**: Replace `rustarr_api_url` / `rustarr_api_key` with your service's credential fields.

### `.codex-plugin/plugin.json`

Codex equivalent of the Claude Code manifest. Shares `.mcp.json` and `skills/` with the Claude plugin. Adds Codex-specific UI fields under `interface`:

- `displayName`, `shortDescription`, `longDescription` — registry presentation
- `defaultPrompt` — three sample prompts shown in the Codex UI
- `brandColor` — hex color for the plugin icon (e.g., `#6366F1`)
- `composerIcon`, `logo` — asset paths (512×512 PNG, SVG)

See `.codex-plugin/README.md` for a full field reference and `brandColor` guide.

### `.mcp.json`

Shared MCP server connection config used by both plugins. Points both clients at the same HTTP endpoint with the same auth headers.

```json
{
  "mcpServers": {
    "rustarr": {
      "type": "http",
      "url": "${user_config.server_url}/mcp",
      "headers": {
        "Authorization": "Bearer ${user_config.api_token}"
      }
    }
  }
}
```

---

## Hooks

### `hooks/hooks.json`

Defines two lifecycle hooks:

| Hook | Trigger | Script |
|---|---|---|
| `SessionStart` | Every Claude Code session start | `hooks/plugin-setup.sh` |
| `ConfigChange` | User updates plugin settings | `hooks/plugin-setup.sh` |

Timeout: 300 seconds.

### `hooks/plugin-setup.sh`

The lifecycle adapter. Runs on every session start and config change.

- Reads `CLAUDE_PLUGIN_OPTION_*` env vars from plugin `userConfig`
- Exports those values as the binary's runtime environment variables
- Prepares the plugin appdata directory
- Ensures `rustarr` is available on `PATH`
- Calls `rustarr setup plugin-hook "$@"`

Deployment policy, repair behavior, and failure classification live in the Rust binary, not in the hook script. The script is idempotent and intentionally does not manage Docker, systemd, config rewrites, port conflicts, or OAuth redirect construction itself.

---

## Skills

### `skills/rustarr/SKILL.md`

Three-tier structured documentation for the `rustarr` MCP tool, used by both Claude Code and Codex to understand when and how to invoke the tool.

**Tier 1** (above the fold): tool name, quick action table, most common usage.  
**Tier 2**: full action reference — parameters, types, rustarr calls, response shapes.  
**Tier 3**: multi-step workflows demonstrating real-world use.

Also includes HTTP fallback rustarrs using `CLAUDE_PLUGIN_OPTION_SERVER_URL` and `CLAUDE_PLUGIN_OPTION_API_TOKEN` env vars for when the MCP connection isn't available.

**TEMPLATE**: Replace the action table and rustarrs with your service's actual actions.

---

## Version sync

Three files must stay in sync when you bump the version:

| File | Field |
|---|---|
| `Cargo.toml` | `version` |
| `.claude-plugin/plugin.json` | `version` |
| `.codex-plugin/plugin.json` | `version` |

Use `scripts/bump-version.sh patch` (or `minor`/`major`) to update all of them atomically.

---

## TEMPLATE checklist

When adapting this plugin for a real service:

1. Replace all `rustarr` / `Rustarr` / `RUSTARR_` identifiers with your service name.
2. Update `userConfig` in both `plugin.json` files to match your service's credential fields.
3. Update `skills/rustarr/SKILL.md` with your actual actions, parameters, and rustarrs.
4. Set `brandColor` in `.codex-plugin/plugin.json` to your service's color.
5. Replace `defaultPrompt` entries in the Codex manifest with realistic prompts for your service.
6. Run `scripts/bump-version.sh` after any version change.
