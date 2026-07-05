---
title: "Environment Variables"
doc_type: "guide"
status: "active"
owner: "yarr"
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

The template uses `YARR_*` variables. Rename the prefix when adapting the template.

## Upstream services

| Variable | Purpose |
|---|---|
| `YARR_SERVICES` | Comma-separated configured service names, for example `sonarr,radarr,plex`. |
| `YARR_<SERVICE>_KIND` | Optional service kind override. Defaults to the service name. |
| `YARR_<SERVICE>_URL` | Upstream service base URL. Required for each configured service. |
| `YARR_<SERVICE>_API_KEY` | API key for services that use `X-Api-Key`, query API keys, or token-compatible auth. |
| `YARR_<SERVICE>_USERNAME` | Username for services such as qBittorrent. |
| `YARR_<SERVICE>_PASSWORD` | Password for services such as qBittorrent. |
| `YARR_<SERVICE>_TOKEN` | Bearer/token auth for services such as Plex or Jellyfin. |
| `YARR_HTTP_TIMEOUT_SECS` | Per-request upstream timeout in seconds (default `30`). Raise for stacks with slow upstreams (e.g. a Prowlarr `/indexer` read that fans out to many trackers). `0`/unparseable falls back to `30`. |

## MCP HTTP server

| Variable | Default | Purpose |
|---|---:|---|
| `YARR_MCP_HOST` | `127.0.0.1` | Bind host for HTTP transport. Set `0.0.0.0` only with bearer, OAuth, or trusted-gateway auth configured. |
| `YARR_MCP_PORT` | `40070` | Bind port for HTTP transport. |
| `YARR_MCP_NO_AUTH` | `false` | Disable local auth for loopback development only. |
| `YARR_NOAUTH` | `false` | Trusted-gateway no-auth mode for non-loopback deployments. Requires explicit `YARR_MCP_ALLOWED_HOSTS` or `YARR_MCP_ALLOWED_ORIGINS` provenance. |
| `YARR_MCP_TOKEN` | unset | Static bearer token. Required for bearer-only mounted HTTP. |
| `YARR_MCP_ALLOWED_HOSTS` | unset | Extra accepted Host header values (comma-separated). |
| `YARR_MCP_ALLOWED_ORIGINS` | unset | Extra CORS origins (comma-separated). |
| `YARR_MCP_PUBLIC_URL` | unset | Public URL used for OAuth metadata endpoints. |
| `YARR_MCP_AUTH_MODE` | `bearer` | `bearer` or `oauth`. |

## OAuth mode

Only required when `YARR_MCP_AUTH_MODE=oauth`:

| Variable | Purpose |
|---|---|
| `YARR_MCP_GOOGLE_CLIENT_ID` | Google OAuth client ID. |
| `YARR_MCP_GOOGLE_CLIENT_SECRET` | Google OAuth client secret. |
| `YARR_MCP_AUTH_ADMIN_EMAIL` | Initial/admin email allowed by the OAuth flow. |

## Docker runtime

| Variable | Purpose |
|---|---|
| `PUID` | UID to run the container as (default: 1000). |
| `PGID` | GID to run the container as (default: 1000). |
| `DOCKER_NETWORK` | Docker network name (default: `mcp`). |
| `VERSION` | Image tag to pull (default: `latest`). |

## Logging

| Variable | Yarr | Purpose |
|---|---|---|
| `RUST_LOG` | `info,rmcp=warn` | Tracing filter. |
| `NO_COLOR` | `1` | Disable ANSI color in console logs. |
| `FORCE_COLOR` | `1` | Force ANSI color even when stderr is not a TTY. |

## `.env` file structure

```bash
# .env — secrets and URLs ONLY
YARR_SERVICES=sonarr,radarr,plex
YARR_SONARR_URL=https://sonarr.internal
YARR_SONARR_API_KEY=your_sonarr_key_here
YARR_RADARR_URL=https://radarr.internal
YARR_RADARR_API_KEY=your_radarr_key_here
YARR_PLEX_URL=https://plex.internal
YARR_PLEX_TOKEN=your_plex_token_here

# MCP auth
YARR_MCP_TOKEN=your_bearer_token_here

# OAuth (only when auth_mode=oauth in config.toml)
# YARR_MCP_GOOGLE_CLIENT_ID=...
# YARR_MCP_GOOGLE_CLIENT_SECRET=...

# Docker runtime
PUID=1000
PGID=1000
DOCKER_NETWORK=mcp
RUST_LOG=info
```

## Safety

`.env` and `.env.*` are ignored by `.gitignore` and blocked by `scripts/block-env-commits.sh`. Only `.env.yarr` belongs in git.

Non-secret settings (host, port, auth mode, TTLs) go in `config.toml`, not `.env`. See `docs/CONFIG.md` for the full split.

Generate a bearer token:

```bash
just gen-token
# or: openssl rand -hex 32
```

See `docs/CONFIG.md` for the config loading pattern and auth policy details.
