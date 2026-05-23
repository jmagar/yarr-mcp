# Codex Plugin — .codex-plugin/plugin.json

<!-- TEMPLATE: This README explains the Codex plugin manifest. Keep it in the repo
     so future maintainers understand what each field does. Update the TEMPLATE:
     comments in plugin.json before publishing. -->

## What this is

`plugin.json` is the Codex plugin manifest — the Codex equivalent of the Claude Code
`.claude-plugin/plugin.json`. Both files live next to each other in `plugins/<service>/`
and share the same MCP server connection config (`.mcp.json`).

## File structure

```
plugins/example/
  .claude-plugin/
    plugin.json     ← Claude Code plugin manifest
  .codex-plugin/
    plugin.json     ← Codex plugin manifest (this file's sibling)
    README.md       ← You are here
  .mcp.json         ← Shared MCP server connection config (both plugins use this)
  hooks/            ← Claude Code hooks (Claude-specific, not used by Codex)
  skills/           ← Shared skills (both Claude Code and Codex can load these)
```

## Field reference

| Field | Description |
|---|---|
| `name` | TEMPLATE: Unique plugin identifier. Convention: `<service>-mcp`. |
| `version` | TEMPLATE: Semver; keep in sync with `Cargo.toml` and `server.json`. |
| `description` | TEMPLATE: One-line description for registries and `--help` output. |
| `homepage` | TEMPLATE: Your project's GitHub URL. |
| `repository` | TEMPLATE: Same as homepage for GitHub-hosted projects. |
| `license` | Keep `"MIT"` unless you chose a different license. |
| `keywords` | TEMPLATE: 3–6 tags for registry search. |
| `skills` | Path to shared skills directory. Do not change — convention is `"./skills/"`. |
| `mcpServers` | Path to shared `.mcp.json`. Do not change — both plugins use the same file. |
| `interface.displayName` | TEMPLATE: Human-readable name shown in Codex UI. |
| `interface.shortDescription` | TEMPLATE: 50-char tagline shown in plugin listings. |
| `interface.longDescription` | TEMPLATE: Full description for the detail page. |
| `interface.developerName` | TEMPLATE: Your name or org name. |
| `interface.category` | One of: `"Infrastructure"`, `"Productivity"`, `"Developer Tools"`, `"Data"`. |
| `interface.capabilities` | TEMPLATE: `["Read"]` for read-only, `["Read", "Write"]` for write ops. |
| `interface.websiteURL` | TEMPLATE: Your project URL. |
| `interface.defaultPrompt` | TEMPLATE: 3 sample prompts showing the most useful actions. |
| `interface.brandColor` | TEMPLATE: Hex color for the plugin icon background. `#6366F1` is Indigo-500. |
| `interface.composerIcon` | Path to a square PNG (512×512) for the composer icon. |
| `interface.logo` | Path to an SVG logo for the plugin detail page. |
| `author.name` | TEMPLATE: Your full name. |
| `author.email` | TEMPLATE: Your GitHub noreply email or public email. |
| `author.url` | TEMPLATE: Your GitHub profile URL. |

## Capabilities: Read vs Write

Set `capabilities` based on what your MCP server actually does:

- `["Read"]` — server only fetches/queries data (no mutations, no destructive actions)
- `["Read", "Write"]` — server can modify state (create, update, delete operations)

The template includes both `"Read"` and `"Write"` to show the pattern. If your server
is read-only, remove `"Write"`.

## Keeping plugin.json in sync

These fields must stay in sync across files:

| Field | plugin.json | Cargo.toml | server.json |
|---|---|---|---|
| `version` | `version` | `version` | `version` + `packages[0].version` |
| `homepage` / `repository` | both fields | `homepage` | `repository.url` |

The `scripts/check-version-sync.sh` script validates these automatically. Run it
before tagging a release.

## brandColor choices

| Color | Hex | Use case |
|---|---|---|
| Indigo-500 (default) | `#6366F1` | Generic/template |
| Amber-400 | `#F59E0B` | Unraid (warm hardware theme) |
| Emerald-500 | `#10B981` | Gotify (notifications/green) |
| Sky-500 | `#0EA5E9` | UniFi (networking/blue) |
| Violet-500 | `#8B5CF6` | Tailscale (purple brand) |

<!-- TEMPLATE: Pick a color that fits your service's brand or the color scheme
     of the upstream service's own UI. -->
