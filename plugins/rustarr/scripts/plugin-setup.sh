#!/usr/bin/env bash
# SessionStart / ConfigChange hook for the Rustarr plugin.
set -euo pipefail

binary="${RUSTARR_MCP_BIN:-rustarr}"

if ! command -v "${binary}" >/dev/null 2>&1; then
  printf 'rustarr plugin setup: rustarr is not installed or not on PATH.\n' >&2
  printf 'Install rustarr separately, then run: rustarr setup\n' >&2
  exit 0
fi

exec "${binary}" setup plugin-hook "$@"
