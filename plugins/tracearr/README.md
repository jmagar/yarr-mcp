# Tracearr (skills-only plugin)

Query Tracearr's public API for media tracing and analytics. Skills-only, no MCP server required.

This is a **skills-only, no-MCP** plugin. The skill drives the Tracearr REST API
directly with `curl`. Install it on its own if all you want is Tracearr — no
rustarr MCP server required. (For the full media fleet behind one MCP tool, with
these skills bundled as an offline fallback, install the `rustarr` plugin instead.)

## Configure

Set these in the plugin settings (`userConfig`). A `SessionStart` / `ConfigChange`
hook writes them to `~/.config/lab-tracearr/config.env`, which the skill scripts load
automatically — do not hand-edit or commit credentials.

| Setting | Sensitive | Description |
|---|---|---|
| `tracearr_url` | no | Tracearr URL |

## What's inside

- `skills/tracearr/` — the Tracearr skill (SKILL.md + helper scripts + references)
- `hooks/hooks.json` + `scripts/setup.sh` — bridges plugin settings to the script env file
- `.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `gemini-extension.json` — per-platform manifests
