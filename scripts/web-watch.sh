#!/usr/bin/env bash
# Watch apps/web for changes and rebuild on save.
# Must be run from the repository root.
# Requires watchexec: cargo install watchexec-cli
set -euo pipefail

if ! command -v watchexec >/dev/null 2>&1; then
    echo "error: watchexec is required for web-watch" >&2
    echo "install: cargo install watchexec-cli" >&2
    exit 1
fi

echo "Building apps/web once..."
bash scripts/build-web.sh

echo "Watching apps/web for changes..."
watchexec \
    --project-origin . \
    --watch apps/web \
    --ignore 'apps/web/.next' \
    --ignore 'apps/web/.next/**' \
    --ignore 'apps/web/out' \
    --ignore 'apps/web/out/**' \
    --ignore 'apps/web/node_modules' \
    --ignore 'apps/web/node_modules/**' \
    --debounce 1000ms \
    --on-busy-update queue \
    --wrap-process=none \
    'bash scripts/build-web.sh'
