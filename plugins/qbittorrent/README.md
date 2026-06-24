# qBittorrent (skills-only plugin)

List, add, pause, resume, and remove torrents and check transfer stats in qBittorrent via its WebUI API. Skills-only, no MCP server required.

This is a **skills-only, no-MCP** plugin. The skill drives the qBittorrent REST API
directly with `curl`. Install it on its own if all you want is qBittorrent — no
rustarr MCP server required. (For the full media fleet behind one MCP tool, with
these skills bundled as an offline fallback, install the `rustarr` plugin instead.)

## Configure

Set these in the plugin settings (`userConfig`). A `SessionStart` / `ConfigChange`
hook writes them to `~/.config/lab-qbittorrent/config.env`, which the skill scripts load
automatically — do not hand-edit or commit credentials.

| Setting | Sensitive | Description |
|---|---|---|
| `qbittorrent_url` | no | qBittorrent URL |
| `qbittorrent_username` | no | qBittorrent username |
| `qbittorrent_password` | yes | qBittorrent password |

## What's inside

- `skills/qbittorrent/` — the qBittorrent skill (SKILL.md + helper scripts + references)
- `hooks/hooks.json` + `scripts/setup.sh` — bridges plugin settings to the script env file
- `.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `gemini-extension.json` — per-platform manifests
