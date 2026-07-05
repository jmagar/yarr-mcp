# plugins/yarr — Claude Code instructions

## What this directory is

Multi-platform plugin package for the Yarr MCP server. Contains manifests for Claude Code, Codex, and Gemini CLI — all pointing at the same MCP connection config and skills.

## File map

| File | Role |
|---|---|
| `.claude-plugin/plugin.json` | Claude Code manifest — identity, hooks, skills, monitors, `userConfig` |
| `.codex-plugin/plugin.json` | Codex manifest — same data + Codex UI fields (`interface`) |
| `gemini-extension.json` | Gemini CLI manifest — uses `settings` array instead of `userConfig` |
| `.mcp.json` | Shared MCP server connection config used by all three platforms |
| `bin/yarr` | Release binary used by the monitor — populate with `just install` |
| `hooks/hooks.json` | Lifecycle hook definitions: `SessionStart`, `ConfigChange` |
| `monitors/monitors.json` | Background health monitor config (requires Claude Code v2.1.105+) |
| `skills/yarr/SKILL.md` | Three-tier tool documentation shared by Claude and Codex |

## Versioning rule

**Do not add a `version` field to any manifest.** The marketplace derives version from the git commit SHA. An explicit `version` field causes every push to register as a new version and creates duplicate marketplace entries.

## Updating a manifest

When changing connection config (URL, auth headers), update `.mcp.json` — do not duplicate the values into each manifest separately. All three platforms read `.mcp.json`.

When changing user-configurable settings, update all three manifests: `userConfig` in the Claude and Codex `plugin.json` files, and `settings` in `gemini-extension.json`. Keep field names and descriptions consistent across all three.

## Monitors (Claude Code v2.1.105+)

`monitors/monitors.json` runs `scripts/watch.sh`, which delegates to an installed
`yarr` on PATH. Plugin monitors must not assume a bundled binary in the
plugin directory.

The monitor command uses `${user_config.server_url}` substitution — this is resolved at runtime from the user's plugin settings. Do not hardcode URLs in `monitors.json`.

When adding a new monitor: add an entry to `monitors.json` and reference only
scripts under `${CLAUDE_PLUGIN_ROOT}/scripts/`; those scripts should resolve the
runtime binary from PATH and exit non-blocking when it is unavailable.

## Updating the skill

`skills/yarr/SKILL.md` is shared by Claude Code and Codex. Gemini reads it via the `skills` path in `gemini-extension.json`. Edit it once — all platforms see the change.

The three-tier structure must be preserved:
- **Tier 1** (above fold): tool name, quick action table, critical gotchas
- **Tier 2** (middle): full action reference with parameters and response shapes
- **Tier 3** (bottom): workflows, HTTP fallback, error handling

## Updating plugin setup

`hooks/hooks.json` runs `${CLAUDE_PLUGIN_ROOT}/scripts/plugin-setup.sh`, which
delegates to an installed `yarr` on PATH. When you add or rename a
`userConfig` field, update the binary-owned plugin setup env mapping in
`src/main.rs` / `src/cli/setup.rs` so
`CLAUDE_PLUGIN_OPTION_*` values still map to the correct `YARR_*` variables.

Sensitive fields declared `"sensitive": true` in `plugin.json` are available as env vars in hooks but are **never** substituted into skill content.

## Yarr-specific package

This directory is the concrete yarr plugin package. Keep identifiers aligned
with the `yarr` binary, `YARR_*` environment variables, and the
`skills/yarr/` skill path. Keep the no-version rule: do not add `"version"`
to any manifest.
