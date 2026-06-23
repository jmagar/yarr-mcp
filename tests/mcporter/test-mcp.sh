#!/usr/bin/env bash
# Compatibility wrapper. The legacy `mcporter` live suite was retired (it assumed
# per-service MCP tools, which collapsed into the single `yarr` tool). MCP transport
# coverage now lives in `--suite mcp`; exhaustive spec-backed service coverage lives
# in `--suite contract`.
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.."
exec cargo xtask live --suite mcp "$@"
