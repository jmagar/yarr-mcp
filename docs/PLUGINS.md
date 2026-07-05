# Plugin Surfaces

Yarr ships the **`yarr` MCP plugin** plus **11 standalone, skills-only
(no-MCP) plugins** — one per service. Each plugin has three host-specific
entrypoints, e.g. for `yarr`:

- Claude Code: `plugins/yarr/.claude-plugin/plugin.json`
- Codex: `plugins/yarr/.codex-plugin/plugin.json`
- Gemini: `plugins/yarr/gemini-extension.json`

The `yarr` plugin's three surfaces describe the same MCP server, expose the
same skills, and connect to the same HTTP MCP endpoint. The host manifests
differ, but the service behavior should not.

## Two ways to consume the stack

- **`yarr`** — the full MCP server plugin. It also bundles every per-service
  skill as a direct-HTTP fallback for when the MCP server is unavailable; the
  binary-owned setup hook (`yarr setup plugin-hook`) writes the per-service
  `~/.config/lab-<service>/config.env` files those fallback skills read.
- **One plugin per service** (`plugins/<service>/`, bare-named: `sonarr`,
  `radarr`, `prowlarr`, `overseerr`, `sabnzbd`, `qbittorrent`, `plex`,
  `jellyfin`, `tautulli`, `tracearr`, `bazarr`) — skills-only, no MCP server.
  Each drives its service's REST API directly with `curl` and writes its own
  isolated `~/.config/lab-<service>/config.env` from a `SessionStart` hook.

Both are listed in the marketplaces: `.claude-plugin/marketplace.json` (Claude
Code) and `.agents/plugins/marketplace.json` (Codex). See
[`plugins/README.md`](../plugins/README.md) for the full layout. The rest of this
document focuses on the `yarr` MCP plugin.

## Layout

```text
plugins/yarr/
  .claude-plugin/
    plugin.json          # Claude Code manifest
  .codex-plugin/
    plugin.json          # Codex manifest
    README.md            # Codex manifest field reference
  .mcp.json              # Shared Claude/Codex MCP connection config
  gemini-extension.json  # Gemini CLI extension manifest
  hooks/
    hooks.json           # Claude lifecycle hook declarations
  bin/
    yarr             # Optional Git LFS-tracked plugin binary artifact
  skills/
    yarr/
      SKILL.md           # Shared action documentation
```

## Shared Contract

Each plugin surface should agree on:

- service name and repository URL
- MCP server name
- HTTP MCP URL shape: `<server_url>/mcp`
- bearer token setting name
- upstream service credential names
- action list and skill documentation. The MCP surface is a single `yarr` tool that
  runs Code Mode; the skill documents the in-sandbox surface (per-service callables —
  generated OpenAPI operations for the 6 spec-backed services, curated commands for
  the download/stats capabilities — plus `api.<service>` passthrough and
  `codemode.search`/`describe` discovery)
- read/write capability claims

Keep the plugin manifests thin. Runtime setup belongs in the service binary, not in manifest-specific shell code.

## Claude Code

Claude Code uses `plugins/yarr/.claude-plugin/plugin.json`.

Responsibilities:

- identifies the plugin and repository
- declares shared skills and user settings
- defines `userConfig` settings exposed in Claude Code
- marks sensitive values with `sensitive: true`

Claude-specific lifecycle hooks live in `plugins/yarr/hooks/hooks.json`. The default hooks are:

| Hook | Trigger | Command |
| --- | --- | --- |
| `SessionStart` | every Claude Code session start | `${CLAUDE_PLUGIN_ROOT}/bin/yarr setup plugin-hook` |
| `ConfigChange` | plugin user settings change | `${CLAUDE_PLUGIN_ROOT}/bin/yarr setup plugin-hook` |

Plugin setup is binary-owned. The standard command is:

```bash
<binary> setup plugin-hook
```

For rollout audits, the binary must also support:

```bash
<binary> setup plugin-hook --no-repair
```

