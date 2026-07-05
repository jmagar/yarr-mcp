# SABnzbd (skills-only plugin)

View, pause, resume, and manage the SABnzbd Usenet download queue and history via its REST API. Skills-only, no MCP server required.

This is a **skills-only, no-MCP** plugin. The skill drives the SABnzbd REST API
directly with `curl`. Install it on its own if all you want is SABnzbd — no
yarr MCP server required. (For the full media fleet behind one MCP tool, with
these skills bundled as an offline fallback, install the `yarr` plugin instead.)

## Configure

Set these in the plugin settings (`userConfig`). A `SessionStart` / `ConfigChange`
hook writes them to `~/.config/lab-sabnzbd/config.env`, which the skill scripts load
automatically — do not hand-edit or commit credentials.

| Setting | Sensitive | Description |
|---|---|---|
| `sabnzbd_url` | no | SABnzbd URL |
| `sabnzbd_api_key` | yes | SABnzbd API key |

## What's inside

- `skills/sabnzbd/` — the SABnzbd skill (SKILL.md + helper scripts + references)
- `hooks/hooks.json` + `scripts/setup.sh` — bridges plugin settings to the script env file
- `.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `gemini-extension.json` — per-platform manifests
