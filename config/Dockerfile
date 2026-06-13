# syntax=docker/dockerfile:1.7
# =============================================================================
# Dockerfile — Build the rustarr-mcp container image
#
# Build stages:
#   builder  — Rust compilation with BuildKit cache mounts (fast incremental builds)
#   runtime  — Minimal Debian image with only the binary + curl for health checks
# =============================================================================

# ── Stage 1: Builder ──────────────────────────────────────────────────────────
# To pin to a digest for reproducible builds:
#   docker pull rust:1.90-slim-bookworm
#   docker inspect --format '{{index .RepoDigests 0}}' rust:1.90-slim-bookworm
FROM rust:1.96-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies.
# pkg-config and libssl-dev are needed for openssl-sys (used by reqwest/lab-auth).
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# ── Dependency caching layer ──────────────────────────────────────────────────
# Copy only Cargo manifests first and build a stub binary. This lets Docker/
# BuildKit cache the compiled dependencies as a separate layer — subsequent
# builds that only change src/ skip this expensive step entirely.
COPY Cargo.toml Cargo.lock ./
# Strip xtask workspace member — not needed in Docker build context
RUN sed -i 's/, \"xtask\"//g; s/\"xtask\", //g; s/\"xtask\"//g' Cargo.toml

RUN --mount=type=cache,id=rustarr-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=rustarr-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=rustarr-cargo-target,target=/app/target,sharing=locked \
    mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release --locked -p rustarr && rm -rf src

# ── Application build ─────────────────────────────────────────────────────────
# Now copy the real source and build the actual binary.
# The `touch src/main.rs` forces Cargo to re-link even when timestamps haven't
# changed (Docker layer caching can confuse Cargo's incremental build detection).
COPY src/ src/
RUN --mount=type=cache,id=rustarr-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=rustarr-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=rustarr-cargo-target,target=/app/target,sharing=locked \
    touch src/main.rs && cargo build --release --locked -p rustarr && \
    cp target/release/rustarr /usr/local/bin/rustarr

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
# Minimal Debian image — no Rust toolchain, no build tools.
# Only ca-certificates (for TLS) and curl (for HEALTHCHECK).
# Pinned to bookworm-slim 2026-05-15 — update quarterly with:
#   docker pull debian:bookworm-slim
#   docker inspect --format '{{index .RepoDigests 0}}' debian:bookworm-slim
FROM debian:bookworm-slim

# Install runtime dependencies:
#   ca-certificates — TLS certificate verification for outbound HTTPS calls
#   curl            — used by HEALTHCHECK
#   gosu            — privilege drop in entrypoint.sh (exec as UID 1000:1000)
#
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl gosu && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/rustarr /usr/local/bin/rustarr

# Wire in the entrypoint.
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Create a non-root user and group for the service.
# UID/GID 1000 matches the host user convention in docker-compose.yml.
RUN groupadd --gid 1000 rustarr && \
    useradd --uid 1000 --gid rustarr --no-create-home --shell /sbin/nologin rustarr && \
    mkdir -p /data && \
    chown rustarr:rustarr /data

# Default log level. Override with RUST_LOG env var in docker-compose or at runtime.
ENV RUST_LOG=info

# ENV RUSTARR_MCP_HOST=0.0.0.0
# ENV RUSTARR_MCP_PORT=40070

# NOTE: No USER directive here — entrypoint.sh drops to UID 1000:1000 via gosu.
# This allows entrypoint.sh to run as root (for chown /data) before execing the binary.
# Pattern §26 + §16 (USER 1000:1000 via entrypoint, not Dockerfile USER directive).

EXPOSE 40070/tcp

# Health check hits /health (always unauthenticated in the rustarr design).
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:40070/health || exit 1

# MCP registry label — used by Claude Code plugin discovery.
LABEL io.modelcontextprotocol.server.name="tv.tootie/rustarr-mcp"

# Entrypoint handles permission setup and privilege drop.
# CMD is the default argument passed to entrypoint.sh.
ENTRYPOINT ["/entrypoint.sh"]
CMD ["serve", "mcp"]
