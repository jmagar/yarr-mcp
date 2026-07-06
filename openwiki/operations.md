# Operations

Deployment, runtime, monitoring, and maintenance for yarr.

## Running yarr

### Development

```bash
# Run from source
cargo run -- serve           # HTTP MCP server
cargo run -- mcp             # stdio MCP server
cargo run -- radarr status   # CLI command

# With just(1)
just serve                   # HTTP server
just mcp                     # stdio
just dev-server              # HTTP with auto-reload
```

### Production

```bash
# HTTP MCP server
yarr serve

# stdio MCP server (for local child-process clients)
yarr mcp

# CLI
yarr radarr status
```

## Modes

yarr supports three run modes selected by the first CLI argument:

| Mode | Entry point | Use case |
|------|-------------|----------|
| **HTTP server** | `yarr serve` | MCP over HTTP with auth |
| **stdio** | `yarr mcp` | MCP over stdio (local agents only, no auth) |
| **CLI** | `yarr <service> <command>` | Scripting, debugging, automation |

Mode dispatch happens in `src/main.rs` — the binary parses `argv[1]` and calls the appropriate `RunMode::*` handler.

## HTTP server

### Binding and ports

```bash
# Default: bind to all interfaces on port 40070
yarr serve

# Loopback only (development)
YARR_MCP_HOST=127.0.0.1 yarr serve

# Custom port
YARR_MCP_PORT=3000 yarr serve
```

### Auth modes

```bash
# No auth (loopback only, development)
YARR_MCP_HOST=127.0.0.1 YARR_MCP_NO_AUTH=true yarr serve

# Bearer token
YARR_MCP_TOKEN=$(openssl rand -hex 32) yarr serve

# OAuth (Google accounts)
YARR_MCP_AUTH_MODE=oauth \
YARR_MCP_PUBLIC_URL=https://yarr.example.com \
YARR_MCP_GOOGLE_CLIENT_ID=... \
YARR_MCP_GOOGLE_CLIENT_SECRET=... \
YARR_MCP_AUTH_ADMIN_EMAIL=you@example.com \
yarr serve
```

See `/docs/AUTH.md` for OAuth setup.

### Health and status endpoints

```bash
# Health check (always returns 200)
curl http://127.0.0.1:40070/health

# Status (requires auth)
curl -H "Authorization: Bearer $TOKEN" \
  http://127.0.0.1:40070/status
```

The `/metrics` endpoint exposes Prometheus metrics (unauthenticated).

## Docker deployment

### Quickstart

```bash
# Build image
docker build -t yarr .

# Run with config
docker run -d \
  --name yarr \
  -p 40070:40070 \
  --env-file .env \
  yarr
```

### Docker Compose

```yaml
services:
  yarr:
    image: ghcr.io/jmagar/yarr:latest
    ports:
      - "40070:40070"
    env_file:
      - .env
    volumes:
      - yarr-data:/data
    restart: unless-stopped

volumes:
  yarr-data:
```

See `/docs/DOCKER.md` for complete Docker setup.

## systemd service

### Service unit

```ini
[Unit]
Description=yarr MCP and CLI server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=yarr
Group=yarr
EnvironmentFile=/etc/yarr/env
ExecStart=/usr/local/bin/yarr serve
Restart=on-failure
RestartSec=5

# Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/yarr

[Install]
WantedBy=multi-user.target
```

### Installation

```bash
# Install binary
sudo cp bin/yarr /usr/local/bin/

# Create user
sudo useradd -r -s /bin/false yarr

# Create data directory
sudo mkdir -p /var/lib/yarr
sudo chown yarr:yarr /var/lib/yarr

# Install service
sudo cp systemd/yarr.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable yarr
sudo systemctl start yarr

# Check logs
sudo journalctl -u yarr -f
```

See `/docs/SYSTEMD.md` for details.

## Logging

yarr uses dual logging:

- **stderr** — Pretty colored output for console/containers (env-filtered by `RUST_LOG`)
- **file** — JSON-lines to `{data_dir}/logs/yarr.log` (truncated to 10MB on startup)

### Log levels

```bash
# Default: INFO
RUST_LOG=info yarr serve

# Debug logging
RUST_LOG=debug yarr serve

# Module-specific
RUST_LOG=yarr::app=debug,yarr::mcp=trace yarr serve
```

### Log location

