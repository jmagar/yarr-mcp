#!/usr/bin/env bash
# SessionStart / ConfigChange hook for the Rustarr MCP server plugin.
# Keep setup policy in the binary; this script only adapts plugin settings to env.
set -euo pipefail

: "${CLAUDE_PLUGIN_ROOT:=$(cd "$(dirname "$0")/.." && pwd)}"
: "${CLAUDE_PLUGIN_DATA:=${HOME}/.claude/plugins/data/rustarr-jmagar-lab}"
: "${RUSTARR_HOME:=${CLAUDE_PLUGIN_DATA}}"

reject_unsafe_value() {
  local name="$1" value="${2:-}"
  if [[ "${value}" == *$'\n'* || "${value}" == *$'\r'* ]]; then
    printf 'rustarr plugin setup: %s must not contain newlines\n' "${name}" >&2
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

ensure_rustarr_binary() {
  if command -v rustarr >/dev/null 2>&1; then
    return 0
  fi

  local bundled="${CLAUDE_PLUGIN_ROOT}/bin/rustarr"
  if [[ ! -x "${bundled}" ]]; then
    printf 'rustarr plugin setup: bundled binary not found at %s\n' "${bundled}" >&2
    printf '  → run: just install   (builds release binary and copies to plugins/rustarr/bin/)\n' >&2
    exit 1
  fi

  mkdir -p "${HOME}/.local/bin"
  ln -sf "${bundled}" "${HOME}/.local/bin/rustarr"
  export PATH="${HOME}/.local/bin:${PATH}"

  command -v rustarr >/dev/null 2>&1 || {
    printf 'rustarr plugin setup: symlink created but rustarr still not found in PATH\n' >&2
    printf '  → ensure %s is on your PATH\n' "${HOME}/.local/bin" >&2
    exit 1
  }
}

main() {
  reject_unsafe_value "CLAUDE_PLUGIN_OPTION_API_TOKEN" "${CLAUDE_PLUGIN_OPTION_API_TOKEN:-}"
  export_if_set RUSTARR_MCP_TOKEN CLAUDE_PLUGIN_OPTION_API_TOKEN
  export_if_set RUSTARR_SERVER_URL CLAUDE_PLUGIN_OPTION_SERVER_URL
  export_if_set RUSTARR_SERVICES CLAUDE_PLUGIN_OPTION_RUSTARR_SERVICES
  export_if_set RUSTARR_SONARR_URL CLAUDE_PLUGIN_OPTION_SONARR_URL
  export_if_set RUSTARR_SONARR_API_KEY CLAUDE_PLUGIN_OPTION_SONARR_API_KEY
  export_if_set RUSTARR_RADARR_URL CLAUDE_PLUGIN_OPTION_RADARR_URL
  export_if_set RUSTARR_RADARR_API_KEY CLAUDE_PLUGIN_OPTION_RADARR_API_KEY
  export_if_set RUSTARR_PROWLARR_URL CLAUDE_PLUGIN_OPTION_PROWLARR_URL
  export_if_set RUSTARR_PROWLARR_API_KEY CLAUDE_PLUGIN_OPTION_PROWLARR_API_KEY
  export_if_set RUSTARR_OVERSEERR_URL CLAUDE_PLUGIN_OPTION_OVERSEERR_URL
  export_if_set RUSTARR_OVERSEERR_API_KEY CLAUDE_PLUGIN_OPTION_OVERSEERR_API_KEY
  export_if_set RUSTARR_QBITTORRENT_URL CLAUDE_PLUGIN_OPTION_QBITTORRENT_URL
  export_if_set RUSTARR_QBITTORRENT_USERNAME CLAUDE_PLUGIN_OPTION_QBITTORRENT_USERNAME
  export_if_set RUSTARR_QBITTORRENT_PASSWORD CLAUDE_PLUGIN_OPTION_QBITTORRENT_PASSWORD
  export_if_set RUSTARR_PLEX_URL CLAUDE_PLUGIN_OPTION_PLEX_URL
  export_if_set RUSTARR_PLEX_TOKEN CLAUDE_PLUGIN_OPTION_PLEX_TOKEN
  export_if_set RUSTARR_JELLYFIN_URL CLAUDE_PLUGIN_OPTION_JELLYFIN_URL
  export_if_set RUSTARR_JELLYFIN_API_KEY CLAUDE_PLUGIN_OPTION_JELLYFIN_API_KEY
  export_if_set RUSTARR_MCP_AUTH_MODE CLAUDE_PLUGIN_OPTION_AUTH_MODE
  export_if_set RUSTARR_MCP_NO_AUTH CLAUDE_PLUGIN_OPTION_NO_AUTH
  export_if_set RUSTARR_MCP_PUBLIC_URL CLAUDE_PLUGIN_OPTION_PUBLIC_URL
  export_if_set RUSTARR_MCP_GOOGLE_CLIENT_ID CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_ID
  export_if_set RUSTARR_MCP_GOOGLE_CLIENT_SECRET CLAUDE_PLUGIN_OPTION_GOOGLE_CLIENT_SECRET
  export_if_set RUSTARR_MCP_AUTH_ADMIN_EMAIL CLAUDE_PLUGIN_OPTION_AUTH_ADMIN_EMAIL

  mkdir -p "${RUSTARR_HOME}"
  if ! chmod 700 "${RUSTARR_HOME}" 2>/dev/null; then
    printf 'rustarr plugin setup: warning: could not chmod 700 %s; check directory ownership and permissions\n' "${RUSTARR_HOME}" >&2
  fi
  export RUSTARR_HOME

  ensure_rustarr_binary
  rustarr setup plugin-hook "$@"
}

main "$@"
