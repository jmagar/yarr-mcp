#!/usr/bin/env bash
# Stop, rebuild, and restart the example-mcp service.
# Must be run from the repository root.
# Supports systemd user units and Docker Compose.
set -euo pipefail

echo "==> Stopping example-mcp..."
if systemctl --user is-active --quiet example-mcp.service 2>/dev/null; then
    systemctl --user stop example-mcp.service
    echo "    stopped systemd unit"
elif docker ps --filter 'name=^/example-mcp$' --quiet 2>/dev/null | grep -q .; then
    docker stop example-mcp >/dev/null 2>&1 || true
    echo "    stopped docker container"
else
    echo "    no running instance found"
fi

echo "==> Rebuilding release binary..."
cargo build --release

echo "==> Restarting..."
if systemctl --user list-unit-files example-mcp.service 2>/dev/null | grep -q example-mcp; then
    mkdir -p "${HOME}/.local/bin"
    install -m 755 target/release/example "${HOME}/.local/bin/example"
    systemctl --user start example-mcp.service
    echo "    started systemd unit"
elif [ -f docker-compose.yml ]; then
    docker compose build
    docker compose up -d --force-recreate
    echo "    started docker compose service"
else
    echo "    no service manager detected; binary at target/release/example"
fi

echo "==> Done"
