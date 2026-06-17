#!/usr/bin/env bash
# =============================================================================
# test-mcp.sh — Integration smoke-test for the Rustarr MCP server
# PHILOSOPHY — what makes a good integration test:
#   A test that only checks "did it return JSON?" is NOT a good test.
#   A test that checks "did integrations include sonarr?" IS a good test — it
#   proves semantic correctness, not just liveness.
#
#   This script demonstrates the pattern with four checks:
#     integrations()         → response MUST list supported services
#     service_status()       → response MUST inspect a configured service when env is present
#     api_get()              → response MUST return upstream data when env is present
#     help()                 → response MUST list the current action names
#     schema resource        → MUST include service-named schemas with inputSchema
# Server is assumed to be running as HTTP on localhost:40070 (the `just dev` port).
# Credentials are sourced from ~/.rustarr/.env OR environment variables:
#   RUSTARR_MCP_HOST  (default: localhost)
#   RUSTARR_MCP_PORT  (default: 40070)
#   RUSTARR_MCP_TOKEN (optional; omit for no-auth dev mode)
#
# Usage:
#   ./tests/mcporter/test-mcp.sh [--timeout-ms N] [--parallel] [--verbose]
#
# Options:
#   --timeout-ms N   Per-call timeout in milliseconds (default: 15000)
#   --parallel       Run test suites in parallel (default: sequential)
#   --verbose        Print raw mcporter output for each call
#
# Exit codes:
#   0 — all tests passed or skipped
#   1 — one or more tests failed
#   2 — prerequisite check failed (mcporter not found, server unreachable)
#
# TEMPLATE: To add more actions, copy the pattern from suite_core below:
#   run_test_semantic "label" "tool" '{"action":"your_action","arg":"val"}' \
#     "response_key" "expected_substring_or_exact_value" "exact"
# =============================================================================

set -uo pipefail

# ── Constants ─────────────────────────────────────────────────────────────────
readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_DIR="$(cd -- "${SCRIPT_DIR}/../.." && pwd -P)"
readonly SCRIPT_NAME="$(basename -- "${BASH_SOURCE[0]}")"
readonly TS_START="$(date +%s%N)"
readonly LOG_FILE="${TMPDIR:-/tmp}/${SCRIPT_NAME%.sh}.$(date +%Y%m%d-%H%M%S).log"

# TEMPLATE: Change this path if your credentials live elsewhere.
#           rustarr uses ~/.rustarr/.env; adapt to your convention.
readonly ENV_FILE="${HOME}/.rustarr-shart/.env"

# ── Colour support ────────────────────────────────────────────────────────────
if [[ -t 1 ]]; then
  C_RESET='\033[0m' C_BOLD='\033[1m' C_GREEN='\033[0;32m'
  C_RED='\033[0;31m' C_YELLOW='\033[0;33m' C_CYAN='\033[0;36m' C_DIM='\033[2m'
else
  C_RESET='' C_BOLD='' C_GREEN='' C_RED='' C_YELLOW='' C_CYAN='' C_DIM=''
fi

# ── Defaults ──────────────────────────────────────────────────────────────────
CALL_TIMEOUT_MS=15000
USE_PARALLEL=false
VERBOSE=false

# ── Counters ──────────────────────────────────────────────────────────────────
PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0
declare -a FAIL_NAMES=()

# Runtime globals — populated in load_env
MCP_URL=''
MCPORTER_HEADER_ARGS=()

# ── Argument parsing ──────────────────────────────────────────────────────────
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --timeout-ms) CALL_TIMEOUT_MS="${2:?--timeout-ms requires a value}"; shift 2 ;;
      --parallel)   USE_PARALLEL=true; shift ;;
      --verbose)    VERBOSE=true; shift ;;
      -h|--help)
        printf 'Usage: %s [--timeout-ms N] [--parallel] [--verbose]\n' "${SCRIPT_NAME}"
        exit 0
        ;;
      *) printf '[ERROR] Unknown argument: %s\n' "$1" >&2; exit 2 ;;
    esac
  done
}

