#!/usr/bin/env bash
# refresh-docs.sh — Refresh reference docs for rustarr
#
# TEMPLATE: This script fetches the MCP protocol docs and Rust SDK so every
# server built from this template has up-to-date reference material for the
# underlying transport and protocol. When you adapt this template, ADD your
# service's own docs/repos below the MCP section.
#
# Pattern: §38 in docs/PATTERNS.md
# Adapted from: agentcast/scripts/refresh-docs.sh
#
# Usage:
#   scripts/refresh-docs.sh [--dry-run] [--skip-crawl] [--skip-repomix]
#
# What it fetches:
#   CRAWL:   https://modelcontextprotocol.io  — Full MCP protocol docs
#            https://docs.rs/rmcp             — rmcp crate docs (if axon supports)
#   REPOMIX: modelcontextprotocol/rust-sdk    — rmcp crate source (primary reference)
#            modelcontextprotocol/modelcontextprotocol — MCP spec
#            modelcontextprotocol/registry    — server.json schema + registry spec
#            openclaw/mcporter               — mcporter testing tool source
#
# TEMPLATE: Add your service's crawls and repomix packs in the marked section below.
#
# Exit codes:  0 success  |  1 error  |  2 bad args
set -Eeuo pipefail
IFS=$'\n\t'

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
REF_DIR="$ROOT_DIR/docs/references"
CHANGES_FILE="$REF_DIR/CHANGES.md"
AXON_OUTPUT_DIR="${AXON_OUTPUT_DIR:-$HOME/.axon/output}"

DRY_RUN=false
SKIP_CRAWL=false
SKIP_REPOMIX=false

usage() {
  cat <<'EOF'
Usage: scripts/refresh-docs.sh [OPTIONS]

Fetch and refresh local reference documentation.

  Crawled sites:
    https://modelcontextprotocol.io  — MCP protocol specification and guides
    https://code.claude.com          — Claude Code documentation

  Repomix packs:
    modelcontextprotocol/rust-sdk    — rmcp Rust SDK (primary reference for this template)
    modelcontextprotocol/modelcontextprotocol — MCP spec source
    modelcontextprotocol/registry    — server.json schema, MCP registry spec
    openclaw/mcporter                — mcporter integration test tool

  TEMPLATE: add your service's crawls and repos above/below the MCP section.

Options:
  --dry-run        Print plan without writing.
  --skip-crawl     Skip Axon crawls; update Repomix packs only.
  --skip-repomix   Skip Repomix packs; run Axon crawls only.
  -h, --help       Show this help.

Environment:
  AXON_OUTPUT_DIR   Axon host output dir (default: ~/.axon/output)
  REPOMIX_BIN       Repomix binary path (default: repomix or npx --yes repomix)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)      DRY_RUN=true;      shift ;;
    --skip-crawl)   SKIP_CRAWL=true;   shift ;;
    --skip-repomix) SKIP_REPOMIX=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "ERROR: unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ "$SKIP_CRAWL" == true && "$SKIP_REPOMIX" == true ]] && {
  echo "ERROR: --skip-crawl and --skip-repomix cannot both be set" >&2; exit 2
}

log() { printf '[refresh-docs] %s\n' "$*"; }

refresh_scope() {
  if   [[ "$SKIP_CRAWL"   == true ]]; then printf 'repomix-only'
  elif [[ "$SKIP_REPOMIX" == true ]]; then printf 'crawl-only'
  else printf 'full'; fi
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "ERROR: required command not found: $1" >&2; exit 1; }
}

make_tmpdir() { mktemp -d "${TMPDIR:-/tmp}/rustarr-refresh-docs.XXXXXX"; }

atomic_replace_dir() {
  local src="$1" dst="$2" parent backup
  parent="$(dirname -- "$dst")"; mkdir -p "$parent"
  backup="$(mktemp -d "$parent/.$(basename "$dst").backup.XXXXXX")"; rmdir "$backup"
  [[ -e "$dst" ]] && mv -- "$dst" "$backup"
  if mv -- "$src" "$dst"; then rm -rf -- "$backup"
  else [[ -e "$backup" ]] && mv -- "$backup" "$dst"; return 1; fi
}

