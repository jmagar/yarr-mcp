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

`mcporter` is used for live MCP integration testing and CLI generation.

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

## Configuration

```json
{
  "mcpServers": {
    "rustarr": {
      "url": "http://localhost:40060/mcp",
      "transport": "http"
    }
  }
}
```

The script targets `http://<RUSTARR_MCP_HOST>:<RUSTARR_MCP_PORT>/mcp`, defaulting to `http://localhost:40060/mcp` to match `just dev`. It remaps `0.0.0.0` to `localhost`. If `RUSTARR_MCP_TOKEN` is set, it sends `Authorization: Bearer <token>`.

## What the test suite validates

- auth rejection when `RUSTARR_MCP_TOKEN` is set
- tool semantic behavior for `integrations`, `service_status`, `api_get`, and `help`
- MCP resource behavior for `rustarr://schema/mcp-tool`

The resource suite prefers mcporter resource commands when available and falls back to JSON-RPC `resources/read` for older mcporter versions. Bearer-auth tool calls fall back to JSON-RPC `tools/call` when the installed mcporter does not yet support HTTP headers on `mcporter call`.

## Test philosophy

Use semantic assertions, not liveness-only checks:

```bash
# Bad test — only proves MCP responded
run_test "server info" "rustarr" '{"action":"integrations"}'

# Good test — proves the service actually returned real data
run_test "inventory lists supported services" "rustarr" '{"action":"integrations"}' "supported"
run_test "help lists api_get" "rustarr" '{"action":"help"}' "examples.api_get"
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
- The tool name is `rustarr`.
- `inputSchema.type` is `object`.
- `inputSchema.properties.action` exists.

## Generated CLIs

`just generate-cli` demonstrates generating a standalone CLI from a running MCP server. Generated CLIs may embed auth material; do not commit them unless they are intentionally scrubbed and reviewed.

See `docs/PATTERNS.md` §17 for the full mcporter integration test pattern.