# ── Logging ───────────────────────────────────────────────────────────────────
log_info()  { printf "${C_CYAN}[INFO]${C_RESET}  %s\n" "$*" | tee -a "${LOG_FILE}"; }
log_warn()  { printf "${C_YELLOW}[WARN]${C_RESET}  %s\n" "$*" | tee -a "${LOG_FILE}"; }
log_error() { printf "${C_RED}[ERROR]${C_RESET} %s\n" "$*" | tee -a "${LOG_FILE}" >&2; }

cleanup() {
  local rc=$?
  [[ $rc -ne 0 ]] && log_warn "Script exited with rc=${rc}. Log: ${LOG_FILE}"
}
trap cleanup EXIT

# ── Load environment ──────────────────────────────────────────────────────────
load_env() {
  (cd "${PROJECT_DIR}" && cargo xtask live --suite guard >/dev/null)
  export RUSTARR_HOME="${HOME}/.rustarr-shart"

  if [[ -f "${ENV_FILE}" ]]; then
    # shellcheck disable=SC1090
    set -a; source "${ENV_FILE}"; set +a
    log_info "Loaded shart credentials from ${ENV_FILE}"
  else
    log_warn "${ENV_FILE} not found — using environment variables"
  fi

  local host="${RUSTARR_MCP_HOST:-localhost}"
  # Remap bind address 0.0.0.0 → localhost for outbound connections
  [[ "${host}" == "0.0.0.0" ]] && host="localhost"
  local port="${RUSTARR_MCP_PORT:-40070}"
  MCP_URL="http://${host}:${port}/mcp"

  local token="${RUSTARR_MCP_TOKEN:-}"
  MCPORTER_HEADER_ARGS=()
  if [[ -n "${token}" ]]; then
    MCPORTER_HEADER_ARGS+=(--header "Authorization: Bearer ${token}")
  fi

  log_info "MCP URL: ${MCP_URL}"
  if [[ ${#MCPORTER_HEADER_ARGS[@]} -gt 0 ]]; then
    log_info "Auth: Bearer token configured"
  else
    log_info "Auth: none (RUSTARR_MCP_TOKEN unset — server must be in no-auth mode)"
  fi
}

# ── Prerequisites ─────────────────────────────────────────────────────────────
check_prerequisites() {
  local missing=false
  command -v mcporter &>/dev/null || { log_error "mcporter not found in PATH"; missing=true; }
  command -v python3  &>/dev/null || { log_error "python3 not found in PATH";  missing=true; }
  command -v curl     &>/dev/null || { log_error "curl not found in PATH";     missing=true; }
  command -v jq       &>/dev/null || { log_error "jq not found in PATH";       missing=true; }
  [[ "${missing}" == true ]] && return 2
  return 0
}

# ── Server connectivity smoke-test ────────────────────────────────────────────
smoke_test_server() {
  log_info "Smoke-testing server connectivity..."
  local base_url="${MCP_URL%/mcp}"

  # /health is always unauthenticated — safe to test without token
  local health_status
  health_status="$(
    curl -sf --max-time 10 "${base_url}/health" 2>/dev/null | \
    python3 -c "import sys,json; print(json.load(sys.stdin).get('status',''))" 2>/dev/null
  )" || health_status=''

  if [[ "${health_status}" != "ok" ]]; then
    log_error "Health endpoint at ${base_url}/health did not return status=ok"
    log_error "Is the rustarr-mcp server running?  just dev   or   just docker-up"
    log_error "Then retry:  ./tests/mcporter/test-mcp.sh"
    return 2
  fi
  log_info "Health endpoint OK (status=ok)"

  # Confirm the MCP layer responds to tools/list
  local tool_count
  tool_count="$(
    curl -sf --max-time 10 \
      -X POST "${MCP_URL}" \
      -H "Content-Type: application/json" \
      -H "Accept: application/json, text/event-stream" \
      ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
      -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' 2>/dev/null | \
    python3 -c "
import sys, json
d = json.load(sys.stdin)
print(len(d.get('result', {}).get('tools', [])))
" 2>/dev/null
  )" || tool_count=0

  if [[ "${tool_count}" -lt 1 ]] 2>/dev/null; then
    log_error "tools/list returned ${tool_count} tools — expected at least 1"
    return 2
  fi
  log_info "Server OK — ${tool_count} MCP tools available"
  return 0
}

# ── mcporter wrappers ────────────────────────────────────────────────────────
# Makes a single MCP tool call via mcporter and returns the JSON output.
mcporter_supports_headers() {
  mcporter call --help 2>/dev/null | grep -q -- '--header'
}

jsonrpc_tool_call() {
  local tool="${1:?tool required}"
  local args_json="${2:?args_json required}"
  local payload
  payload="$(python3 -c '
import json, sys
print(json.dumps({
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {"name": sys.argv[1], "arguments": json.loads(sys.argv[2])},
}))
' "${tool}" "${args_json}")"

  curl -sf --max-time "$(( (CALL_TIMEOUT_MS + 999) / 1000 ))" \
    -X POST "${MCP_URL}" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
    -d "${payload}" 2>>"${LOG_FILE}" \
    | python3 -c '
import json, sys
try:
    outer = json.load(sys.stdin)
    if "error" in outer:
        print(json.dumps({"error": outer["error"]}, indent=2))
        sys.exit(0)
    content = outer.get("result", {}).get("content", [])
    first = content[0] if content else {}
    if isinstance(first, dict) and isinstance(first.get("json"), dict):
        print(json.dumps(first["json"], indent=2))
        sys.exit(0)
    text = first.get("text", "") if isinstance(first, dict) else ""
    if text:
        parsed = json.loads(text)
        print(json.dumps(parsed, indent=2))
        sys.exit(0)
    print(json.dumps({"error": "empty tool result"}, indent=2))
except Exception as exc:
    print(json.dumps({"error": str(exc)}, indent=2))
'
}

mcporter_call() {
  local tool="${1:?tool required}"
  local args_json="${2:?args_json required}"

  if [[ ${#MCPORTER_HEADER_ARGS[@]} -gt 0 ]] && ! mcporter_supports_headers; then
    printf "${C_YELLOW}[WARN]${C_RESET}  mcporter call lacks --header support; falling back to JSON-RPC tools/call\n" \
      | tee -a "${LOG_FILE}" >&2
    jsonrpc_tool_call "${tool}" "${args_json}"
    return
  fi

  mcporter call \
    --http-url "${MCP_URL}" \
    --allow-http \
    ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
    --tool "${tool}" \
    --args "${args_json}" \
    --timeout "${CALL_TIMEOUT_MS}" \
    --output json \
    2>>"${LOG_FILE}"
}

# Reads an MCP resource. Newer mcporter builds can exercise resources directly;
# keep a JSON-RPC fallback so this harness remains compatible with older local
# versions while still preferring mcporter when the command is available.
mcporter_read_resource() {
  local resource_uri="${1:?resource URI required}"

  if mcporter resource read --help >/dev/null 2>&1; then
    mcporter resource read \
      --http-url "${MCP_URL}" \
      --allow-http \
      ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
      --uri "${resource_uri}" \
      --timeout "${CALL_TIMEOUT_MS}" \
      --output json \
      2>>"${LOG_FILE}"
    return
  fi

  if mcporter resources read --help >/dev/null 2>&1; then
    mcporter resources read \
      --http-url "${MCP_URL}" \
      --allow-http \
      ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
      --uri "${resource_uri}" \
      --timeout "${CALL_TIMEOUT_MS}" \
      --output json \
      2>>"${LOG_FILE}"
    return
  fi

  printf "${C_YELLOW}[WARN]${C_RESET}  mcporter resource command unavailable; falling back to JSON-RPC resources/read\n" \
    | tee -a "${LOG_FILE}" >&2
  curl -sf --max-time 15 \
    -X POST "${MCP_URL}" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    ${MCPORTER_HEADER_ARGS[@]+"${MCPORTER_HEADER_ARGS[@]}"} \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"resources/read\",\"params\":{\"uri\":\"${resource_uri}\"}}" \
    2>/dev/null
}

# ── run_test: basic structural test ──────────────────────────────────────────
# Verifies the call returns valid JSON with an optional top-level key.
# Use run_test_semantic (below) when you can validate actual values.
run_test() {
  local label="${1:?}" tool="${2:?}" args="${3:?}" expected_key="${4:-}"
  local t0 output elapsed_ms json_check

  t0="$(date +%s%N)"
  output="$(mcporter_call "${tool}" "${args}")" || true
  elapsed_ms="$(( ( $(date +%s%N) - t0 ) / 1000000 ))"

  [[ "${VERBOSE}" == true ]] && printf '%s\n' "${output}" | tee -a "${LOG_FILE}" \
    || printf '%s\n' "${output}" >> "${LOG_FILE}"

  # Validate JSON is parseable and not an error payload
  json_check="$(
    printf '%s' "${output}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    if isinstance(d, dict) and ('error' in d or d.get('kind') == 'error'):
        print('error: ' + str(d.get('error', d.get('message', 'unknown'))))
    else:
        print('ok')
except Exception as e:
    print('invalid_json: ' + str(e))
" 2>/dev/null
  )" || json_check="parse_error"

  if [[ "${json_check}" != "ok" ]]; then
    _fail "${label}" "${elapsed_ms}" "JSON check failed: ${json_check}"
    return 1
  fi

  # Validate optional key presence
  if [[ -n "${expected_key}" ]]; then
    local key_check
    key_check="$(
      printf '%s' "${output}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    keys = '${expected_key}'.split('.')
    node = d
    for k in keys:
        node = node[int(k)] if (isinstance(node, list) and k.isdigit()) else node[k]
    print('ok')
except Exception as e:
    print('missing: ' + str(e))
" 2>/dev/null
    )" || key_check="parse_error"

    if [[ "${key_check}" != "ok" ]]; then
      _fail "${label}" "${elapsed_ms}" "key '.${expected_key}' missing: ${key_check}"
      return 1
    fi
  fi

  _pass "${label}" "${elapsed_ms}"
  return 0
}

# ── run_test_semantic: value-level test ───────────────────────────────────────
# This is the stronger test form — it validates that a specific field contains
# an expected value, not just that the field exists.
#
# TEMPLATE: Use this for all checks where you can predict the response value.
#           "does the response key exist?" is weak.
#           "does the response value equal/contain X?" proves correctness.
#
# Usage:
#   run_test_semantic "label" "tool" '{"action":"..."}' \
#     "response.key.path" "expected_value" "exact|contains"
#
# Modes:
#   exact    — field value must equal expected_value exactly
#   contains — field value (as string) must contain expected_value as a substring
run_test_semantic() {
  local label="${1:?}" tool="${2:?}" args="${3:?}"
  local key_path="${4:?}" expected="${5:?}" mode="${6:-contains}"
  local t0 output elapsed_ms

  t0="$(date +%s%N)"
  output="$(mcporter_call "${tool}" "${args}")" || true
  elapsed_ms="$(( ( $(date +%s%N) - t0 ) / 1000000 ))"

  [[ "${VERBOSE}" == true ]] && printf '%s\n' "${output}" | tee -a "${LOG_FILE}" \
    || printf '%s\n' "${output}" >> "${LOG_FILE}"

  local check_result
  check_result="$(
    printf '%s' "${output}" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    if isinstance(d, dict) and ('error' in d or d.get('kind') == 'error'):
        print('error: ' + str(d.get('error', d.get('message', 'unknown'))))
        sys.exit(0)
    # Navigate to the key
    keys = '${key_path}'.split('.')
    node = d
    for k in keys:
        if k:
            node = node[int(k)] if (isinstance(node, list) and k.isdigit()) else node[k]
    value = str(node) if not isinstance(node, str) else node
    expected = '${expected}'
    mode = '${mode}'
    if mode == 'exact':
        if value == expected:
            print('ok')
        else:
            print('mismatch: expected exactly \"' + expected + '\", got \"' + value + '\"')
    else:  # contains
        if expected in value:
            print('ok')
        else:
            print('mismatch: expected \"' + expected + '\" in \"' + value + '\"')
except KeyError as e:
    print('missing key: ' + str(e))
except Exception as e:
    print('error: ' + str(e))
" 2>/dev/null
  )" || check_result="parse_error"

  if [[ "${check_result}" != "ok" ]]; then
    _fail "${label}" "${elapsed_ms}" "${check_result}"
    return 1
  fi

  _pass "${label}" "${elapsed_ms}"
  return 0
}

# ── skip_test ─────────────────────────────────────────────────────────────────
skip_test() {
  local label="${1:?}" reason="${2:-prerequisite not met}"
  printf "${C_YELLOW}[SKIP]${C_RESET} %-60s %s\n" "${label}" "${reason}" | tee -a "${LOG_FILE}"
  SKIP_COUNT=$(( SKIP_COUNT + 1 ))
}

# ── Internal pass/fail helpers ────────────────────────────────────────────────
_pass() {
  local label="${1:?}" elapsed_ms="${2:-0}"
  printf "${C_GREEN}[PASS]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
    "${label}" "${elapsed_ms}" | tee -a "${LOG_FILE}"
  PASS_COUNT=$(( PASS_COUNT + 1 ))
}

_fail() {
  local label="${1:?}" elapsed_ms="${2:-0}" reason="${3:-unknown}"
  printf "${C_RED}[FAIL]${C_RESET} %-60s ${C_DIM}%dms${C_RESET}\n" \
    "${label}" "${elapsed_ms}" | tee -a "${LOG_FILE}"
  printf '       %s\n' "${reason}" | tee -a "${LOG_FILE}"
  FAIL_COUNT=$(( FAIL_COUNT + 1 ))
  FAIL_NAMES+=("${label}")
}

# =============================================================================
# TEST SUITES
# =============================================================================

# ── suite_auth ────────────────────────────────────────────────────────────────
suite_auth() {
  printf '\n%b== auth enforcement ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  if [[ -z "${RUSTARR_MCP_TOKEN:-}" ]]; then
    skip_test "auth: unauthenticated /mcp returns 401" "RUSTARR_MCP_TOKEN unset"
    skip_test "auth: bad token returns 401"             "RUSTARR_MCP_TOKEN unset"
    return
  fi

  local label status

  label="auth: unauthenticated /mcp returns 401"
  status="$(curl -s --max-time 10 -o /dev/null -w "%{http_code}" \
    "${MCP_URL}" -X POST -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' 2>/dev/null)" || status=0
  if [[ "${status}" == "401" ]]; then _pass "${label}" 0
  else _fail "${label}" 0 "expected HTTP 401, got ${status}"; fi

  label="auth: bad token returns 401"
  status="$(curl -s --max-time 10 -o /dev/null -w "%{http_code}" \
    "${MCP_URL}" -X POST \
    -H "Authorization: Bearer intentionally-bad-token" \
    -H "Content-Type: application/json" \
    -H "Accept: application/json, text/event-stream" \
    -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' 2>/dev/null)" || status=0
  if [[ "${status}" == "401" ]]; then _pass "${label}" 0
  else _fail "${label}" 0 "expected HTTP 401, got ${status}"; fi
}

# ── suite_core ────────────────────────────────────────────────────────────────
# TEMPLATE: This is the main test suite. Each test here is a semantic check —
#           it verifies that the response contains the RIGHT data, not just JSON.
#
# Key pattern: run_test_semantic validates a specific field value.
# Key principle: test the contract, not the implementation.
suite_core() {
  printf '\n%b== service tools — core actions ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # ── integrations ────────────────────────────────────────────────────────────
  run_test "sonarr integrations: returns supported inventory" \
    "sonarr" '{"action":"integrations"}' "supported"

  run_test_semantic "sonarr integrations: includes sonarr" \
    "sonarr" '{"action":"integrations"}' \
    "supported" "sonarr" "contains"

  # ── service_status / api_get ────────────────────────────────────────────────
  if [[ -n "${RUSTARR_SONARR_URL:-}" && -n "${RUSTARR_SONARR_API_KEY:-}" ]]; then
    run_test "sonarr service_status: sonarr status returns appName" \
      "sonarr" '{"action":"service_status"}' "appName"

    run_test "sonarr api_get: sonarr status returns version" \
      "sonarr" '{"action":"api_get","path":"/api/v3/system/status"}' "version"
  else
    skip_test "rustarr service_status: sonarr status returns appName" "RUSTARR_SONARR_URL/API_KEY unset"
    skip_test "rustarr api_get: sonarr status returns version" "RUSTARR_SONARR_URL/API_KEY unset"
  fi

  # ── help ────────────────────────────────────────────────────────────────────
  run_test "sonarr help: returns action list" \
    "sonarr" '{"action":"help"}' "actions"

  run_test_semantic "sonarr help: mentions api_get action" \
    "sonarr" '{"action":"help"}' \
    "actions" "api_get" "contains"
}

# ── suite_schema_resource ──────────────────────────────────────────────────────
# TEMPLATE: The schema resource is exposed by the rustarr as:
#           rustarr://schema/mcp-tool
#           Replace "rustarr" with your service name in the URI.
suite_schema_resource() {
  printf '\n%b== schema resource ==%b\n' "${C_BOLD}" "${C_RESET}" | tee -a "${LOG_FILE}"

  # Fetch the schema resource via mcporter when supported. The wrapper falls
  # back to raw JSON-RPC for older mcporter versions.
  local resource_uri="rustarr://schema/mcp-tool"
  local output t0 elapsed_ms

  t0="$(date +%s%N)"
  output="$(mcporter_read_resource "${resource_uri}")" || output=''
  elapsed_ms="$(( ( $(date +%s%N) - t0 ) / 1000000 ))"

  printf '%s\n' "${output}" >> "${LOG_FILE}"
  [[ "${VERBOSE}" == true ]] && printf '%s\n' "${output}"

  if [[ -z "${output}" ]]; then
    _fail "schema resource: fetch ${resource_uri}" "${elapsed_ms}" "empty response from resources/read"
    return 1
  fi

  # Parse and validate the schema content
  local schema_check
  schema_check="$(
    printf '%s' "${output}" | python3 -c "
import sys, json
try:
    outer = json.load(sys.stdin)
    # mcporter resource commands may return the resource JSON directly, while
    # JSON-RPC resources/read returns result.contents[0].text.
    if isinstance(outer, dict) and 'name' in outer and 'inputSchema' in outer:
        schema = outer
    else:
        contents = outer.get('result', {}).get('contents', []) if isinstance(outer, dict) else []
        first = contents[0] if contents else {}
        if isinstance(first, dict) and isinstance(first.get('json'), dict):
            schema = first['json']
        else:
            text = first.get('text', '') if isinstance(first, dict) else ''
            if not text:
                print('error: empty text in resource content')
                sys.exit(0)
            schema = json.loads(text)

    if isinstance(schema, list):
        schema = schema[0] if schema else {}

    name = schema.get('name', '')
    if name != 'sonarr':
        print('name mismatch: expected first service tool \"sonarr\", got \"' + name + '\"')
        sys.exit(0)

    if 'inputSchema' not in schema:
        print('missing inputSchema in tool schema')
        sys.exit(0)

    input_schema = schema['inputSchema']
    if input_schema.get('type') != 'object':
        print('inputSchema.type should be object, got: ' + str(input_schema.get('type')))
        sys.exit(0)

    props = input_schema.get('properties', {})
    if 'action' not in props:
        print('inputSchema.properties should include \"action\"')
        sys.exit(0)

    print('ok')
except Exception as e:
    print('error: ' + str(e))
" 2>/dev/null
  )" || schema_check="parse_error"

  if [[ "${schema_check}" == "ok" ]]; then
    _pass "schema resource: valid service JSON schema with inputSchema" "${elapsed_ms}"
  else
    _fail "schema resource: valid service JSON schema with inputSchema" \
      "${elapsed_ms}" "${schema_check}"
  fi
}

