---
title: "Configuration"
doc_type: "guide"
status: "active"
owner: "rmcp-template"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "docs/PATTERNS.md"
last_reviewed: "2026-05-15"
---

# Configuration

Configuration is split between non-secret settings (`config.toml`) and secrets (`.env`). Env vars always override `config.toml`.

## Files

| File | Purpose |
|---|---|
| `.env.example` | Documented environment variable template. Safe to commit. |
| `.env` | Local secrets and deployment settings. Never commit. |
| `config.example.toml` | Optional structured config example for derived services. |
| `src/config.rs` | Loads env/config into typed Rust structs. |

## What goes where

| Goes in `.env` | Goes in `config.toml` |
|---|---|
| API keys, tokens, passwords | bind host, port, server_name |
| Service URLs | TLS skip, site, tailnet |
| Google OAuth credentials | auth_mode, auth TTLs |
| MCP bearer token | allowed_hosts, allowed_origins |
| Docker runtime vars (PUID, PGID) | retention settings, batch sizes |
| RUST_LOG | resource limits |

## config.toml structure

```toml
# config.toml — non-secret settings only
# Env vars override everything here.

[mcp]
host = "127.0.0.1"
port = 40060
server_name = "example-mcp"
no_auth = false
trusted_gateway = false
allowed_hosts = []
allowed_origins = []

[mcp.auth]
mode = "bearer"           # or "oauth"
admin_email = ""
sqlite_path = "/data/auth.db"
key_path = "/data/auth-jwt.pem"
access_token_ttl_secs = 3600
refresh_token_ttl_secs = 2592000
auth_code_ttl_secs = 300
```

## .env structure

```bash
# .env — secrets and URLs ONLY
EXAMPLE_API_URL=https://example.internal/api
EXAMPLE_API_KEY=your_api_key_here

# MCP auth
EXAMPLE_MCP_TOKEN=your_bearer_token_here

# OAuth (only when auth_mode=oauth in config.toml)
# EXAMPLE_MCP_GOOGLE_CLIENT_ID=...
# EXAMPLE_MCP_GOOGLE_CLIENT_SECRET=...

# Docker runtime
PUID=1000
PGID=1000
DOCKER_NETWORK=mcp
RUST_LOG=info
```

## Config loading pattern

`src/config.rs` is the source of truth. `Config::load()` starts from typed defaults, loads the first readable `config.toml` from `~/.example/config.toml` or `./config.toml`, then applies env overrides.

Current env overrides include:

| Config field | Env var |
|---|---|
| `mcp.host` | `EXAMPLE_MCP_HOST` |
| `mcp.port` | `EXAMPLE_MCP_PORT` |
| `mcp.server_name` | `EXAMPLE_MCP_SERVER_NAME` |
| `mcp.no_auth` | `EXAMPLE_MCP_NO_AUTH` |
| `mcp.trusted_gateway` | `EXAMPLE_NOAUTH` |
| `mcp.api_token` | `EXAMPLE_MCP_TOKEN` |
| `mcp.allowed_hosts` | `EXAMPLE_MCP_ALLOWED_HOSTS` |
| `mcp.allowed_origins` | `EXAMPLE_MCP_ALLOWED_ORIGINS` |
| `mcp.auth.public_url` | `EXAMPLE_MCP_PUBLIC_URL` |
| `mcp.auth.mode` | `EXAMPLE_MCP_AUTH_MODE` |
| `mcp.auth.google_client_id` | `EXAMPLE_MCP_GOOGLE_CLIENT_ID` |
| `mcp.auth.google_client_secret` | `EXAMPLE_MCP_GOOGLE_CLIENT_SECRET` |
| `mcp.auth.admin_email` | `EXAMPLE_MCP_AUTH_ADMIN_EMAIL` |
| `example.api_url` | `EXAMPLE_API_URL` |
| `example.api_key` | `EXAMPLE_API_KEY` |

## Auth policy summary

| Situation | Policy |
|---|---|
| Stdio transport | `LoopbackDev` |
| Loopback bind or `EXAMPLE_MCP_NO_AUTH=true` | `LoopbackDev` |
| Non-loopback with bearer token | `Mounted { auth_state: None }` |
| OAuth mode (`auth_mode=oauth`) | `Mounted { auth_state: Some(_) }` |
| Explicit trusted gateway (`EXAMPLE_NOAUTH=true`) | `TrustedGatewayUnscoped` |

Non-loopback no-auth should only be used when an upstream gateway enforces authorization.

```rust
pub enum AuthPolicy {
    /// No auth — only legal when bound to loopback (127.x).
    LoopbackDev,
    /// Auth active. auth_state=Some → OAuth+JWKS; auth_state=None → bearer-only.
    Mounted { auth_state: Option<Arc<lab_auth::state::AuthState>> },
}
```

## Defaults

- Host defaults to `127.0.0.1` for HTTP serving.
- Port defaults to `40060`.
- Appdata defaults to `~/.<service>` locally, `/data` in Docker.

## Validation

```bash
just doctor
cargo xtask check-env
scripts/check-version-sync.sh
```

See `docs/ENV.md` for variable-by-variable reference. See `docs/PATTERNS.md` §4 and §5 for the full config split and auth policy patterns.