```bash
# Default
~/.yarr/logs/yarr.log

# Custom
YARR_DATA_DIR=/var/lib/yarr yarr serve
# Logs to /var/lib/yarr/logs/yarr.log
```

See `/docs/OBSERVABILITY.md` for observability guidance.

## Upstream connectivity

yarr validates upstream connectivity during `yarr doctor` and on startup failures. Common issues:

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| "connection refused" | Wrong URL/port in config | Check `YARR_<NAME>_URL` |
| "401 Unauthorized" | Invalid API key | Check `YARR_<NAME>_API_KEY` |
| "login rejected" | Wrong qBittorrent username/password | Check `YARR_QBITTORRENT_USER/PASS` |
| timeout | Slow upstream or network issue | Increase `YARR_HTTP_TIMEOUT_SECS` |

### Doctor command

```bash
yarr doctor
```

Pre-flight checks:
- Environment variables present
- Config TOML valid
- Upstream URLs reachable
- Auth credentials accepted
- API version matches expected

See `/docs/DEPLOYMENT.md` for deployment checklist.

## Monitoring

### Metrics endpoint

```bash
curl http://127.0.0.1:40070/metrics
```

Prometheus metrics include:
- `mcp_requests_total` — MCP tool calls by action
- `mcp_request_duration_seconds` — Request latency histogram
- `upstream_requests_total` — Upstream HTTP calls by service
- `upstream_request_errors_total` — Upstream error count

### Health checks

```bash
# Simple liveness
curl http://127.0.0.1:40070/health

# Readiness (includes upstream checks)
curl -H "Authorization: Bearer $TOKEN" \
  http://127.0.0.1:40070/status
```

## Updates

### From npm package

```bash
npm update -g yarr-mcp
```

### From source

```bash
git pull
cargo build --release
sudo cp target/release/yarr /usr/local/bin/
sudo systemctl restart yarr
```

### From release binary

```bash
curl -fsSL https://raw.githubusercontent.com/jmagar/yarr/main/scripts/install.sh | bash
```

## Backup and restore

### Data to backup

- `config.toml` — Non-secret configuration
- `.env` — **DO NOT BACK UP THIS** — contains secrets; use a secrets manager instead
- `~/.yarr/snippets/` — Saved Code Mode snippets (if you use them)

### Restore

```bash
# Restore config
cp backup/config.toml /etc/yarr/

# Restore snippets
cp -r backup/snippets/* ~/.yarr/snippets/

# Re-enter secrets in .env from your secrets manager
```

## Performance

### Resource usage

Typical resource usage with 10 services:

- **Memory**: ~50-100 MB RSS (depends on Code Mode usage)
- **CPU**: Near-zero idle; spikes during Code Mode runs
- **Disk**: <10 MB for binary + logs + snippets

### Tuning

| Setting | When to adjust | How |
|---------|----------------|-----|
| `YARR_HTTP_TIMEOUT_SECS` | Upstreams >30s to respond | `YARR_HTTP_TIMEOUT_SECS=60` |
| Log truncation size | Logs grow too fast | Edit `src/logging.rs` truncation limit |
| Token budgets | Large responses truncated | Edit `src/token_limit.rs` budgets |

## Troubleshooting

### Server won't start

```bash
# Check config
yarr doctor

# Check logs
journalctl -u yarr -n 50

# Validate env
cargo xtask check-env
```

### Auth errors

```bash
# Verify token is set
echo $YARR_MCP_TOKEN

# Test with curl
curl -v -H "Authorization: Bearer $TOKEN" \
  http://127.0.0.1:40070/health
```

### Upstream timeouts

```bash
# Increase timeout
YARR_HTTP_TIMEOUT_SECS=60 yarr serve

# Check upstream health
curl http://127.0.0.1:7878/api/v3/system/status  # Radarr example
```

## CI/CD

### GitHub Actions

```yaml
name: CI
on: [push, pull_request]
jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - run: cargo xtask ci
```

See `/docs/CI.md` for CI setup.

## Further reading

- `/docs/DEPLOYMENT.md` — Deployment checklist and examples
- `/docs/DOCKER.md` — Docker setup and compose files
- `/docs/SYSTEMD.md` — systemd service units
- `/docs/OBSERVABILITY.md` — Metrics, logging, monitoring
- `/docs/ENV.md` — Complete environment variable reference