# =============================================================================
# ORCHESTRATION
# =============================================================================

print_summary() {
  local total_ms total
  total_ms="$(( ( $(date +%s%N) - TS_START ) / 1000000 ))"
  total=$(( PASS_COUNT + FAIL_COUNT + SKIP_COUNT ))

  printf '\n%b%s%b\n' "${C_BOLD}" "$(printf '=%.0s' {1..65})" "${C_RESET}"
  printf '%b%-20s%b  %b%d%b\n' "${C_BOLD}" "PASS"    "${C_RESET}" "${C_GREEN}"  "${PASS_COUNT}" "${C_RESET}"
  printf '%b%-20s%b  %b%d%b\n' "${C_BOLD}" "FAIL"    "${C_RESET}" "${C_RED}"    "${FAIL_COUNT}" "${C_RESET}"
  printf '%b%-20s%b  %b%d%b\n' "${C_BOLD}" "SKIP"    "${C_RESET}" "${C_YELLOW}" "${SKIP_COUNT}" "${C_RESET}"
  printf '%b%-20s%b  %d\n'     "${C_BOLD}" "TOTAL"   "${C_RESET}" "${total}"
  printf '%b%-20s%b  %ds (%dms)\n' "${C_BOLD}" "ELAPSED" "${C_RESET}" \
    "$(( total_ms / 1000 ))" "${total_ms}"
  printf '%b%s%b\n' "${C_BOLD}" "$(printf '=%.0s' {1..65})" "${C_RESET}"

  if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    printf '\n%bFailed tests:%b\n' "${C_RED}" "${C_RESET}"
    local name
    for name in "${FAIL_NAMES[@]}"; do printf '  * %s\n' "${name}"; done
    printf '\nFull log: %s\n' "${LOG_FILE}"
  fi
}

