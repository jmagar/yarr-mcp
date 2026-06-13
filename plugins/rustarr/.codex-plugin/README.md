# Codex Plugin Manifest

This directory contains the Codex manifest for the Rustarr MCP plugin. It is the
Codex sibling of `../.claude-plugin/plugin.json` and shares the connection
settings in `../.mcp.json`.

Keep these files aligned when editing plugin metadata:

| Surface | File |
|---|---|
| Claude Code manifest | `plugins/rustarr/.claude-plugin/plugin.json` |
| Codex manifest | `plugins/rustarr/.codex-plugin/plugin.json` |
| Gemini manifest | `plugins/rustarr/gemini-extension.json` |
| Shared MCP connection | `plugins/rustarr/.mcp.json` |
| Shared skill | `plugins/rustarr/skills/rustarr/SKILL.md` |

Do not add a `version` field to any plugin manifest. The marketplace derives
plugin versions from git commits; explicit versions create duplicate entries.

Rustarr exposes both read and write capabilities because `api_post` can trigger
commands in configured upstream services. Credential values must stay in
`userConfig`/settings and server environment variables, never in skill text.
