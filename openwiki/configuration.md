# Configuration

yarr uses a **split config approach** to keep secrets out of version control:

- **config.toml** — Ports, bind addresses, feature flags, timeouts, rate limits (safe to commit)
- **.env** — API URLs, API keys, tokens, OAuth credentials, passwords (NEVER commit)

Environment variables always override config.toml values.

## Quick setup

```bash
# Copy templates
cp config.example.toml config.toml
cp .env.example .env

# Edit .env with your service URLs and credentials
```

## Environment variables (.env)

### MCP server config

| Variable | Description | Default |
|----------|-------------|---------|
| `YARR_MCP_HOST` | Bind host for MCP HTTP server | `0.0.0.0` |
| `YARR_MCP_PORT` | Bind port for MCP HTTP server | `40070` |
| `YARR_MCP_SERVER_NAME` | Server name advertised to MCP clients | `yarr-mcp` |
| `YARR_MCP_NO_AUTH` | Disable authentication (loopback only) | `false` |
| `YARR_MCP_TOKEN` | Static bearer token for /mcp auth | (empty) |
| `YARR_MCP_ALLOWED_HOSTS` | Extra Host headers allowed (comma-separated) | (empty) |
| `YARR_MCP_ALLOWED_ORIGINS` | Extra CORS origins (comma-separated) | (empty) |

### Services

Services are configured through `YARR_SERVICES` + per-service variables:

```bash
# Declare which services to configure
export YARR_SERVICES=radarr,sonarr,prowlarr,plex

# Configure each service
export YARR_RADARR_URL=http://127.0.0.1:7878
export YARR_RADARR_API_KEY=your-radarr-api-key

export YARR_SONARR_URL=http://127.0.0.1:8989
export YARR_SONARR_API_KEY=your-sonarr-api-key

export YARR_PROWLARR_URL=http://127.0.0.1:9696
export YARR_PROWLARR_API_KEY=your-prowlarr-api-key

export YARR_PLEX_URL=http://127.0.0.1:32400
export YARR_PLEX_TOKEN=your-plex-token
```

### Service types and auth

yarr supports 11 service types with different auth styles:

| Service | `YARR_<NAME>_URL` | Auth variable | Auth style |
|---------|------------------|---------------|------------|
| **Sonarr** | `YARR_SONARR_URL` | `YARR_SONARR_API_KEY` | `X-Api-Key` header |
| **Radarr** | `YARR_RADARR_URL` | `YARR_RADARR_API_KEY` | `X-Api-Key` header |
| **Prowlarr** | `YARR_PROWLARR_URL` | `YARR_PROWLARR_API_KEY` | `X-Api-Key` header |
| **Overseerr** | `YARR_OVERSEERR_URL` | `YARR_OVERSEERR_API_KEY` | `X-Api-Key` header |
| **Bazarr** | `YARR_BAZARR_URL` | `YARR_BAZARR_API_KEY` | `X-Api-Key` header |
| **SABnzbd** | `YARR_SABNZBD_URL` | `YARR_SABNZBD_API_KEY` | Query string `apikey=` |
| **qBittorrent** | `YARR_QBITTORRENT_URL` | `YARR_QBITTORRENT_USER` + `YARR_QBITTORRENT_PASS` | Cookie session |
| **Plex** | `YARR_PLEX_URL` | `YARR_PLEX_TOKEN` | Query string `X-Plex-Token=` |
| **Jellyfin** | `YARR_JELLYFIN_URL` | `YARR_JELLYFIN_API_KEY` | `X-Emby-Token` header |
| **Tautulli** | `YARR_TAUTULLI_URL` | `YARR_TAUTULLI_API_KEY` | Query string `apikey=` |
| **Tracearr** | `YARR_TRACEARR_URL` | `YARR_TRACEARR_API_KEY` | `X-Api-Key` header |

### OAuth / JWT auth (optional)

Enable OAuth mode with Google account-based auth:

