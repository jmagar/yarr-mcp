#!/usr/bin/env bash
# Claude monitor entry point. Uses an installed yarr from PATH.
set -euo pipefail

binary="${YARR_MCP_BIN:-yarr}"

if ! command -v "${binary}" >/dev/null 2>&1; then
  printf 'yarr monitor: yarr is not installed or not on PATH.\n' >&2
  exit 0
fi

exec "${binary}" watch "$@"
