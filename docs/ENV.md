---
title: "Environment Variables"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
upstream_refs:
  - "src/config.rs"
last_reviewed: "2026-05-15"
---

# Environment variables

The template uses `RUSTARR_*` variables. Rename the prefix when adapting the template.

## Upstream services

| Variable | Purpose |
|---|---|
| `RUSTARR_SERVICES` | Comma-separated configured service names, for example `sonarr,radarr,plex`. |
| `RUSTARR_<SERVICE>_KIND` | Optional service kind override. Defaults to the service name. |
| `RUSTARR_<SERVICE>_URL` | Upstream service base URL. Required for each configured service. |
| `RUSTARR_<SERVICE>_API_KEY` | API key for services that use `X-Api-Key`, query API keys, or token-compatible auth. |
| `RUSTARR_<SERVICE>_USERNAME` | Username for services such as qBittorrent. |
| `RUSTARR_<SERVICE>_PASSWORD` | Password for services such as qBittorrent. |
| `RUSTARR_<SERVICE>_TOKEN` | Bearer/token auth for services such as Plex or Jellyfin. |
| `RUSTARR_HTTP_TIMEOUT_SECS` | Per-request upstream timeout in seconds (default `30`). Raise for stacks with slow upstreams (e.g. a Prowlarr `/indexer` read that fans out to many trackers). `0`/unparseable falls back to `30`. |

## MCP HTTP server

| Variable | Default | Purpose |
|---|---:|---|
| `RUSTARR_MCP_HOST` | `127.0.0.1` | Bind host for HTTP transport. Set `0.0.0.0` only with bearer, OAuth, or trusted-gateway auth configured. |
| `RUSTARR_MCP_PORT` | `40070` | Bind port for HTTP transport. |
| `RUSTARR_MCP_NO_AUTH` | `false` | Disable local auth for loopback development only. |
| `RUSTARR_NOAUTH` | `false` | Trusted-gateway no-auth mode for non-loopback deployments. Requires explicit `RUSTARR_MCP_ALLOWED_HOSTS` or `RUSTARR_MCP_ALLOWED_ORIGINS` provenance. |
| `RUSTARR_MCP_TOKEN` | unset | Static bearer token. Required for bearer-only mounted HTTP. |
| `RUSTARR_MCP_ALLOWED_HOSTS` | unset | Extra accepted Host header values (comma-separated). |
| `RUSTARR_MCP_ALLOWED_ORIGINS` | unset | Extra CORS origins (comma-separated). |
| `RUSTARR_MCP_PUBLIC_URL` | unset | Public URL used for OAuth metadata endpoints. |
| `RUSTARR_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth`. |

## OAuth mode

Only required when `RUSTARR_MCP_AUTH_MODE=oauth`:

| Variable | Purpose |
|---|---|
| `RUSTARR_MCP_GOOGLE_CLIENT_ID` | Google OAuth client ID. |
| `RUSTARR_MCP_GOOGLE_CLIENT_SECRET` | Google OAuth client secret. |
| `RUSTARR_MCP_AUTH_ADMIN_EMAIL` | Initial/admin email allowed by the OAuth flow. |

## Docker runtime

| Variable | Purpose |
|---|---|
| `PUID` | UID to run the container as (default: 1000). |
| `PGID` | GID to run the container as (default: 1000). |
| `DOCKER_NETWORK` | Docker network name (default: `mcp`). |
| `VERSION` | Image tag to pull (default: `latest`). |

## Logging

| Variable | Rustarr | Purpose |
|---|---|---|
| `RUST_LOG` | `info,rmcp=warn` | Tracing filter. |
| `NO_COLOR` | `1` | Disable ANSI color in console logs. |
| `FORCE_COLOR` | `1` | Force ANSI color even when stderr is not a TTY. |

## `.env` file structure

```bash
# .env — secrets and URLs ONLY
RUSTARR_SERVICES=sonarr,radarr,plex
RUSTARR_SONARR_URL=https://sonarr.internal
RUSTARR_SONARR_API_KEY=your_sonarr_key_here
RUSTARR_RADARR_URL=https://radarr.internal
RUSTARR_RADARR_API_KEY=your_radarr_key_here
RUSTARR_PLEX_URL=https://plex.internal
RUSTARR_PLEX_TOKEN=your_plex_token_here

# MCP auth
RUSTARR_MCP_TOKEN=your_bearer_token_here

# OAuth (only when auth_mode=oauth in config.toml)
# RUSTARR_MCP_GOOGLE_CLIENT_ID=...
# RUSTARR_MCP_GOOGLE_CLIENT_SECRET=...

# Docker runtime
PUID=1000
PGID=1000
DOCKER_NETWORK=mcp
RUST_LOG=info
```

## Safety

`.env` and `.env.*` are ignored by `.gitignore` and blocked by `scripts/block-env-commits.sh`. Only `.env.rustarr` belongs in git.

Non-secret settings (host, port, auth mode, TTLs) go in `config.toml`, not `.env`. See `docs/CONFIG.md` for the full split.

Generate a bearer token:

```bash
just gen-token
# or: openssl rand -hex 32
```

See `docs/CONFIG.md` for the config loading pattern and auth policy details.
