#!/usr/bin/env bash
# Stop, rebuild, and restart the rustarr-mcp service.
# Must be run from the repository root.
# Supports systemd user units and Docker Compose.
set -euo pipefail

echo "==> Stopping rustarr-mcp..."
if systemctl --user is-active --quiet rustarr-mcp.service 2>/dev/null; then
    systemctl --user stop rustarr-mcp.service
    echo "    stopped systemd unit"
elif docker ps --filter 'name=^/rustarr-mcp$' --quiet 2>/dev/null | grep -q .; then
    docker stop rustarr-mcp >/dev/null
    echo "    stopped docker container"
else
    echo "    no running instance found"
fi

echo "==> Rebuilding release binary..."
cargo build --release

echo "==> Restarting..."
if systemctl --user list-unit-files rustarr-mcp.service 2>/dev/null | grep -q rustarr-mcp; then
    mkdir -p "${HOME}/.local/bin"
    install -m 755 target/release/rustarr "${HOME}/.local/bin/rustarr"
    systemctl --user start rustarr-mcp.service
    echo "    started systemd unit"
elif [ -f docker-compose.yml ]; then
    docker compose build
    docker compose up -d --force-recreate
    echo "    started docker compose service"
else
    echo "    no service manager detected; binary at target/release/rustarr"
fi

echo "==> Done"
