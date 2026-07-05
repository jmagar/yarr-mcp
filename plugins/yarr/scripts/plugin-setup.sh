#!/usr/bin/env bash
# SessionStart / ConfigChange hook for the Yarr plugin.
set -euo pipefail

binary="${YARR_MCP_BIN:-yarr}"

if ! command -v "${binary}" >/dev/null 2>&1; then
  printf 'yarr plugin setup: yarr is not installed or not on PATH.\n' >&2
  printf 'Install yarr separately, then run: yarr setup\n' >&2
  exit 0
fi

exec "${binary}" setup plugin-hook "$@"
