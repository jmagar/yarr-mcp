#!/usr/bin/env bash
# Live read-only smoke checks for the shart test rustarr environment.
set -euo pipefail

BIN="${RUSTARR_BIN:-rustarr}"
SHART_RUSTARR_HOME="${SHART_RUSTARR_HOME:-/home/jmagar/.rustarr-shart}"

if [[ -z "${RUSTARR_HOME:-}" ]]; then
  export RUSTARR_HOME="$SHART_RUSTARR_HOME"
elif [[ "$RUSTARR_HOME" != "$SHART_RUSTARR_HOME" ]]; then
  printf 'FATAL  live-read-smoke may only use RUSTARR_HOME=%s (got %s)\n' \
    "$SHART_RUSTARR_HOME" "$RUSTARR_HOME" >&2
  exit 2
fi

PASS=0
FAIL=0

pass() {
  printf 'PASS  %s\n' "$1"
  PASS=$((PASS + 1))
}

fail() {
  printf 'FAIL  %s\n' "$1" >&2
  FAIL=$((FAIL + 1))
}

assert_shart_services() {
  python3 - <<'PY'
import os
import sys
from pathlib import Path
from urllib.parse import urlparse

allowed_hosts = {
    "100.118.209.1",
    "shart",
    "shart.manatee-triceratops.ts.net",
}

def parse_dotenv_value(raw: str) -> str:
    raw = raw.strip()
    if len(raw) >= 2 and raw[0] == '"' and raw[-1] == '"':
        out = []
        chars = iter(range(1, len(raw) - 1))
        i = 1
        while i < len(raw) - 1:
            ch = raw[i]
            if ch == "\\" and i + 1 < len(raw) - 1:
                nxt = raw[i + 1]
                out.append("\n" if nxt == "n" else nxt)
                i += 2
            else:
                out.append(ch)
                i += 1
        return "".join(out)
    return raw

effective = {}
home = Path(os.environ["RUSTARR_HOME"])
env_path = home / ".env"
if not env_path.is_file():
    print(f"FATAL  shart smoke env file is missing: {env_path}", file=sys.stderr)
    sys.exit(2)

for line_no, raw_line in enumerate(env_path.read_text().splitlines(), start=1):
    line = raw_line.strip()
    if not line or line.startswith("#"):
        continue
    if "=" not in line:
        print(f"FATAL  {env_path}:{line_no}: expected KEY=VALUE", file=sys.stderr)
        sys.exit(2)
    key, raw_value = line.split("=", 1)
    key = key.strip()
    if key and key not in os.environ:
        effective[key] = parse_dotenv_value(raw_value)

for key, value in os.environ.items():
    if key.startswith("RUSTARR_"):
        effective[key] = value

services = [item.strip() for item in effective.get("RUSTARR_SERVICES", "").split(",") if item.strip()]
if not services:
    print("FATAL  RUSTARR_SERVICES is empty in the shart smoke environment", file=sys.stderr)
    sys.exit(2)

bad = []
missing = []
for service in services:
    env_name = "".join(ch.upper() if ch.isalnum() else "_" for ch in service)
    key = f"RUSTARR_{env_name}_URL"
    raw_url = effective.get(key, "")
    if not raw_url:
        missing.append(key)
        continue
    parsed = urlparse(raw_url)
    host = (parsed.hostname or "").lower()
    if host not in allowed_hosts:
        bad.append(f"{key}={raw_url}")

if missing or bad:
    if missing:
        print("FATAL  missing service URLs: " + ", ".join(missing), file=sys.stderr)
    if bad:
        print("FATAL  refusing to run live-read-smoke against non-shart services:", file=sys.stderr)
        for item in bad:
            print(f"  {item}", file=sys.stderr)
    sys.exit(2)
PY
  pass "shart service guard"
}

run_json_check() {
  local label="$1"
  shift
  local output
  if ! output="$("$@" 2>&1)"; then
    fail "$label ($(printf '%s' "$output" | tr -d '\n' | cut -c1-200))"
    return
  fi
  if ! printf '%s' "$output" | python3 -m json.tool >/dev/null 2>&1; then
    fail "$label (output was not JSON)"
    return
  fi
  pass "$label"
}

run_status_check() {
  local service="$1"
  local output
  if ! output="$("$BIN" status --service "$service" 2>&1)"; then
    fail "status $service ($(printf '%s' "$output" | tr -d '\n' | cut -c1-200))"
    return
  fi
  if [[ -z "$output" ]]; then
    fail "status $service (empty output)"
    return
  fi
  pass "status $service"
}

run_get_check() {
  local service="$1"
  local path="$2"
  run_json_check "get $service $path" "$BIN" get --service "$service" --path "$path"
}

