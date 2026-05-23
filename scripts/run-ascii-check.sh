#!/usr/bin/env bash
# Check (or fix) tracked source/config/docs files for non-ASCII characters.
# Must be run from the repository root.
#
# Usage:
#   scripts/run-ascii-check.sh          # check mode (default)
#   scripts/run-ascii-check.sh --fix    # rewrite smart punctuation to ASCII
set -euo pipefail

args=()
if [[ "${1:-}" == "--fix" ]]; then
    args+=("--fix")
fi

mapfile -t files < <(
    git ls-files '*.md' '*.rs' '*.toml' '*.json' '*.yml' '*.yaml' '*.sh' '*.py' \
        ':!:docs/references/**' ':!:docs/sessions/**' \
        | while IFS= read -r file; do [[ -f "$file" ]] && printf '%s\n' "$file"; done
)

if [[ "${#files[@]}" -eq 0 ]]; then
    echo "No files to check"
    exit 0
fi

python3 scripts/asciicheck.py "${args[@]}" "${files[@]}"
