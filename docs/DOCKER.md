---
title: "Docker"
doc_type: "guide"
status: "active"
owner: "yarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-05-15"
---

# Docker

Docker support lives in `config/Dockerfile` and `docker-compose.yml`.

## Common commands

```bash
just docker-build      # build image
just docker-up         # start compose stack
just docker-down       # stop stack
just docker-rebuild    # rebuild image and recreate container
just docker-logs       # follow logs
just runtime-current   # compare running image with local compose image
```

## Dockerfile pattern

```dockerfile
# syntax=docker/dockerfile:1.7
FROM rust:1.90-slim-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,id=yarr-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=yarr-cargo-target,target=/app/target,sharing=locked \
    mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --locked && rm -rf src

# Build real binary
COPY src/ src/
RUN --mount=type=cache,id=yarr-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=yarr-cargo-target,target=/app/target,sharing=locked \
    touch src/main.rs && cargo build --release --locked && \
    cp target/release/yarr /usr/local/bin/yarr

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/yarr /usr/local/bin/yarr
RUN groupadd --gid 1000 yarr && \
    useradd --uid 1000 --gid yarr --no-create-home --shell /sbin/nologin yarr && \
    mkdir -p /data && chown yarr:yarr /data

USER 1000:1000
EXPOSE 40070/tcp
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:40070/health || exit 1
CMD ["yarr", "serve", "mcp"]
```

## docker-compose.yml pattern

```yaml
services:
  yarr-mcp:
    image: ghcr.io/jmagar/yarr:${VERSION:-latest}
    build:
      context: .
      dockerfile: config/Dockerfile
    container_name: yarr-mcp
    restart: unless-stopped
    user: "${PUID:-1000}:${PGID:-1000}"
    env_file:
      - path: .env
        required: false
    ports:
      - "${YARR_MCP_HOST_PORT:-40070}:40070/tcp"
    volumes:
      - ${HOME}/.yarr:/data
    networks:
      - mcp
    healthcheck:
      test: ["CMD-SHELL", "curl -sf http://localhost:40070/health || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    deploy:
      resources:
        limits:
          memory: 256M
          cpus: '0.5'

networks:
  mcp:
    name: ${DOCKER_NETWORK:-mcp}
    external: true
```

Key requirements:
- `yarr-mcp` is the Compose service/container name; the image/package is `ghcr.io/jmagar/yarr`, and the binary is `/usr/local/bin/yarr`.
- `container_name` must be unique across your stack.
- Use the `${DOCKER_NETWORK:-mcp}` external network.
- `env_file.required: false` so the container starts without `.env` (relies on `config.toml` defaults).
- Resource limits to prevent runaway memory on homelab.

Create the network if it doesn't exist:

```bash
docker network create mcp
```

## Appdata convention

Local binary and Docker use the same data directory:

| Deployment | Data directory |
|---|---|
| Local binary | `~/.yarr/` |
| Docker | `/data/` inside container, mounted from `~/.yarr/` on host |
| Plugin | `$CLAUDE_PLUGIN_DATA` (symlinked to `~/.yarr/`) |

```rust
fn default_data_dir() -> PathBuf {
    if std::path::Path::new("/.dockerenv").exists() {
        return PathBuf::from("/data");
    }
    dirs::home_dir().unwrap_or_default().join(".yarr")
}
```

## Docker entrypoint

Every Docker image has an `entrypoint.sh` that runs as root, fixes permissions, validates required env vars, then drops to UID 1000:

```bash
#!/bin/sh
set -e
DATA_DIR="${DATA_DIR:-/data}"

# Validate required service inventory before starting, if this image should be strict.
[ -z "${YARR_SERVICES:-}" ] && { echo "FATAL: YARR_SERVICES is not set" >&2; exit 1; }

mkdir -p "${DATA_DIR}/logs"
chown -R 1000:1000 "${DATA_DIR}"

# Secure secret files
for f in "${DATA_DIR}/.env" "${DATA_DIR}/auth-jwt.pem"; do
    [ -f "${f}" ] && chmod 600 "${f}" || true
done

exec su-exec 1000:1000 "$@"
```

Key principles: fail fast, check every assumption, `exec` not `run` (so PID 1 is the actual service), no signal traps.

## Health and auth

- Healthcheck probes `/health`.
- `/mcp` requires auth outside loopback unless explicitly behind a trusted gateway.
- Use `scripts/test-mcp-auth.sh` for bearer auth smoke tests.
- Recreate (don't just restart) the container after editing `.env`:

```bash
docker compose up -d --force-recreate
```

## Build artifacts

`just build-plugin` copies the release binary to both `bin/yarr` and `plugins/yarr/bin/yarr`. The plugin binary path is allowlisted in `scripts/blob-size-allowlist.txt`.

See `docs/PATTERNS.md` §14, §15, §25, §26, §50 for the full Dockerfile, compose, appdata, and entrypoint patterns.
