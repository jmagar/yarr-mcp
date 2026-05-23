#!/usr/bin/env bash
# SessionStart / ConfigChange hook for the Example MCP server plugin.
# Keep setup policy in the binary; this script only adapts plugin settings to env.
set -euo pipefail

: "${CLAUDE_PLUGIN_ROOT:=$(cd "$(dirname "$0")/.." && pwd)}"
: "${CLAUDE_PLUGIN_DATA:=${HOME}/.claude/plugins/data/example-jmagar-lab}"
: "${EXAMPLE_HOME:=${CLAUDE_PLUGIN_DATA}}"

reject_unsafe_value() {
  local name="$1" value="${2:-}"
  if [[ "${value}" == *$'\n'* || "${value}" == *$'\r'* ]]; then
    printf 'example plugin setup: %s must not contain newlines\n' "${name}" >&2
    exit 2
  fi
}

export_if_set() {
  local env_name="$1" option_name="$2" value
  value="$(printenv "${option_name}" || true)"
  reject_unsafe_value "${option_name}" "${value}"
  [[ -n "${value}" ]] || return 0
  export "${env_name}=${value}"
}

ensure_example_binary() {
  if command -v example >/dev/null 2>&1; then
    return 0
  fi

  local bundled="${CLAUDE_PLUGIN_ROOT}/bin/example"
  if [[ ! -x "${bundled}" ]]; then
    printf 'example plugin setup: bundled binary not found at %s\n' "${bundled}" >&2
    printf '  → run: just install   (builds release binary and copies to plugins/example/bin/)\n' >&2
    exit 1
  fi

  mkdir -p "${HOME}/.local/bin"
  ln -sf "${bundled}" "${HOME}/.local/bin/example"
  export PATH="${HOME}/.local/bin:${PATH}"

  command -v example >/dev/null 2>&1 || {
    printf 'example plugin setup: symlink created but example still not found in PATH\n' >&2
    printf '  → ensure %s is on your PATH\n' "${HOME}/.local/bin" >&2
    exit 1
  }
}

main() {
  reject_unsafe_value "CLAUDE_PLUGIN_OPTION_API_TOKEN" "${CLAUDE_PLUGIN_OPTION_API_TOKEN:-}"
  export_if_set EXAMPLE_MCP_TOKEN CLAUDE_PLUGIN_OPTION_API_TOKEN
  export_if_set EXAMPLE_SERVER_URL CLAUDE_PLUGIN_OPTION_SERVER_URL
  export_if_set EXAMPLE_API_URL CLAUDE_PLUGIN_OPTION_EXAMPLE_API_URL
  export_if_set EXAMPLE_API_KEY CLAUDE_PLUGIN_OPTION_EXAMPLE_API_KEY
  export_if_set EXAMPLE_MCP_AUTH_MODE CLAUDE_PLUGIN_OPTION_AUTH_MODE
  export_if_set EXAMPLE_MCP_NO_AUTH CLAUDE_PLUGIN_OPTION_NO_AUTH
  export_if_set EXAMPLE_MCP_PUBLIC_URL CLAUDE_PLUGIN_OPTION_PUBLIC_URL
  export_if_set EXAMPLE_MCP_GOOGLE_CLIENT_ID CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_ID
  export_if_set EXAMPLE_MCP_GOOGLE_CLIENT_SECRET CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_SECRET
  export_if_set EXAMPLE_MCP_AUTH_ADMIN_EMAIL CLAUDE_PLUGIN_OPTION_AUTH_ADMIN_EMAIL

  mkdir -p "${EXAMPLE_HOME}"
  chmod 700 "${EXAMPLE_HOME}" 2>/dev/null || true
  export EXAMPLE_HOME

  ensure_example_binary
  example setup plugin-hook "$@"
}

main "$@"
