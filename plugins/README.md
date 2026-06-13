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
│   └── hooks.json        # Lifecycle hook definitions
└── skills/
    ├── rustarr/
    │   └── SKILL.md      # Tool documentation for Claude and Codex
```

---

## Manifests

### `.claude-plugin/plugin.json`

Claude Code plugin manifest. Defines the plugin identity, shared skills,
monitor config, and user-configurable options.

**User config fields** (set via Claude Code plugin settings):

| Field | Type | Description |
|---|---|---|
| `server_url` | string | MCP HTTP server base URL (default: `http://localhost:40070`) |
| `api_token` | string (sensitive) | Bearer token for auth |
| `no_auth` | boolean | Disable auth (loopback dev only; non-loopback requires an upstream gateway) |
| `auth_mode` | string | `bearer` or `oauth` |
| `public_url` | string | Public URL for OAuth callbacks |
| `google_client_id` | string (sensitive) | Google OAuth client ID |
| `google_client_secret` | string (sensitive) | Google OAuth client secret |
| `auth_admin_email` | string | OAuth admin email |
| `rustarr_services` | string | Comma-separated configured media services |
| `sonarr_url` / `sonarr_api_key` | string / sensitive string | Sonarr connection |
| `radarr_url` / `radarr_api_key` | string / sensitive string | Radarr connection |

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

| Hook | Trigger | Command |
|---|---|---|
| `SessionStart` | Every Claude Code session start | `${CLAUDE_PLUGIN_ROOT}/bin/rustarr setup plugin-hook` |
| `ConfigChange` | User updates plugin settings | `${CLAUDE_PLUGIN_ROOT}/bin/rustarr setup plugin-hook` |

Timeout: 300 seconds.

### Binary-Owned Setup

The lifecycle command runs on every session start and config change.

- Reads `CLAUDE_PLUGIN_OPTION_*` env vars from plugin `userConfig`
- Exports those values as the binary's runtime environment variables
- Prepares the plugin appdata directory
- Runs setup checks and idempotent repairs through `rustarr setup plugin-hook`

Deployment policy, repair behavior, and failure classification live in the Rust
binary, not in manifest-specific shell code. Hooks intentionally do not manage
Docker, systemd, config rewrites, port conflicts, or OAuth redirect construction
themselves.

---

## Skills

### `skills/rustarr/SKILL.md`

Three-tier structured documentation for the `rustarr` MCP tool, used by both Claude Code and Codex to understand when and how to invoke the tool.

**Tier 1** (above the fold): tool name, quick action table, most common usage.  
**Tier 2**: full action reference — parameters, types, rustarr calls, response shapes.  
**Tier 3**: multi-step workflows demonstrating real-world use.

Also includes HTTP fallback rustarrs using `CLAUDE_PLUGIN_OPTION_SERVER_URL` and `CLAUDE_PLUGIN_OPTION_API_TOKEN` env vars for when the MCP connection isn't available.

Keep this skill aligned with the real rustarr action surface.

---

## Versioning

Plugin manifests must stay versionless. The marketplace derives plugin version
from the git commit SHA; adding an explicit manifest `version` field creates a
new duplicate marketplace entry on every push.

---

## Maintenance checklist

When changing the plugin package:

1. Keep Claude, Codex, and Gemini settings aligned.
2. Update `skills/rustarr/SKILL.md` when the action surface changes.
3. Keep `hooks/hooks.json` pointed at `${CLAUDE_PLUGIN_ROOT}/bin/rustarr setup plugin-hook`.
4. Verify all plugin manifests still omit explicit `version` fields.
