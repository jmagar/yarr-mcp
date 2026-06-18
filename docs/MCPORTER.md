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

`mcporter` is used for live MCP transport testing and CLI generation. The
canonical mcporter harness is now driven by `cargo xtask live --suite mcporter`.

## Test harness

The compatibility wrapper is:

```bash
tests/mcporter/test-mcp.sh
```

Run it through Just:

```bash
just test-mcporter
```

For current live coverage, prefer:

```bash
cargo xtask live --suite mcporter
cargo xtask live --suite all
```

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

The xtask starts a local Rustarr MCP server against `/home/jmagar/.rustarr-shart/.env`, discovers the advertised service tool schemas through `mcporter list --schema`, and calls each advertised action through `mcporter call`. It must not target production service credentials.

## What the test suite validates

- `mcporter list --schema` advertises exactly the shart matrix service tools.
- Every advertised action for every advertised service tool is called.
- Read actions must pass semantic payload assertions, such as configured service inventory, real service status fields, matrix-backed `api_get` expectations, help text containing current actions, and action-specific JSON shape checks.
- Seeded fixture assertions prove the test stack is not merely returning empty success: Prowlarr must expose/search the `Rustarr Live LinuxTracker` indexer, Plex and Jellyfin must expose/search `Rustarr Live Movies` / `Rustarr Fixture Movie`, and Tautulli must return that library through its inventory command.
- Mutating actions are invoked through `confirm=false` guard/preview fixtures and, on the shart test stack, confirmed stateful write lifecycles. Confirmed lifecycle checks must validate observable before/after state and cleanup, not merely `is_error=false`.
- Confirmed write coverage includes Arr tag create/update/delete through generic `api_post`/`api_put`/`api_delete`, Sonarr/Radarr media add/monitor/unmonitor/set-quality/search/refresh/delete lifecycles, Prowlarr `indexer_test`, Overseerr request create/approve/decline/delete cleanup, Jellyfin library scan, SABnzbd add/pause/resume/remove against a local NZB fixture, and qBittorrent add/pause/resume/remove against a test magnet. Plex is confirmed against the live fixture state: if shart has a library, the suite runs `media_scan`; if not, it asserts the expected missing-library error shape.
- Confirmed generic write-error coverage also calls `api_post`, `api_put`, and `api_delete` with `confirm=true` for every service and requires either a stable MCP execution error or a service-native error body. This proves the call reached the upstream path and was not stopped by the confirm guard.
- "Destructive" means permanent loss of data that cannot be quickly and easily regenerated or recreated with minimal effort. Ordinary media-stack writes such as removing a test torrent, deleting re-downloadable media, clearing OAuth tokens, stopping containers, killing restartable processes, or toggling gateway state are mutating, but not destructive under this project vocabulary.

If a protected live action lacks credentials in `/home/jmagar/.rustarr-shart/.env`, the suite should fail. That is a live stack setup issue, not a harness success.

## Test philosophy

Use semantic assertions, not liveness-only checks:

```bash
# Bad test — only proves MCP responded.
mcporter call --http-url http://127.0.0.1:40170/mcp --allow-http --tool sonarr --args '{"action":"integrations"}'

# Good test — the xtask validates the actual payload.
cargo xtask live --suite mcporter
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
