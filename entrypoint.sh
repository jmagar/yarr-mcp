#!/bin/sh
# =============================================================================
# entrypoint.sh — Docker entrypoint for yarr
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
#   CMD ["yarr", "serve", "mcp"]
#
# The ENTRYPOINT + CMD split means:
#   - `docker run image`                   → runs: /entrypoint.sh yarr serve mcp
#   - `docker run image yarr --help`    → runs: /entrypoint.sh yarr --help
#   - `docker run image sh`                → runs: /entrypoint.sh sh  (useful for debugging)
#
# =============================================================================
set -e

SERVICE_NAME="yarr"
BINARY="/usr/local/bin/${SERVICE_NAME}"

# ── Data directory ─────────────────────────────────────────────────────────────
# DATA_DIR is the container's persistent storage path. It is always /data inside
# the container and bind-mounted from ~/.yarr on the host via Compose.
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
    chmod 600 "${DATA_DIR}/auth.db"
fi

# ── Validate required environment variables ────────────────────────────────────
# Yarr supports many optional upstream services. The binary validates
# YARR_SERVICES and per-service credentials in `yarr doctor` and
# `yarr setup check` so Docker startup can remain usable while operators
# add services incrementally.

# ── Drop privileges and exec the service ──────────────────────────────────────
# `gosu` (Alpine) or `gosu` (Debian/Ubuntu) replaces the current process
# with the service binary running as UID 1000:1000.
#
# `exec` is critical — it replaces the shell process so the binary receives
# OS signals (SIGTERM, SIGINT) directly. Without exec, the shell would buffer
# signals and Docker's stop timeout would kill the container ungracefully.
#
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
