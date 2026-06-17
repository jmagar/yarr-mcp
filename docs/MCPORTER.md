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

`mcporter` is used for legacy live MCP smoke testing and CLI generation. The
canonical full live suite is `cargo xtask live`.

## Test harness

The live test script is:

```bash
tests/mcporter/test-mcp.sh
```

Run it through Just:

```bash
just dev
just test-mcporter
```

For current live coverage, prefer:

```bash
cargo xtask live --suite mcp
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

The script targets `http://<RUSTARR_MCP_HOST>:<RUSTARR_MCP_PORT>/mcp`, defaulting to `http://localhost:40070/mcp` to match `just dev`. It remaps `0.0.0.0` to `localhost`. If `RUSTARR_MCP_TOKEN` is set, it sends `Authorization: Bearer <token>`. The script first runs the shart guard and loads `/home/jmagar/.rustarr-shart/.env`; it must not target production service credentials.

## What the test suite validates

- auth rejection when `RUSTARR_MCP_TOKEN` is set
- tool semantic behavior for service-named tools (`sonarr`, `radarr`, ...): `integrations`, `service_status`, `api_get`, and `help`
- MCP resource behavior for `rustarr://schema/mcp-tool`

> **Note:** these are the original generic actions. The action set is now
> **registry-derived** and much broader — it includes the generic passthroughs
> (`api_get`/`api_post`/`api_put`/`api_delete`) plus the curated, capability-scoped
> commands (e.g. `list`, `set_quality`, `download_queue`, `stats_activity`). Run
> the `help` action (or `rustarr help`) for the current full list, and prefer
> `cargo xtask live` for comprehensive coverage. This smoke test intentionally
> exercises only a representative subset.

The resource suite prefers mcporter resource commands when available and falls back to JSON-RPC `resources/read` for older mcporter versions. Bearer-auth tool calls fall back to JSON-RPC `tools/call` when the installed mcporter does not yet support HTTP headers on `mcporter call`.

## Test philosophy

Use semantic assertions, not liveness-only checks:

```bash
# Bad test — only proves MCP responded
run_test "server info" "sonarr" '{"action":"integrations"}'

# Good test — proves the service actually returned real data
run_test "inventory lists supported services" "sonarr" '{"action":"integrations"}' "supported"
run_test "help lists api_get" "sonarr" '{"action":"help"}' "examples.api_get"
```

A test that checks `is_error: false` is not a good test — it only verifies the MCP protocol layer responded. Semantic tests check that the actual service data is present and structurally correct.

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
