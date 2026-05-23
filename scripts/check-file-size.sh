#!/usr/bin/env bash
# Prevent monolithic staged source files from being committed.
#
# Checks staged .rs / .ts / .tsx files against per-type effective line limits.
# Test files are exempt. Rust inline test modules are excluded from the count.
set -euo pipefail

MAX_RS="${MAX_RS:-350}"
MAX_TS="${MAX_TS:-300}"

is_test_file() {
    local f="$1"
    [[ "$f" =~ (/tests?/|_test\.rs$|/tests\.rs$) ]] && return 0
    [[ "$f" =~ (\.(test|spec)\.(ts|tsx)$|/__tests__/) ]] && return 0
    return 1
}

count_effective_loc() {
    local f="$1"
    local end_line="${2:-0}"
    awk -v end_line="$end_line" '
        BEGIN { count=0; in_block=0 }
        end_line > 0 && NR > end_line { exit }
        {
            line=$0
            sub(/^[[:space:]]+/, "", line)
            if (line == "") next

            if (in_block) {
                if (line ~ /\*\//) {
                    sub(/^.*\*\//, "", line)
                    sub(/^[[:space:]]+/, "", line)
                    in_block=0
                    if (line == "") next
                } else {
                    next
                }
            }

            if (line ~ /^\/\//) next

            if (line ~ /^\/\*/) {
                if (line ~ /\*\//) {
                    sub(/^\/\*.*\*\//, "", line)
                    sub(/^[[:space:]]+/, "", line)
                    if (line == "") next
                } else {
                    in_block=1
                    next
                }
            }

            count++
        }
        END { print count }
    ' "$f"
}

rs_production_lines() {
    local f="$1"
    local end_line=0
    # Only exclude a trailing test *module* — #[cfg(test)] immediately preceding `mod <name> {`.
    # Stops at the first such pair to avoid cutting off production code annotated with
    # other #[cfg(test)] attributes (e.g. on individual functions or impls).
    local test_mod_line
    test_mod_line=$(awk '
        /#\[cfg\(test\)\]/ { cfg_line = NR; next }
        cfg_line && /^[[:space:]]*(pub[[:space:]]+)?mod [a-z_]+ \{/ { print cfg_line; exit }
        { cfg_line = 0 }
    ' "$f" || true)
    if [[ -n "$test_mod_line" ]]; then
        end_line=$(( test_mod_line - 1 ))
    fi
    count_effective_loc "$f" "$end_line"
}

violations=()

while IFS= read -r file; do
    [[ -f "$file" ]] || continue
    is_test_file "$file" && continue

    case "$file" in
        *.rs)
            lines=$(rs_production_lines "$file")
            limit=$MAX_RS
            ;;
        *.ts|*.tsx)
            lines=$(count_effective_loc "$file")
            limit=$MAX_TS
            ;;
        *) continue ;;
    esac

    if (( lines > limit )); then
        violations+=("  ${file}: ${lines} effective lines (limit: ${limit})")
    fi
done < <(git diff --cached --name-only --diff-filter=ACM)

if (( ${#violations[@]} > 0 )); then
    echo "" >&2
    echo "Monolithic staged file(s) detected; split them into focused modules:" >&2
    printf '%s\n' "${violations[@]}" >&2
    echo "" >&2
    echo "Limits: .rs=${MAX_RS} production lines, .ts/.tsx=${MAX_TS} lines; test files exempt." >&2
    exit 1
fi
