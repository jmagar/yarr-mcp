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
├── gemini-extension.json   # Gemini CLI extension manifest (no MCP connection yet — skills only)
├── .mcp.json               # Claude Code / Codex MCP connection — stdio, spawns bin/yarr directly
├── bin/
│   └── yarr                # Bundled release binary, committed to the repo
├── hooks/
│   └── hooks.json          # SessionStart + ConfigChange hook definitions
├── monitors/
│   └── monitors.json       # Background health monitor (requires Claude Code v2.1.105+)
└── skills/
    └── yarr/
        └── SKILL.md        # Tool documentation (shared by Claude and Codex)
```

## Platform manifests

Claude Code and Codex read their MCP connection config from `.mcp.json`. Gemini CLI has no MCP connection at all yet — `gemini-extension.json` only wires up the bundled skills, not a `mcpServers` block. All three share the same `skills/` directory.

| File | Platform | MCP config | Variable syntax |
|---|---|---|---|
| `.claude-plugin/plugin.json` | Claude Code | `.mcp.json` | `${user_config.*}` |
| `.codex-plugin/plugin.json` | Codex | `.mcp.json` | `${user_config.*}` |
| `gemini-extension.json` | Gemini CLI | none (skills-only) | `${settings.*}` |

**No `version` field in any manifest.** The marketplace assigns version from the git commit SHA. Adding an explicit version creates duplicate entries on every push.

## MCP connection

`.mcp.json` defaults to **stdio** — it spawns the bundled `bin/yarr` binary directly, no separately-run server required:

```json
{
  "mcpServers": {
    "yarr": {
      "type": "stdio",
      "command": "${CLAUDE_PLUGIN_ROOT}/bin/yarr",
      "args": ["mcp"],
      "env": {
        "YARR_SERVICES": "${user_config.yarr_services}",
        "YARR_SONARR_URL": "${user_config.sonarr_url}",
        "YARR_SONARR_API_KEY": "${user_config.sonarr_api_key}"
      }
    }
  }
}
```

(Full `env` block covers all 11 services — see the file itself.) `${user_config.*}` is populated from Claude/Codex `userConfig` settings at runtime. Installing the plugin is enough; there's nothing to start or point at a URL for.

A user who instead wants to run `yarr` as a persistent HTTP server (e.g. for other MCP clients, or to share one server across machines) can still do so separately — that's what the `server_url`/`api_token` `userConfig` fields and the health monitor (below) are for. That mode is independent of this plugin's own stdio MCP connection.

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

The MCP connection itself (`.mcp.json`) uses the bundled `bin/yarr`, but the
health monitor's `watch.sh` still resolves `yarr` from PATH (or `YARR_MCP_BIN`)
— it's checking an independent, optionally self-hosted HTTP server at
`${user_config.server_url}`, not the stdio process `.mcp.json` spawns. Install
`yarr` separately if you want the monitor to work:

```bash
npm i -g yarr-mcp
```

Disabling the plugin mid-session does not stop an already-running monitor; it stops when the session ends.

## Skills

`skills/yarr/SKILL.md` is the three-tier structured documentation for the `yarr` MCP tool. The AI reads Tier 1 for quick lookups, Tier 2 for parameter details, Tier 3 for multi-step workflows.

## Packaging checklist

1. Rebuild `bin/yarr` with `just release-sync` (or `cargo build --release` + copy) before tagging a release — it's the real binary spawned by `.mcp.json` and `hooks/hooks.json`, committed to the repo.
2. Confirm `yarr` is installed separately on PATH when testing the health monitor (`watch.sh`) — that's independent of the bundled `bin/yarr`.
3. Run `cargo test --test plugin_contract`.
4. Verify all manifests still omit explicit `version` fields.
5. Install through the target marketplace or local plugin path.
