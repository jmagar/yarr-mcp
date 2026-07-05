#!/usr/bin/env bash
# Smoke-test the HTTP MCP static bearer-token gate.
set -euo pipefail

MCP_URL="${YARR_MCP_URL:-http://localhost:3000/mcp}"
TOKEN="${YARR_MCP_TOKEN:-}"
TIMEOUT="${MCP_AUTH_TIMEOUT:-10}"
CHECK_X_API_KEY=false

usage() {
  cat <<'EOF'
Usage: scripts/test-mcp-auth.sh [OPTIONS]

Options:
  --url URL              MCP URL. Default: YARR_MCP_URL or http://localhost:3000/mcp.
  --token TOKEN          Expected static bearer token. Default: YARR_MCP_TOKEN.
  --check-x-api-key      Also require x-api-key auth to succeed. Off by default because
                         the template's pinned lab-auth layer only supports Bearer.
  -h, --help             Show this help.

Checks:
  - /health is reachable without auth
  - /mcp rejects missing bearer token with 401
  - /mcp rejects a bad bearer token with 401
  - /mcp accepts Authorization: Bearer <token>
  - x-api-key is skipped unless --check-x-api-key is set
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --url)
      MCP_URL="${2:?--url requires a value}"
      shift 2
      ;;
    --token)
      TOKEN="${2:?--token requires a value}"
      shift 2
      ;;
    --check-x-api-key)
      CHECK_X_API_KEY=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$TOKEN" ]]; then
  echo "ERROR: set YARR_MCP_TOKEN or pass --token" >&2
  exit 2
fi

BASE_URL="${MCP_URL%/mcp}"
BAD_TOKEN="intentionally-bad-token"
PASS=0
FAIL=0

request_body='{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'

pass() {
  printf 'PASS  %s\n' "$1"
  PASS=$((PASS + 1))
}

fail() {
  printf 'FAIL  %s\n' "$1" >&2
  FAIL=$((FAIL + 1))
}

http_code() {
  curl -sS --max-time "$TIMEOUT" -o /tmp/yarr-auth-body.txt -w '%{http_code}' "$@"
}

expect_code() {
  local label="$1" expected="$2"
  shift 2
  local code
  if ! code="$(http_code "$@")"; then
    fail "$label (curl failed)"
    return
  fi
  if [[ "$code" == "$expected" ]]; then
    pass "$label"
  else
    fail "$label (expected HTTP $expected, got $code; body: $(tr -d '\n' </tmp/yarr-auth-body.txt | cut -c1-200))"
  fi
}

expect_success_jsonrpc() {
  local label="$1"
  shift
  local code body_check
  if ! code="$(http_code "$@")"; then
    fail "$label (curl failed)"
    return
  fi
  if [[ "$code" != "200" ]]; then
    fail "$label (expected HTTP 200, got $code; body: $(tr -d '\n' </tmp/yarr-auth-body.txt | cut -c1-200))"
    return
  fi
  body_check="$(python3 - <<'PY'
import json, sys
try:
    data = json.load(open("/tmp/yarr-auth-body.txt", encoding="utf-8"))
    tools = data.get("result", {}).get("tools", [])
    print("ok" if isinstance(tools, list) and tools else "missing tools")
except Exception as exc:
    print(f"parse error: {exc}")
PY
)"
  if [[ "$body_check" == "ok" ]]; then
    pass "$label"
  else
    fail "$label ($body_check)"
  fi
}

expect_code "health is public" "200" "${BASE_URL}/health"

expect_code "missing bearer token is rejected" "401" \
  -X POST "$MCP_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d "$request_body"

expect_code "bad bearer token is rejected" "401" \
  -X POST "$MCP_URL" \
  -H "Authorization: Bearer ${BAD_TOKEN}" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d "$request_body"

expect_success_jsonrpc "valid bearer token is accepted" \
  -X POST "$MCP_URL" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d "$request_body"

if [[ "$CHECK_X_API_KEY" == true ]]; then
  expect_success_jsonrpc "x-api-key token is accepted" \
    -X POST "$MCP_URL" \
    -H "x-api-key: ${TOKEN}" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    -d "$request_body"
else
  printf 'SKIP  x-api-key acceptance (pass --check-x-api-key only for services that implement it)\n'
fi

rm -f /tmp/yarr-auth-body.txt

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
[[ "$FAIL" -eq 0 ]]