read_probe_paths() {
  local kind="$1"
  case "$kind" in
    sonarr)
      printf '%s\n' \
        "/api/v3/system/status" \
        "/api/v3/system/backup" \
        "/api/v3/system/task" \
        "/api/v3/series" \
        "/api/v3/queue" \
        "/api/v3/history" \
        "/api/v3/calendar" \
        "/api/v3/tag" \
        "/api/v3/rootfolder" \
        "/api/v3/qualityprofile" \
        "/api/v3/languageprofile" \
        "/api/v3/metadata" \
        "/api/v3/indexer" \
        "/api/v3/downloadclient" \
        "/api/v3/notification" \
        "/api/v3/health" \
        "/api/v3/log/file" \
        "/api/v3/update"
      ;;
    radarr)
      printf '%s\n' \
        "/api/v3/system/status" \
        "/api/v3/system/backup" \
        "/api/v3/system/task" \
        "/api/v3/movie" \
        "/api/v3/queue" \
        "/api/v3/history" \
        "/api/v3/calendar" \
        "/api/v3/tag" \
        "/api/v3/rootfolder" \
        "/api/v3/qualityprofile" \
        "/api/v3/metadata" \
        "/api/v3/indexer" \
        "/api/v3/downloadclient" \
        "/api/v3/notification" \
        "/api/v3/health" \
        "/api/v3/log/file" \
        "/api/v3/update"
      ;;
    prowlarr)
      printf '%s\n' \
        "/api/v1/system/status" \
        "/api/v1/system/backup" \
        "/api/v1/system/task" \
        "/api/v1/indexer" \
        "/api/v1/indexerstats" \
        "/api/v1/applications" \
        "/api/v1/downloadclient" \
        "/api/v1/notification" \
        "/api/v1/tag" \
        "/api/v1/health" \
        "/api/v1/log/file" \
        "/api/v1/update"
      ;;
    tautulli)
      printf '%s\n' \
        "/api/v2?cmd=get_server_info" \
        "/api/v2?cmd=get_activity" \
        "/api/v2?cmd=get_libraries" \
        "/api/v2?cmd=get_library_names" \
        "/api/v2?cmd=get_home_stats" \
        "/api/v2?cmd=get_recently_added" \
        "/api/v2?cmd=get_users" \
        "/api/v2?cmd=get_history" \
        "/api/v2?cmd=get_plays_by_date"
      ;;
    overseerr)
      printf '%s\n' \
        "/api/v1/status" \
        "/api/v1/settings/public"
      ;;
    bazarr)
      printf '%s\n' \
        "/api/system/status" \
        "/api/system/health" \
        "/api/series" \
        "/api/movies" \
        "/api/badges" \
        "/api/providers"
      ;;
    tracearr)
      printf '%s\n' "/health"
      ;;
    lidarr|readarr)
      printf '%s\n' "/api/v1/system/status"
      ;;
    sabnzbd)
      printf '%s\n' \
        "/api?mode=version" \
        "/api?mode=queue" \
        "/api?mode=history" \
        "/api?mode=server_stats" \
        "/api?mode=get_config"
      ;;
    qbittorrent)
      printf '%s\n' \
        "/api/v2/app/version" \
        "/api/v2/app/webapiVersion" \
        "/api/v2/app/buildInfo" \
        "/api/v2/app/preferences" \
        "/api/v2/torrents/info" \
        "/api/v2/torrents/categories" \
        "/api/v2/torrents/tags" \
        "/api/v2/transfer/info" \
        "/api/v2/sync/maindata"
      ;;
    wizarr)
      printf '%s\n' "/api/status"
      ;;
    notifiarr)
      printf '%s\n' "/api/ping"
      ;;
    plex)
      printf '%s\n' "/identity"
      ;;
    jellyfin)
      printf '%s\n' "/System/Info/Public"
      ;;
  esac
}

services_from_integrations() {
  "$BIN" integrations | python3 -c '
import json
import sys

payload = json.load(sys.stdin)
for service in payload.get("configured", []):
    name = service.get("name")
    kind = service.get("kind")
    if name and kind:
        print(f"{name}\t{kind}")
'
}

assert_shart_services

run_json_check "help" "$BIN" help
run_json_check "integrations" "$BIN" integrations
run_json_check "doctor" "$BIN" doctor --json

mapfile -t services < <(services_from_integrations)
if (( ${#services[@]} == 0 )); then
  fail "configured services (none returned by integrations)"
else
  pass "configured services (${#services[@]})"
fi

for entry in "${services[@]}"; do
  service="${entry%%$'\t'*}"
  kind="${entry#*$'\t'}"
  run_status_check "$service"
  mapfile -t paths < <(read_probe_paths "$kind")
  if (( ${#paths[@]} == 0 )); then
    fail "read probes $service (no read-only probe paths for kind=$kind)"
    continue
  fi
  for path in "${paths[@]}"; do
    run_get_check "$service" "$path"
  done
done

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
[[ "$FAIL" -eq 0 ]]