run_sequential() {
  suite_auth
  suite_core
  suite_schema_resource
}

run_parallel() {
  log_warn "--parallel: running suites concurrently"
  local tmp_dir pids=() suite
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf -- "${tmp_dir}"' RETURN

  for suite in suite_auth suite_core suite_schema_resource; do
    (
      PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAIL_NAMES=()
      "${suite}"
      printf '%d %d %d\n' "${PASS_COUNT}" "${FAIL_COUNT}" "${SKIP_COUNT}" \
        > "${tmp_dir}/${suite}.counts"
      printf '%s\n' "${FAIL_NAMES[@]:-}" > "${tmp_dir}/${suite}.fails"
    ) &
    pids+=($!)
  done

  local pid; for pid in "${pids[@]}"; do wait "${pid}" || true; done

  local f p fl s
  for f in "${tmp_dir}"/*.counts; do
    [[ -f "${f}" ]] || continue
    read -r p fl s < "${f}"
    PASS_COUNT=$(( PASS_COUNT + p ))
    FAIL_COUNT=$(( FAIL_COUNT + fl ))
    SKIP_COUNT=$(( SKIP_COUNT + s ))
  done
  for f in "${tmp_dir}"/*.fails; do
    [[ -f "${f}" ]] || continue
    while IFS= read -r line; do [[ -n "${line}" ]] && FAIL_NAMES+=("${line}"); done < "${f}"
  done
}

main() {
  parse_args "$@"
  load_env

  printf '%b%s%b\n' "${C_BOLD}" "$(printf '=%.0s' {1..65})" "${C_RESET}"
  printf '%b  rustarr-mcp integration smoke-test%b\n' "${C_BOLD}" "${C_RESET}"
  printf '%b  Project:  %s%b\n' "${C_BOLD}" "${PROJECT_DIR}" "${C_RESET}"
  printf '%b  MCP URL:  %s%b\n' "${C_BOLD}" "${MCP_URL}" "${C_RESET}"
  printf '%b  Timeout:  %dms/call | Parallel: %s%b\n' \
    "${C_BOLD}" "${CALL_TIMEOUT_MS}" "${USE_PARALLEL}" "${C_RESET}"
  printf '%b  Log:      %s%b\n' "${C_BOLD}" "${LOG_FILE}" "${C_RESET}"
  printf '%b%s%b\n\n' "${C_BOLD}" "$(printf '=%.0s' {1..65})" "${C_RESET}"

  check_prerequisites || exit 2

  smoke_test_server || {
    log_error ""
    log_error "Server connectivity check failed. Aborting."
    log_error ""
    log_error "To diagnose:"
    log_error "  just dev                            # start in no-auth dev mode"
    log_error "  curl http://localhost:40070/health   # check health endpoint"
    log_error "  docker ps | grep rustarr-mcp        # check Docker container"
    exit 2
  }

  if [[ "${USE_PARALLEL}" == true ]]; then
    run_parallel
  else
    run_sequential
  fi

  print_summary
  [[ "${FAIL_COUNT}" -gt 0 ]] && exit 1 || exit 0
}

main "$@"
