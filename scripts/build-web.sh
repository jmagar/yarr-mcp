#!/usr/bin/env bash
# Build the Next.js web UI static export.
# Must be run from the repository root.
# Output lands in apps/web/out/ and is baked into the binary via the `web` feature.
set -euo pipefail

if [ ! -d apps/web ]; then
    echo "No apps/web directory found — skipping web build"
    exit 0
fi

cd apps/web

if [ ! -d node_modules ]; then
    echo "Installing web dependencies..."
    pnpm install --frozen-lockfile
fi

pnpm build
echo "Web assets built → apps/web/out/"