| Variable | Description |
|----------|-------------|
| `YARR_MCP_AUTH_MODE` | `bearer` (default) or `oauth` |
| `YARR_MCP_PUBLIC_URL` | Public URL of this MCP server (required in OAuth mode) |
| `YARR_MCP_GOOGLE_CLIENT_ID` | Google OAuth 2.0 client ID |
| `YARR_MCP_GOOGLE_CLIENT_SECRET` | Google OAuth 2.0 client secret |
| `YARR_MCP_AUTH_ADMIN_EMAIL` | Bootstrap admin email |
| `YARR_MCP_AUTH_ALLOWED_EMAILS` | Additional allowed emails (comma-separated) |

See `/docs/AUTH.md` for OAuth setup details.

### Runtime options

| Variable | Description | Default |
|----------|-------------|---------|
| `YARR_HTTP_TIMEOUT_SECS` | Per-request upstream timeout | `30` |
| `YARR_DATA_DIR` | Root directory for data (logs, snippets, artifacts) | `~/.yarr` |

## config.toml

The `config.toml` file holds non-secret defaults:

```toml
# Media automation services (usually configured via .env)
[yarr]
services = []

# MCP HTTP server
[mcp]
host = "0.0.0.0"
port = 40070
server_name = "yarr-mcp"

# Auth options
[mcp]
no_auth = false
api_token = ""  # Set via YARR_MCP_TOKEN instead
allowed_hosts = ["yarr.tootie.tv", "yarr.lan"]
allowed_origins = ["https://claude.ai", "http://localhost:5173"]

# OAuth / JWT (optional)
[mcp.auth]
mode = "bearer"
public_url = "https://yarr.tootie.tv"
google_client_id = ""
google_client_secret = ""
admin_email = ""
allowed_emails = []
```

### Bind address guidance

| Host value | When to use | Auth required? |
|------------|-------------|----------------|
| `127.0.0.1` | Local development only | No (loopback is trusted) |
| `0.0.0.0` | Docker, reverse proxy, LAN access | Yes (bearer or OAuth) |

Set `YARR_MCP_HOST=127.0.0.1` for local development with `YARR_MCP_NO_AUTH=true`.

## Service configuration

### config.toml (not recommended for URLs/keys)

```toml
[yarr]
services = [
    { name = "radarr", kind = "Radarr" },
    { name = "sonarr", kind = "Sonarr" },
]
```

### .env (recommended for URLs/keys)

```bash
export YARR_SERVICES=radarr,sonarr
export YARR_RADARR_URL=http://127.0.0.1:7878
export YARR_RADARR_API_KEY=your-api-key
export YARR_SONARR_URL=http://127.0.0.1:8989
export YARR_SONARR_API_KEY=your-api-key
```

## Validation

Validate your config before starting the server:

```bash
cargo xtask check-env
# or
just check-env
```

This prints the status of every required and optional variable, then exits non-zero if any required variable is missing.

## Priority order

1. **Environment variables** (highest priority)
2. **config.toml**
3. **Code defaults** (lowest priority)

Example: If `YARR_MCP_PORT=40070` is set in `.env`, it overrides `port = 3000` in `config.toml`.

## Path allowlists

Generic passthrough (`api_get`, `api_post`, etc.) is restricted to path prefixes configured per `ServiceKind` in `src/capability.rs`:

```rust
pub struct KindDescriptor {
    pub path_allowlist: &'static [&'static str],
    // ...
}
```

This prevents accidental calls to unexpected endpoints (e.g., `/api/v3/command` vs `/api/v3/series`). Paths containing `apikey=`, `token=`, or `X-Plex-Token` in the query string are rejected at the transport layer — credentials belong in config, not in user-supplied paths.

## Secrets security

- **Never commit** `.env`, `config.toml` with credentials, or any file containing API keys
- **Use `.env.example`** as a template — it contains placeholders only
- **Prefer environment variables** for secrets over config.toml
- **Rotate credentials** if they're ever accidentally committed

## Data directory

yarr stores runtime data in `~/.yarr` (or `YARR_DATA_DIR`):

```
~/.yarr/
  logs/
    yarr.log           # Server logs (truncated at 10MB on startup)
  snippets/            # Saved Code Mode snippets
  artifacts/           # Code Mode writeArtifact output (per-run subdirs)
```

## Further reading

- `/docs/ENV.md` — Detailed environment variable reference
- `/docs/CONFIG.md` — Config file format and options
- `/docs/AUTH.md` — OAuth/JWT authentication setup