copy_job_output_to_layout() {
  local source_dir="$1" target_dir="$2" tmp_target
  [[ -f "$source_dir/manifest.jsonl" ]] || { echo "ERROR: missing Axon manifest: $source_dir/manifest.jsonl" >&2; return 1; }
  [[ -d "$source_dir/markdown" ]]       || { echo "ERROR: missing Axon markdown dir: $source_dir/markdown" >&2; return 1; }
  tmp_target="$(make_tmpdir)"
  cp -a "$source_dir/." "$tmp_target/"
  atomic_replace_dir "$tmp_target" "$target_dir"
}

newest_domain_run() {
  local domain_dir="$AXON_OUTPUT_DIR/domains/$1"
  [[ -d "$domain_dir" ]] || return 1
  find "$domain_dir" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' \
    | sort -nr | awk 'NR==1{$1=""; sub(/^ /, ""); print}'
}

crawl_docs() {
  local url="$1" domain="$2" target_rel="$3"
  local target_dir="$REF_DIR/$target_rel" output job_id source_dir
  log "crawl $url -> docs/references/$target_rel"
  [[ "$DRY_RUN" == true ]] && return 0
  require_cmd axon
  output="$(axon crawl "$url" --wait true --yes 2>&1)"; printf '%s\n' "$output"
  job_id="$(awk '/^Job ID:/{print $3}' <<<"$output" | tail -1)"
  if [[ -n "$job_id" && -d "$AXON_OUTPUT_DIR/domains/$domain/$job_id" ]]; then
    source_dir="$AXON_OUTPUT_DIR/domains/$domain/$job_id"
  else
    source_dir="$(newest_domain_run "$domain")"
  fi
  [[ -n "$source_dir" && -d "$source_dir" ]] || {
    echo "ERROR: could not locate Axon output for $domain" >&2; return 1
  }
  copy_job_output_to_layout "$source_dir" "$target_dir"
}

repomix_command() {
  if   [[ -n "${REPOMIX_BIN:-}" ]]; then "$REPOMIX_BIN" "$@"
  elif command -v repomix >/dev/null 2>&1; then repomix "$@"
  else require_cmd npx; npx --yes repomix "$@"; fi
}

pack_repo() {
  local remote="$1" target_rel="$2" include_patterns="${3:-}" ignore_patterns="${4:-}"
  local target_file="$REF_DIR/$target_rel" tmp_dir tmp_file
  log "pack $remote -> docs/references/$target_rel"
  [[ -n "$include_patterns" ]] && log "  include: $include_patterns"
  [[ -n "$ignore_patterns"  ]] && log "  ignore:  $ignore_patterns"
  [[ "$DRY_RUN" == true ]] && return 0
  tmp_dir="$(make_tmpdir)"; tmp_file="$tmp_dir/repomix-output.xml"
  local args=(--remote "$remote" --style xml --output "$tmp_file" --top-files-len 10)
  [[ -n "$include_patterns" ]] && args+=(--include "$include_patterns")
  [[ -n "$ignore_patterns"  ]] && args+=(--ignore  "$ignore_patterns")
  repomix_command "${args[@]}"
  [[ -s "$tmp_file" ]] || { echo "ERROR: Repomix produced no output for $remote" >&2; rm -rf -- "$tmp_dir"; return 1; }
  mkdir -p "$(dirname -- "$target_file")"
  mv -- "$tmp_file" "$target_file"
  rm -rf -- "$tmp_dir"
}