The binary may map `CLAUDE_PLUGIN_OPTION_*` values into runtime env vars, create the appdata directory, ensure prerequisites are available, and perform setup checks or repair. Hooks should not own Docker/systemd orchestration, config rewriting, smoke-test policy, or failure classification.

## Codex

Codex uses `plugins/yarr/.codex-plugin/plugin.json`.

Responsibilities:

- identifies the plugin for Codex listings
- points at shared `skills`
- describes the interface shown in Codex UI
- declares read/write capabilities
- provides yarr prompts
- provides branding fields such as `brandColor`, `composerIcon`, and `logo`

Codex does not use Claude lifecycle hooks. Its manifest should still point to the same MCP server and shared skills so behavior stays aligned with Claude Code.

Codex-specific fields to adapt:

| Field | Purpose |
| --- | --- |
| `interface.displayName` | human-readable plugin name |
| `interface.shortDescription` | short listing text |
| `interface.longDescription` | full listing text |
| `interface.capabilities` | `["Read"]` or `["Read", "Write"]` |
| `interface.defaultPrompt` | three realistic prompts |
| `interface.brandColor` | service-appropriate hex color |

See `plugins/yarr/.codex-plugin/README.md` for the full manifest field reference.

## Gemini

Gemini uses `plugins/yarr/gemini-extension.json`.

Responsibilities:

- identifies the extension
- declares Gemini settings
- connects to the MCP HTTP endpoint
- points at shared skills
- optionally points Gemini at a context file with `contextFileName`

The Gemini manifest uses `settings.*` interpolation instead of Claude/Codex `user_config.*` interpolation:

```json
"url": "${settings.server_url}/mcp"
```

Sensitive Gemini settings use:

```json
"secret": true
```

Keep Gemini setting names aligned with Claude/Codex where possible. For yarr, prefer `server_url`, `api_token`, `<service>_api_url`, and `<service>_api_key` across all three surfaces.

## Plugin Validation

Run the plugin layout validator after changing manifests, MCP config, hooks, or
skills:

```bash
just validate-plugin
# or
scripts/validate-plugin-layout.sh
```

The validator checks:

- Claude, Codex, and Gemini manifests are valid JSON
- plugin manifests do not contain a `version` field
- manifests keep connection config external and point at the shared skills path
- shared MCP config exposes the `yarr` HTTP server at `${user_config.server_url}/mcp`
- Gemini config exposes the same `yarr` HTTP server at `${settings.server_url}/mcp`
- hook config runs `${CLAUDE_PLUGIN_ROOT}/bin/yarr setup plugin-hook`
- every skill has `name:` and `description:` frontmatter

Use `PLUGIN_ROOT=plugins/<service>` when validating an adapted service package.

For release checks, `just pre-release` includes this validator and the other
template gates.

## Shared MCP Config

Claude Code and Codex share `plugins/yarr/.mcp.json`:

```json
{
  "mcpServers": {
    "yarr": {
      "type": "http",
      "url": "${user_config.server_url}/mcp",
      "headers": {
        "Authorization": "Bearer ${user_config.api_token}"
      }
    }
  }
}
```

Gemini carries equivalent MCP config directly in `gemini-extension.json` because its interpolation model is different.

## Skills

`plugins/yarr/skills/yarr/SKILL.md` is shared across Claude, Codex, and Gemini. Every skill follows the three-tier fallback pattern — agents try each tier in order and stop when one works:

```markdown
# yarr — Claude Code Skill

Use this skill whenever you need to query or manage the Yarr service.

## Tier 1: MCP tool (preferred)
Use when the yarr MCP server is configured in your agent.

sonarr(action="list")
sonarr(action="service_status")
sonarr(action="help")          # always available, no auth required

## Tier 2: CLI binary
Use when MCP is unavailable but the binary is installed in $PATH.

yarr things [--json]
yarr thing <id> [--json]
yarr status

Env required: `YARR_SERVICES` plus per-service URL/key variables.

## Tier 3: Direct API (last resort)
Use when neither MCP nor CLI is available.

curl -H "X-Api-Key: $YARR_SONARR_API_KEY" \
     "$YARR_SONARR_URL/api/v3/system/status"

## Gotchas
- [service-specific pitfalls go here]
- [e.g. pagination, required headers, rate limits]
```

