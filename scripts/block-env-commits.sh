#!/usr/bin/env bash
# =============================================================================
# scripts/block-env-commits.sh — Pre-commit guard: block .env secrets
#
# Called by lefthook.yml's env_guard hook.
#
# What it does:
#   Inspects the git staging area and blocks any commit that includes a .env
#   file (other than .env.yarr). This prevents accidentally committing API
#   keys, OAuth secrets, or bearer tokens.
#
# What it allows:
#   - .env.yarr  (the committed template — no real secrets)
#
# What it blocks:
#   - .env          (the real secrets file)
#   - .env.local    (local overrides with real values)
#   - .env.prod     (production secrets)
#   - .env.staging  (staging secrets)
#   - any other .env.* variant
#
# The pattern matches all .env* files regardless of directory depth or naming
# convention.
#
# Usage (called automatically by lefthook):
#   bash scripts/block-env-commits.sh
#
# Exit codes:
#   0  — no blocked files; commit may proceed
#   1  — blocked files found; commit is rejected
# =============================================================================
set -euo pipefail

# Get the list of files currently staged for commit.
staged=$(git diff --cached --name-only)

# Find any .env* files in the staged list, excluding .env.yarr.
# Regex breakdown:
#   (^|/)      — matches .env at root OR in any subdirectory
#   [^/]*      — matches any filename segment (no slashes)
#   \.env[^/]* — the filename starts with .env
#   $          — end of string (no trailing path)
blocked=$(printf '%s\n' "$staged" \
    | grep -E '(^|/)[^/]*\.env[^/]*$' \
    | grep -v '\.env\.yarr$' \
    || true)

if [[ -n "$blocked" ]]; then
    echo "block-env-commits: BLOCKED — .env file(s) staged for commit:" >&2
    echo "$blocked" | sed 's/^/  /' >&2
    echo "" >&2
    echo "Only .env.yarr is allowed to be committed." >&2
    echo "Remove the staged file(s) with: git restore --staged <file>" >&2
    echo "Then add them to .gitignore if they aren't already." >&2
    exit 1
fi

# All clear — no blocked .env files staged.
exit 0