write_index() {
  local mcp_docs=0 claude_docs=0 mcporter_docs=0
  [[ -d "$REF_DIR/mcp/docs"      ]] && mcp_docs="$(find "$REF_DIR/mcp/docs"      -type f | wc -l | tr -d ' ')"
  [[ -d "$REF_DIR/claude-code"   ]] && claude_docs="$(find "$REF_DIR/claude-code" -type f | wc -l | tr -d ' ')"
  [[ -d "$REF_DIR/mcporter/docs" ]] && mcporter_docs="$(find "$REF_DIR/mcporter/docs" -type f | wc -l | tr -d ' ')"

  cat > "$REF_DIR/INDEX.md" <<EOF
# Reference Index — rustarr

TEMPLATE: When you adapt this template, update this index to reflect your service's
reference material.

| Path | Contents | Source |
| --- | --- | --- |
| \`mcp/docs/\`        | MCP protocol docs (crawled)    | modelcontextprotocol.io |
| \`mcp/repos/\`       | MCP Rust SDK + spec (repomix)  | modelcontextprotocol/* |
| \`claude-code/\`     | Claude Code docs (crawled)     | code.claude.com |
| \`mcporter/docs/\`   | mcporter docs (sparse clone)   | openclaw/mcporter/docs |
| \`mcporter/repos/\`  | mcporter source (repomix)      | openclaw/mcporter |

## Crawled Doc File Counts

| Path | Files |
| --- | ---: |
| \`mcp/docs/\`      | $mcp_docs |
| \`claude-code/\`   | $claude_docs |
| \`mcporter/docs/\` | $mcporter_docs |

## Key References for MCP Server Development

- **rmcp crate**: \`mcp/repos/modelcontextprotocol-rust-sdk.xml\`
  The primary reference for implementing ServerHandler, tool dispatch, resource handling, and prompts.

- **MCP spec**: \`mcp/repos/modelcontextprotocol-modelcontextprotocol.xml\`
  Protocol specification — useful when the SDK doesn't expose something you need.

- **server.json schema**: \`mcp/repos/modelcontextprotocol-registry.xml\`
  JSON schema for MCP registry publishing (\`server.json\`).

- **mcporter**: \`mcporter/repos/openclaw-mcporter.xml\`
  Integration testing tool used by \`tests/mcporter/test-mcp.sh\`.

_Updated: $(date -u +%Y-%m-%dT%H:%M:%SZ)_
EOF
}

sparse_clone_path() {
  local remote="$1" sparse_path="$2" target_rel="$3" mode="${4:-recursive}"
  local target_dir="$REF_DIR/$target_rel" tmp_dir clone_dir tmp_target
  log "sparse clone $remote/$sparse_path -> docs/references/$target_rel"
  [[ "$DRY_RUN" == true ]] && return 0
  require_cmd git
  tmp_dir="$(make_tmpdir)"; clone_dir="$tmp_dir/repo"; tmp_target="$tmp_dir/output"
  git clone --filter=blob:none --sparse --depth=1 "$remote" "$clone_dir" >/dev/null
  git -C "$clone_dir" sparse-checkout set "$sparse_path" >/dev/null
  mkdir -p "$tmp_target"
  case "$mode" in
    flat-mdx) find "$clone_dir/$sparse_path" -maxdepth 1 -type f -name '*.mdx' -exec cp -a {} "$tmp_target/" \; ;;
    recursive) cp -a "$clone_dir/$sparse_path/." "$tmp_target/" ;;
    *) echo "ERROR: unknown mode: $mode" >&2; rm -rf -- "$tmp_dir"; return 1 ;;
  esac
  atomic_replace_dir "$tmp_target" "$target_dir"
  rm -rf -- "$tmp_dir"
}

snapshot_references() {
  local output_file="$1"
  [[ ! -d "$REF_DIR" ]] && { : > "$output_file"; return 0; }
  (cd "$REF_DIR"; find . -type f ! -path './CHANGES.md' -print0 \
    | sort -z | xargs -0 -r sha256sum | sed 's#  \./#  #') > "$output_file"
}

snapshot_paths() { awk '{$1=""; sub(/^  /, ""); print}' "$1"; }

ensure_changes_file() {
  mkdir -p "$REF_DIR"
  [[ -f "$CHANGES_FILE" ]] && return 0
  cat > "$CHANGES_FILE" <<EOF
---
title: Reference Refresh Change Log — rustarr
generated_by: scripts/refresh-docs.sh
created_at: $(date -u +%Y-%m-%dT%H:%M:%SZ)
---

# Reference Refresh Change Log

Each entry records file-level changes after a real refresh run.
EOF
}

append_changes_log() {
  ensure_changes_file
  {
    printf '\n## %s\n\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf -- '- scope: `%s`\n' "$(refresh_scope)"
    printf -- '- summary: `%s added, %s modified, %s removed`\n' "$4" "$5" "$6"
  } >> "$CHANGES_FILE"
}

summarize_reference_changes() {
  local before_snapshot="$1" after_snapshot="$2" tmp_dir
  tmp_dir="$(make_tmpdir)"
  local bp="$tmp_dir/b.paths" ap="$tmp_dir/a.paths"
  local added="$tmp_dir/added" removed="$tmp_dir/removed" common="$tmp_dir/common" modified="$tmp_dir/modified"
  snapshot_paths "$before_snapshot" | sort > "$bp"
  snapshot_paths "$after_snapshot"  | sort > "$ap"
  comm -13 "$bp" "$ap" > "$added"; comm -23 "$bp" "$ap" > "$removed"; comm -12 "$bp" "$ap" > "$common"
  : > "$modified"
  while IFS= read -r p; do
    [[ "$(grep -F "  $p" "$before_snapshot" | cut -d' ' -f1)" != "$(grep -F "  $p" "$after_snapshot" | cut -d' ' -f1)" ]] \
      && printf '%s\n' "$p" >> "$modified"
  done < "$common"
  local ac rc mc
  ac="$(wc -l < "$added"   | tr -d ' ')"
  rc="$(wc -l < "$removed" | tr -d ' ')"
  mc="$(wc -l < "$modified"| tr -d ' ')"
  log "change summary: $ac added, $mc modified, $rc removed"
  append_changes_log "$added" "$modified" "$removed" "$ac" "$mc" "$rc"
  rm -rf -- "$tmp_dir"
}

main() {
  local snapshot_dir before_snapshot after_snapshot
  if [[ "$DRY_RUN" != true ]]; then
    snapshot_dir="$(make_tmpdir)"
    before_snapshot="$snapshot_dir/before.sha256"
    after_snapshot="$snapshot_dir/after.sha256"
    snapshot_references "$before_snapshot"
  fi

  mkdir -p \
    "$REF_DIR/mcp/docs"      \
    "$REF_DIR/mcp/repos"     \
    "$REF_DIR/claude-code"   \
    "$REF_DIR/mcporter/docs" \
    "$REF_DIR/mcporter/repos"

  # ── Crawled docs ──────────────────────────────────────────────────────────
  if [[ "$SKIP_CRAWL" != true ]]; then
    required_crawl_failed=0
    # MCP protocol documentation — essential for any MCP server development
    crawl_docs "https://modelcontextprotocol.io"  "modelcontextprotocol.io" "mcp/docs" \
      || { log "ERROR: mcp docs crawl failed"; required_crawl_failed=1; }
    # Claude Code documentation — for plugin/skill/hook development
    crawl_docs "https://code.claude.com/"         "code.claude.com"         "claude-code" \
      || { log "ERROR: claude-code docs crawl failed"; required_crawl_failed=1; }
    if [[ "$required_crawl_failed" -ne 0 ]]; then
      log "ERROR: one or more required crawls failed — reference docs may be stale"
      exit 1
    fi

    # TEMPLATE: Add your service's documentation site here:
    # crawl_docs "https://your-service.com/docs"  "your-service.com"  "your-service/docs"
  fi

  # ── Repomix packs ─────────────────────────────────────────────────────────
  if [[ "$SKIP_REPOMIX" != true ]]; then
    # MCP Rust SDK — THE primary reference for rmcp crate usage
    # Read this to understand ServerHandler, tool dispatch, elicitation, resources, prompts
    pack_repo "modelcontextprotocol/rust-sdk" \
      "mcp/repos/modelcontextprotocol-rust-sdk.xml"

    # MCP specification source — protocol-level reference.
    # Exclude large diagrams; SVG/Excalidraw assets dominate token count while
    # adding little value to text-first reference packs.
    pack_repo "modelcontextprotocol/modelcontextprotocol" \
      "mcp/repos/modelcontextprotocol-modelcontextprotocol.xml" \
      "docs/**,spec/**" "**/*.svg,**/*.excalidraw.svg"

    # MCP registry — server.json schema + publishing workflow reference.
    # Exclude large diagrams for the same reason as the protocol spec pack.
    pack_repo "modelcontextprotocol/registry" \
      "mcp/repos/modelcontextprotocol-registry.xml" \
      "" "**/*.svg,**/*.excalidraw.svg"

    # mcporter — integration test tool used in tests/mcporter/test-mcp.sh
    pack_repo "openclaw/mcporter" \
      "mcporter/repos/openclaw-mcporter.xml"

    # TEMPLATE: Add your service's repos here:
    # pack_repo "your-org/your-service" "your-service/repos/your-service.xml" \
    #   "api/**,src/**" "test/**,node_modules/**"
  fi

  # ── Sparse clones ─────────────────────────────────────────────────────────
  if [[ "$SKIP_REPOMIX" != true ]]; then
    # mcporter docs — sparse checkout of just the docs directory
    sparse_clone_path "https://github.com/openclaw/mcporter" "docs" "mcporter/docs" "recursive"
  fi

  # ── Finalize ──────────────────────────────────────────────────────────────
  if [[ "$DRY_RUN" != true ]]; then
    write_index
    snapshot_references "$after_snapshot"
    summarize_reference_changes "$before_snapshot" "$after_snapshot"
    rm -rf -- "$snapshot_dir"
  fi

  log "done"
}

main "$@"
