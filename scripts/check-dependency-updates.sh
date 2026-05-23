#!/usr/bin/env bash
# Report lockfile-compatible and latest direct dependency updates.
#
# This script is intentionally read-only unless Cargo itself changes behavior:
# `cargo update --dry-run` does not modify Cargo.lock, and the latest-version
# scan only calls `cargo search`.
set -Eeuo pipefail

IFS=$'\n\t'

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"

SKIP_SEARCH=false
FAIL_ON_UPDATES=false

usage() {
  cat <<'EOF'
Usage: scripts/check-dependency-updates.sh [OPTIONS]

Check dependency updates for this Rust workspace.

Reports:
  1. lockfile-compatible updates using `cargo update --dry-run`
  2. latest crates.io versions for direct root dependencies

Options:
  --skip-search       Skip crates.io direct dependency latest-version checks.
  --fail-on-updates   Exit 1 when possible updates are detected.
  -h, --help          Show this help.

Notes:
  - The script does not modify Cargo.lock.
  - Latest-version checks require network access.
  - Git/path dependencies are skipped in the latest-version table.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-search)
      SKIP_SEARCH=true
      shift
      ;;
    --fail-on-updates)
      FAIL_ON_UPDATES=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_cmd() {
  local -r cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || {
    echo "ERROR: required command not found: $cmd" >&2
    exit 1
  }
}

trim() {
  local value="$*"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

extract_direct_deps() {
  awk '
    /^\[(workspace\.)?(dependencies|dev-dependencies|build-dependencies)\]$/ {
      section = $0
      next
    }
    /^\[/ {
      section = ""
      next
    }
    section != "" && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      print
    }
  ' "$ROOT_DIR/Cargo.toml" | sort -u
}

dependency_version_req() {
  local -r line="$1"
  local rhs

  rhs="${line#*=}"
  rhs="$(trim "$rhs")"

  if [[ "$rhs" =~ ^\"([^\"]+)\" ]]; then
    printf '%s' "${BASH_REMATCH[1]}"
    return 0
  fi

  if [[ "$rhs" =~ version[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
    printf '%s' "${BASH_REMATCH[1]}"
    return 0
  fi

  return 1
}

latest_crate_version() {
  local -r crate="$1"
  local output

  output="$(cargo search "$crate" --limit 1 2>/dev/null || true)"
  awk -v crate="$crate" '
    $1 == crate && $2 == "=" {
      version = $3
      gsub(/"/, "", version)
      print version
      exit
    }
  ' <<<"$output"
}

direct_dependency_status() {
  local -r requirement="$1"
  local -r latest="$2"

  if [[ "$requirement" == "$latest" ]]; then
    printf 'ok'
  elif [[ "$requirement" =~ ^[0-9]+$ && "$latest" == "$requirement".* ]]; then
    printf 'compatible-range'
  elif [[ "$requirement" =~ ^[0-9]+\.[0-9]+$ && "$latest" == "$requirement".* ]]; then
    printf 'compatible-range'
  else
    printf 'review'
  fi
}

print_header() {
  printf '\n== %s ==\n' "$*"
}

require_cmd cargo

cd "$ROOT_DIR"
export CARGO_TERM_COLOR=never

updates_found=false

print_header "Lockfile-compatible updates"
dry_run_output="$(cargo update --dry-run 2>&1)"
printf '%s\n' "$dry_run_output"

if grep -Eq '^[[:space:]]+(Adding|Removing|Downgrading)[[:space:]]' <<<"$dry_run_output" \
  || grep -Eq '^[[:space:]]+Updating[[:space:]]+[^[:space:]]+[[:space:]]+v' <<<"$dry_run_output" \
  || grep -Eq '^[[:space:]]+Locking[[:space:]]+[1-9][0-9]*[[:space:]]+packages' <<<"$dry_run_output"; then
  updates_found=true
fi

if [[ "$SKIP_SEARCH" == false ]]; then
  print_header "Direct dependency latest versions"
  printf '%-32s %-18s %-18s %s\n' "crate" "requirement" "latest" "status"

  while IFS= read -r line; do
    dep_name="$(trim "${line%%=*}")"
    version_req="$(dependency_version_req "$line" || true)"

    if [[ -z "$version_req" ]]; then
      printf '%-32s %-18s %-18s %s\n' "$dep_name" "-" "-" "skipped"
      continue
    fi

    latest="$(latest_crate_version "$dep_name")"
    if [[ -z "$latest" ]]; then
      printf '%-32s %-18s %-18s %s\n' "$dep_name" "$version_req" "unknown" "check failed"
      continue
    fi

    status="$(direct_dependency_status "$version_req" "$latest")"
    if [[ "$status" == "review" ]]; then
      updates_found=true
    fi

    printf '%-32s %-18s %-18s %s\n' "$dep_name" "$version_req" "$latest" "$status"
  done < <(extract_direct_deps)
fi

print_header "Result"
if [[ "$updates_found" == true ]]; then
  echo "Dependency updates may be available."
  if [[ "$FAIL_ON_UPDATES" == true ]]; then
    exit 1
  fi
else
  echo "No dependency updates detected."
fi