The skill should also include:

- quick action table (action → description → required params)
- full parameter reference with types
- common workflows (status check → list → inspect)
- response shapes for key actions
- sensitive-value handling notes (never log tokens, etc.)

Do not maintain separate skill docs per host. Update the shared skill when the action surface changes; Claude, Codex, and Gemini all read the same file.

## Binary-Owned Hook Standard

Every Rust server with a Claude plugin should expose:

```bash
<binary> setup plugin-hook
<binary> setup plugin-hook --no-repair
<binary> setup check
<binary> setup repair
```

`setup plugin-hook` should:

- run `setup check` first
- run `setup repair` only when needed and only when `--no-repair` is absent
- emit structured JSON when the global JSON flag is used
- include `exit_policy`, `blocking_failures`, `advisory_failures`, `ran_repair`, and `no_repair`
- exit `0` for success or advisory failures
- exit nonzero for blocking failures
- enforce a bounded total hook runtime

Advisory failures are non-blocking local conditions such as missing `.env` files when process env already supplies values, occupied MCP ports, optional startup proofs, or model prewarm. Blocking failures are prerequisites required for the plugin to function, such as missing appdata directories, missing required upstream credentials, or invalid OAuth/auth configuration.

Use `scripts/check-plugin-hook-contract.py` to audit the cross-repo standard:

```bash
# Static hook/delegation checks for all known Rust servers.
scripts/check-plugin-hook-contract.py

# Also run each binary's `setup plugin-hook --no-repair` JSON contract.
scripts/check-plugin-hook-contract.py --execute
```

## Version And Release Sync

Keep version and metadata synchronized across:

| File | Fields |
| --- | --- |
| `Cargo.toml` | package `version`, homepage/repository when present |
| `plugins/yarr/.claude-plugin/plugin.json` | identity, repository, user config; no `version` field |
| `plugins/yarr/.codex-plugin/plugin.json` | identity, repository, interface metadata; no `version` field |
| `plugins/yarr/gemini-extension.json` | identity, repository, settings |
| `server.json` | package version and registry metadata, when present |

`Cargo.toml` is the canonical version source for this template. Use
`scripts/bump-version.sh` to update Cargo and `server.json` together, then use
`scripts/check-version-sync.sh` or `just pre-release` to verify that
version-bearing files still agree. Plugin manifests should remain versionless.

The template should not claim write capability unless the MCP server has real write actions. Read-only servers should use Codex `["Read"]` and avoid write-oriented sample prompts.

## Adaptation Checklist

When creating a real server from the template:

1. Rename `yarr`, `Yarr`, and `YARR` across plugin files.
2. Update all three manifests with the real repository, description, author, keywords, and capability claims.
3. Keep credential names aligned across Claude `userConfig`, Codex shared `.mcp.json`, and Gemini `settings`.
4. Replace upstream credential fields such as `yarr_api_url` and `yarr_api_key`.
5. Update binary-owned setup env mapping for service-specific plugin options.
6. Implement `<binary> setup plugin-hook`, `--no-repair`, `check`, and `repair`.
7. Update shared skill docs for the actual action surface.
8. Replace Codex `defaultPrompt` entries with realistic prompts.
9. Update Gemini `description`, `settings`, and `contextFileName` if needed.
10. Run `just validate-plugin`, plugin contract tests, and `scripts/check-plugin-hook-contract.py` before release.

## Required Tests

Each server should include tests that prove:

- Claude hook config runs `<binary> setup plugin-hook`
- `setup plugin-hook --no-repair` parses and does not mutate appdata
- JSON plugin-hook output contains `exit_policy`, `blocking_failures`, `advisory_failures`, `ran_repair`, and `no_repair`
- advisory failures exit `0`
- blocking failures exit nonzero
- Claude, Codex, and Gemini manifests use the same service name, endpoint, token setting, and credential fields
