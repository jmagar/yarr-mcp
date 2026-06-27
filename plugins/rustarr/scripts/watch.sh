#!/usr/bin/env bash
# Claude monitor entry point. Uses an installed rustarr from PATH.
set -euo pipefail

binary="${RUSTARR_MCP_BIN:-rustarr}"

if ! command -v "${binary}" >/dev/null 2>&1; then
  printf 'rustarr monitor: rustarr is not installed or not on PATH.\n' >&2
  exit 0
fi

exec "${binary}" watch "$@"
