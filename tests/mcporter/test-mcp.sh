#!/usr/bin/env bash
# Compatibility wrapper for the mcporter-backed MCP contract suite.
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.."
exec cargo xtask live --suite mcporter "$@"
