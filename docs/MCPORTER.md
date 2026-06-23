---
title: "mcporter Integration Testing"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-05-15"
---

# mcporter

`mcporter` is an external CLI for ad-hoc MCP probing and CLI generation. It can call
the single published `yarr` tool directly (see *Test philosophy* below).

> **Retired suite.** The dedicated `cargo xtask live --suite mcporter` harness was
> removed. It assumed one MCP tool *per service* and exercised hand-written curated
> commands for every service — both gone since the MCP surface collapsed to a single
> `yarr` Code Mode tool and the 6 spec-backed services (sonarr/radarr/prowlarr/
> overseerr/jellyfin/plex) moved to generated OpenAPI operations. Its coverage is now
> split across the suites below.

## Live coverage

```bash
cargo xtask live --suite mcp        # MCP transport: handshake, resources, prompts, yarr round-trip
cargo xtask live --suite contract   # exhaustive generated-operation coverage for the 6 spec-backed services
cargo xtask live --suite cli        # service-grouped CLI reads/writes per the service matrix
cargo xtask live --suite all        # everything above + rest/services
```

`tests/mcporter/test-mcp.sh` is a thin compatibility wrapper that now runs
`--suite mcp`.

## Configuration

```json
{
  "mcpServers": {
    "rustarr": {
      "url": "http://localhost:40070/mcp",
      "transport": "http"
    }
  }
}
```

The live suites start a local Rustarr MCP server against `/home/jmagar/.rustarr-shart/.env` and build/use `target/debug/rustarr` by default so they test the current checkout; set `RUSTARR_BIN=/path/to/rustarr` only when intentionally testing a specific binary. They must not target production service credentials. Shart is a disposable fake stack, so confirmed media-stack writes/removes/deletes are expected live coverage.

## What the live suites validate

- **`mcp`** — `tools/list` advertises exactly the single `yarr` tool (no per-service tools); `initialize`, `resources/read` (the schema resource), and `prompts/get quick_start` succeed; a representative `yarr` Code Mode round-trip reaches an upstream service and returns real status fields; a write to a bad path surfaces the service-native error through the Code Mode envelope.
- **`contract`** — drives *every* generated OpenAPI operation for the 6 spec-backed services via the CLI `op` action, with create-first seeding and schema-validated responses. Destructive DELETEs are gated by `--no-destructive` / `RUSTARR_ALLOW_DESTRUCTIVE`.
- **`cli`** — service-grouped CLI reads (`rustarr <service> status`/`get`), per-service `service_status`, matrix-backed `api_get` expectations, and an unconfirmed `api_post` upstream-error probe per service. Writes run immediately; only destructive deletes are gated.
- Seeded-content assertions prove the test stack is not merely returning empty success: Prowlarr exposes the `Rustarr Live LinuxTracker` indexer, Plex/Jellyfin expose `Rustarr Live Movies` / `Rustarr Fixture Movie`, and Tautulli returns that library.
- "Destructive" means permanent loss of data that cannot be quickly and easily regenerated or recreated with minimal effort. Ordinary media-stack writes such as removing a test torrent, deleting re-downloadable media, clearing OAuth tokens, stopping containers, killing restartable processes, or toggling gateway state are mutating, but not destructive under this project vocabulary.

> **Coverage gap (tracked):** doc-based-service stateful lifecycles (sabnzbd/qbittorrent
> `download_*`, tautulli `stats_*` maintenance, bazarr/tracearr seeded `api_delete`
> cleanup) were previously exercised by the retired mcporter suite and are not yet
> re-homed against the `yarr`/`op` surface. Re-adding them (and re-deriving
> `docs/LIVE_ENDPOINT_COVERAGE.md`) needs a live shart run.

If a protected live action lacks credentials in `/home/jmagar/.rustarr-shart/.env`, the suite should fail. That is a live stack setup issue, not a harness success.

## Test philosophy

Use semantic assertions, not liveness-only checks:

```bash
# Bad test — only proves MCP responded.
mcporter call --http-url http://127.0.0.1:40170/mcp --allow-http --tool yarr --args '{"code":"async () => sonarr.service_status()"}'

# Good test — the xtask validates the actual payload.
cargo xtask live --suite mcp
```

A test that checks `is_error: false` is not a good test — it only verifies the MCP protocol layer responded. Semantic tests check that the actual service data is present, structurally correct, and changed as expected for confirmed writes.

## Tool validation helpers

```bash
# Validate that a JSON path exists and is non-empty
assert_key() {
  local label="$1" output="$2" key_path="$3"
  python3 -c "
import sys, json
d = json.loads('''${output}''')
keys = '${key_path}'.split('.')
node = d
for k in keys:
    node = node[int(k)] if isinstance(node, list) and k.isdigit() else node[k]
assert node is not None and node != '' and node != [] and node != {}
" 2>/dev/null || { echo \"[FAIL] ${label}: missing or empty .${key_path}\"; return 1; }
}
```

## Resource validation

MCP resources are public contract, not implementation detail. Test every stable resource URI:

- The resource URI resolves.
- The returned content parses as JSON.
- Service tool names such as `sonarr` and `radarr` are present.
- `inputSchema.type` is `object`.
- `inputSchema.properties.action` exists.

## Generated CLIs

`just generate-cli` demonstrates generating a standalone CLI from a running MCP server. Generated CLIs may embed auth material; do not commit them unless they are intentionally scrubbed and reviewed.

See `docs/PATTERNS.md` §17 for the full mcporter integration test pattern.
