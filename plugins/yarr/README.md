# yarr plugin

Multi-platform plugin package that connects Claude Code, Codex, and Gemini CLI to the Yarr MCP server.

## Structure

```
plugins/yarr/
├── .claude-plugin/
│   └── plugin.json         # Claude Code manifest
├── .codex-plugin/
│   ├── plugin.json         # Codex manifest
│   └── README.md           # Codex manifest field reference
├── gemini-extension.json   # Gemini CLI extension manifest
├── .mcp.json               # Shared MCP server connection config (all three platforms)
├── bin/
│   └── yarr                # Tracked hook wrapper; resolves installed yarr from PATH
├── hooks/
│   └── hooks.json          # SessionStart + ConfigChange hook definitions
├── monitors/
│   └── monitors.json       # Background health monitor (requires Claude Code v2.1.105+)
└── skills/
    └── yarr/
        └── SKILL.md        # Tool documentation (shared by Claude and Codex)
```

## Platform manifests

Claude Code and Codex read their MCP connection config from the shared `.mcp.json`. Gemini CLI embeds its `mcpServers` config inline in `gemini-extension.json` (its own format). All three share the same `skills/` directory.

| File | Platform | MCP config | Variable syntax |
|---|---|---|---|
| `.claude-plugin/plugin.json` | Claude Code | `.mcp.json` | `${user_config.*}` |
| `.codex-plugin/plugin.json` | Codex | `.mcp.json` | `${user_config.*}` |
| `gemini-extension.json` | Gemini CLI | inline `mcpServers` | `${settings.*}` |

**No `version` field in any manifest.** The marketplace assigns version from the git commit SHA. Adding an explicit version creates duplicate entries on every push.

## MCP connection

`.mcp.json` is shared across all platforms:

```json
{
  "mcpServers": {
    "yarr": {
      "type": "http",
      "url": "${user_config.server_url}/mcp",
      "headers": { "Authorization": "Bearer ${user_config.api_token}" }
    }
  }
}
```

The `${user_config.*}` / `${settings.*}` variables are populated from each platform's user-configurable settings at runtime.

## Hooks

`hooks/hooks.json` runs `${CLAUDE_PLUGIN_ROOT}/bin/yarr setup plugin-hook`
on `SessionStart` and `ConfigChange`.

Plugin setup is owned by the installed `yarr` binary. The tracked `bin/yarr`
wrapper only resolves `yarr` from PATH (or `YARR_MCP_BIN`) and fails loudly when
it is unavailable.

## Monitors

**Requires Claude Code v2.1.105+.**

`monitors/monitors.json` declares a background `server-health` monitor that starts automatically at session start. It runs `scripts/watch.sh`, which delegates to an installed `yarr` on PATH, and delivers each stdout line to Claude as a notification whenever the MCP server changes state.

The monitor emits only on state transitions — Claude is not notified while the server is stable. Three states:

- `UP` — `/health` returned 2xx
- `DOWN` — connection refused / timeout
- `DEGRADED(HTTP N)` — non-2xx HTTP response

The plugin ships only a tiny hook wrapper, not the actual server binary. Install
`yarr` separately before enabling hooks or the monitor:

```bash
npm i -g yarr-mcp
```

Disabling the plugin mid-session does not stop an already-running monitor; it stops when the session ends.

## Skills

`skills/yarr/SKILL.md` is the three-tier structured documentation for the `yarr` MCP tool. The AI reads Tier 1 for quick lookups, Tier 2 for parameter details, Tier 3 for multi-step workflows.

## Packaging checklist

1. Confirm `bin/yarr` is a tracked executable wrapper, not a bundled release binary.
2. Confirm `yarr` is installed separately when testing runtime setup.
3. Run `cargo test --test plugin_contract`.
4. Verify all manifests still omit explicit `version` fields.
5. Install through the target marketplace or local plugin path.
