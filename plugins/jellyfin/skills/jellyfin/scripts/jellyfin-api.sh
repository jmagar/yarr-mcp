#!/usr/bin/env bash
# Jellyfin API helper.
# Usage: jellyfin-api.sh <command> [args...]

set -euo pipefail

load_config() {
  local config="${JELLYFIN_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/lab-jellyfin/config.env}"
  if [[ -f "$config" ]]; then
    set -a
    # shellcheck source=/dev/null
    source "$config"
    set +a
  elif [[ -f "$HOME/.lab/.env" ]]; then
    set -a
    # shellcheck source=/dev/null
    source "$HOME/.lab/.env"
    set +a
  fi

  : "${JELLYFIN_URL:?set JELLYFIN_URL in plugin settings or ~/.lab/.env}"
  : "${JELLYFIN_API_KEY:?set JELLYFIN_API_KEY in plugin settings or ~/.lab/.env}"
  JELLYFIN_URL="${JELLYFIN_URL%/}"
}

api() {
  local method="$1"
  local endpoint="$2"
  shift 2
  curl -sS -X "$method" \
    -H "Accept: application/json" \
    -H "X-Emby-Token: ${JELLYFIN_API_KEY}" \
    "$@" \
    "${JELLYFIN_URL}${endpoint}"
}

urlencode() {
  jq -rn --arg v "$1" '$v|@uri'
}

require_id() {
  # Jellyfin item ids are GUID/alphanumeric; reject anything that could alter the path/query.
  [[ "${1:-}" =~ ^[A-Za-z0-9-]+$ ]] || { echo "ERROR: invalid item id '${1:-}'" >&2; exit 2; }
}

usage() {
  cat <<'EOF'
Usage: jellyfin-api.sh <command> [args...]

Commands:
  info                         Server info
  users                        List users
  sessions                     Active sessions
  libraries                    Library virtual folders
  tasks                        Scheduled tasks
  devices                      Known devices
  search <term> [--limit N]    Search items
  item <id>                    Item details
  refresh <item-id>            Refresh metadata for an item (write)

Environment:
  JELLYFIN_URL and JELLYFIN_API_KEY from lab-jellyfin config or ~/.lab/.env.
EOF
}

cmd="${1:-help}"
shift || true
case "$cmd" in
  help|-h|--help) usage; exit 0 ;;
esac
load_config

case "$cmd" in
  info) api GET "/System/Info" ;;
  users) api GET "/Users" ;;
  sessions) api GET "/Sessions" ;;
  libraries) api GET "/Library/VirtualFolders" ;;
  tasks) api GET "/ScheduledTasks" ;;
  devices) api GET "/Devices" ;;
  search)
    term="${1:?search term required}"; shift
    limit="25"
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --limit|-l) limit="${2:?limit required}"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
      esac
    done
    api GET "/Items?Recursive=true&SearchTerm=$(urlencode "$term")&Limit=${limit}"
    ;;
  item)
    id="${1:?item id required}"
    require_id "$id"
    api GET "/Items/${id}"
    ;;
  refresh)
    id="${1:?item id required}"
    require_id "$id"
    api POST "/Items/${id}/Refresh?Recursive=true&MetadataRefreshMode=Default&ImageRefreshMode=Default&ReplaceAllMetadata=false&ReplaceAllImages=false"
    printf '{"status":"ok","message":"refresh requested","itemId":"%s"}\n' "$id"
    ;;
  *) usage >&2; exit 2 ;;
esac
