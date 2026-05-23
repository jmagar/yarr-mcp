#!/bin/sh
# =============================================================================
# entrypoint.sh — Docker entrypoint for rustarr
#
# TEMPLATE: This script runs as root before dropping privileges to UID 1000.
#           Copy it to the repo root and update the TEMPLATE sections below.
#
# Pattern §26: Every Docker image runs this entrypoint before the binary.
#   1. Creates the data directory if it doesn't exist
#   2. Fixes ownership so the service user can write files
#   3. Hardens file permissions on sensitive config
#   4. Validates required env vars (fast-fail with a clear error)
#   5. Drops to UID 1000:1000 and exec's the binary
#
# Dockerfile wires this in:
#   COPY entrypoint.sh /entrypoint.sh
#   RUN chmod +x /entrypoint.sh
#   ENTRYPOINT ["/entrypoint.sh"]
#   CMD ["rustarr", "serve", "mcp"]
#
# The ENTRYPOINT + CMD split means:
#   - `docker run image`                   → runs: /entrypoint.sh rustarr serve mcp
#   - `docker run image rustarr --help`    → runs: /entrypoint.sh rustarr --help
#   - `docker run image sh`                → runs: /entrypoint.sh sh  (useful for debugging)
#
# TEMPLATE: Update REQUIRED_VARS and binary name below.
# =============================================================================
set -e

SERVICE_NAME="rustarr"
BINARY="/usr/local/bin/${SERVICE_NAME}"

# ── Data directory ─────────────────────────────────────────────────────────────
# TEMPLATE: DATA_DIR is the container's persistent storage path. It is always
#           /data inside the container and bind-mounted from ~/.<service>/ on
#           the host via docker-compose.yml.
#           DO NOT change this to a non-/data path inside the container.
DATA_DIR="${DATA_DIR:-/data}"

# Create the data directory if it doesn't exist.
# This is idempotent — running twice is safe.
mkdir -p "${DATA_DIR}"

# Fix ownership so the service user (UID 1000) can write files when the mount
# permits it. Some host bind mounts reject recursive chown even when UID 1000
# already owns the directory, so keep startup tolerant and let the app's own
# write checks surface real permission problems.
if ! chown -R 1000:1000 "${DATA_DIR}" 2>/dev/null; then
    echo "WARN: could not chown ${DATA_DIR} to 1000:1000; continuing" >&2
fi

# Restrict directory permissions when supported by the mount:
#   750 = owner rwx, group rx, others nothing
# This prevents other processes in the container from reading the data dir.
if ! chmod 750 "${DATA_DIR}" 2>/dev/null; then
    echo "WARN: could not chmod ${DATA_DIR} to 750; continuing" >&2
fi

# ── Harden sensitive files ────────────────────────────────────────────────────
# If config.toml exists, make it group-readable but not world-readable.
# TEMPLATE: Add similar blocks for any other sensitive files your service writes.
if [ -f "${DATA_DIR}/config.toml" ]; then
    chmod 640 "${DATA_DIR}/config.toml"
fi

# .env must not be world-readable — it contains API keys.
if [ -f "${DATA_DIR}/.env" ]; then
    chmod 600 "${DATA_DIR}/.env"
fi

# JWT signing key — extremely sensitive; owner-only.
if [ -f "${DATA_DIR}/auth-jwt.pem" ]; then
    chmod 600 "${DATA_DIR}/auth-jwt.pem"
fi

# Auth database — owner + group read, no others.
if [ -f "${DATA_DIR}/auth.db" ]; then
    chmod 640 "${DATA_DIR}/auth.db"
fi

# ── Validate required environment variables ────────────────────────────────────
# TEMPLATE: Add your service's required env vars here.
#           Comment out or remove lines for vars that have safe defaults.
#           The goal: fail loudly here rather than silently misbehave later.
#
# Rustarr supports many optional upstream services. The binary validates
# RUSTARR_SERVICES and per-service credentials in `rustarr doctor` and
# `rustarr setup check` so Docker startup can remain usable while operators
# add services incrementally.

# ── Drop privileges and exec the service ──────────────────────────────────────
# `gosu` (Alpine) or `gosu` (Debian/Ubuntu) replaces the current process
# with the service binary running as UID 1000:1000.
#
# TEMPLATE: The Dockerfile installs gosu (Alpine) or gosu (Debian).
#           The current Dockerfile uses Debian, so install gosu there:
#             RUN apt-get install -y gosu
#           and replace gosu below with gosu.
#
# `exec` is critical — it replaces the shell process so the binary receives
# OS signals (SIGTERM, SIGINT) directly. Without exec, the shell would buffer
# signals and Docker's stop timeout would kill the container ungracefully.
#
# TEMPLATE: Replace "gosu" with "gosu" if using a Debian-based image,
#           or use "exec setpriv --reuid=1000 --regid=1000 --clear-groups" if
#           neither gosu nor gosu is available.
# TEMPLATE: This image uses Debian + gosu. For Alpine, replace "gosu" with "gosu".
# Passthrough: if the first argument is not a known subcommand (e.g. docker run ... bash),
# exec it directly under gosu without prepending the binary.
case "${1:-}" in
  serve|mcp|integrations|status|get|post|watch|doctor|setup|help|--help|-h|--version|"")
    if [ "$(id -u)" = "0" ]; then
      exec gosu 1000:1000 "${BINARY}" "$@"
    fi
    exec "${BINARY}" "$@"
    ;;
  *)
    if [ "$(id -u)" = "0" ]; then
      exec gosu 1000:1000 "$@"
    fi
    exec "$@"
    ;;
esac
