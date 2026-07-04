# Codex Plugin Manifest

This directory contains the Codex manifest for the Yarr MCP plugin. It is the
Codex sibling of `../.claude-plugin/plugin.json` and shares the connection
settings in `../.mcp.json`.

Keep these files aligned when editing plugin metadata:

| Surface | File |
|---|---|
| Claude Code manifest | `plugins/yarr/.claude-plugin/plugin.json` |
| Codex manifest | `plugins/yarr/.codex-plugin/plugin.json` |
| Gemini manifest | `plugins/yarr/gemini-extension.json` |
| Shared MCP connection | `plugins/yarr/.mcp.json` |
| Shared skill | `plugins/yarr/skills/yarr/SKILL.md` |

Do not add a `version` field to any plugin manifest. The marketplace derives
plugin versions from git commits; explicit versions create duplicate entries.

Yarr exposes both read and write capabilities because `api_post` can trigger
commands in configured upstream services. Credential values must stay in
`userConfig`/settings and server environment variables, never in skill text.
