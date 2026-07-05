#!/usr/bin/env bash
# Stop, rebuild, and restart the yarr-mcp service.
# Must be run from the repository root.
# Supports systemd user units and Docker Compose.
set -euo pipefail

echo "==> Stopping yarr-mcp runtimes..."
stopped=false
for unit in yarr-mcp.service rustarr-mcp.service; do
    if systemctl --user is-active --quiet "${unit}" 2>/dev/null; then
        systemctl --user stop "${unit}"
        echo "    stopped systemd unit ${unit}"
        stopped=true
    fi
done
for container in yarr-mcp rustarr-mcp; do
    if docker ps --filter "name=^/${container}$" --quiet 2>/dev/null | grep -q .; then
        docker stop "${container}" >/dev/null
        echo "    stopped docker container ${container}"
        stopped=true
    fi
done
if [[ "${stopped}" == "false" ]]; then
    echo "    no running instance found"
fi

echo "==> Rebuilding release binary..."
cargo build --release

echo "==> Restarting..."
if systemctl --user list-unit-files yarr-mcp.service 2>/dev/null | grep -q yarr-mcp; then
    mkdir -p "${HOME}/.local/bin"
    install -m 755 target/release/yarr "${HOME}/.local/bin/yarr"
    systemctl --user start yarr-mcp.service
    echo "    started systemd unit"
elif systemctl --user list-unit-files rustarr-mcp.service 2>/dev/null | grep -q rustarr-mcp; then
    mkdir -p "${HOME}/.local/bin"
    install -m 755 target/release/yarr "${HOME}/.local/bin/yarr"
    systemctl --user start rustarr-mcp.service
    echo "    started legacy systemd unit rustarr-mcp.service"
elif [ -f docker-compose.yml ]; then
    docker compose build
    docker compose up -d --force-recreate
    echo "    started docker compose service"
else
    echo "    no service manager detected; binary at target/release/yarr"
fi

echo "==> Done"
