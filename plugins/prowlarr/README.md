# Prowlarr (skills-only plugin)

List indexers, run searches across them, and check indexer health and stats in Prowlarr via its REST API. Skills-only, no MCP server required.

This is a **skills-only, no-MCP** plugin. The skill drives the Prowlarr REST API
directly with `curl`. Install it on its own if all you want is Prowlarr — no
rustarr MCP server required. (For the full media fleet behind one MCP tool, with
these skills bundled as an offline fallback, install the `rustarr` plugin instead.)

## Configure

Set these in the plugin settings (`userConfig`). A `SessionStart` / `ConfigChange`
hook writes them to `~/.config/lab-prowlarr/config.env`, which the skill scripts load
automatically — do not hand-edit or commit credentials.

| Setting | Sensitive | Description |
|---|---|---|
| `prowlarr_url` | no | Prowlarr URL |
| `prowlarr_api_key` | yes | Prowlarr API key |

## What's inside

- `skills/prowlarr/` — the Prowlarr skill (SKILL.md + helper scripts + references)
- `hooks/hooks.json` + `scripts/setup.sh` — bridges plugin settings to the script env file
- `.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `gemini-extension.json` — per-platform manifests
