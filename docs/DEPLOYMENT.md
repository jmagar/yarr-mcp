---
title: "Deployment"
doc_type: "guide"
status: "active"
owner: "yarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
last_reviewed: "2026-05-15"
---

# Deployment

This template supports four deployment modes:

1. **Node launcher** with `npx -y yarr-mcp mcp` or `npm i -g yarr-mcp`.
2. **Local development** with `just dev`.
3. **Docker Compose** with `just docker-up`.
4. **User systemd** with an installed release binary.

## Binary command surface

Every server binary exposes exactly two server modes and a CLI:

| Command | Mode | Description |
|---|---|---|
| `yarr mcp` | stdio MCP | For Claude Code `~/.claude/settings.json` stdio servers |
| `yarr serve` | Streamable HTTP MCP | For Docker/remote deployment |
| `yarr [subcommand]` | CLI | Direct API access; all subcommands support `--json` |
| `yarr doctor` | Pre-flight check | Validates environment and config |
| `yarr --help` | Help | Print usage |
| `yarr --version` | Version | Print version |

The npm package exposes `yarr` as the primary command and keeps `yarr` as an
alias:

```bash
npx -y yarr-mcp mcp
npm i -g yarr-mcp
yarr serve
```

## Deployment checklist

1. Build and test locally:
   ```bash
   just verify
   scripts/pre-release-check.sh
   ```
2. Create a `.env` from `.env.yarr` and set real credentials.
3. Generate a bearer token:
   ```bash
   just gen-token
   ```
4. Choose Docker or systemd.
5. Verify runtime freshness:
   ```bash
   just runtime-current
   ```
6. Smoke-test auth:
   ```bash
   YARR_MCP_TOKEN=<token> just auth-smoke
   ```
7. Run MCP integration tests:
   ```bash
   just test-mcporter
   ```

## Binary environment awareness

The binary normalizes paths, bind hosts, and ports based on its deployment context:

```rust
fn is_containerized() -> bool {
    std::path::Path::new("/.dockerenv").exists()
        || std::env::var("RUNNING_IN_CONTAINER").is_ok()
        || std::env::var("container").is_ok()
}

fn resolve_data_dir(config_path: Option<&str>) -> PathBuf {
    if let Some(p) = config_path { return PathBuf::from(p); }
    if is_containerized() { return PathBuf::from("/data"); }
    dirs::home_dir().unwrap_or_default().join(".yarr")
}

fn resolve_bind_host(configured: &str) -> &str {
    if is_containerized() { "0.0.0.0" } else { configured }
}
```

## Appdata convention

All deployments share `~/.<service>` as the logical data root:

| Deployment | Data directory |
|---|---|
| Local binary | `~/.yarr/` |
| Docker | `/data/` in container, mounted from `~/.yarr/` on host |
| Plugin | `$CLAUDE_PLUGIN_DATA` (symlinked to `~/.yarr/`) |

## Auth expectations

Non-loopback HTTP deployments must use bearer auth or OAuth. The server refuses to bind to a non-loopback address without authentication unless explicitly configured:

- Loopback bind or `YARR_MCP_NO_AUTH=true` → `LoopbackDev` (no auth)
- Non-loopback + bearer token → mounted bearer auth
- Non-loopback + `auth_mode=oauth` → mounted OAuth auth
- Non-loopback + `YARR_NOAUTH=true` → `TrustedGatewayUnscoped` (trusted gateway, explicit opt-out)
- Non-loopback + no credentials + no gateway acknowledgment → startup error

## Claude Code stdio config

```json
{
  "mcpServers": {
    "yarr": {
      "type": "stdio",
      "command": "yarr",
      "args": ["mcp"]
    }
  }
}
```

The `yarr` command must be in `$PATH` for stdio configs. Install it with
`npm i -g yarr-mcp`, or use the curl installer when npm is unavailable.

## Public endpoints

- `/health` is public and fast.
- `/status` is public but redacted.
- `/mcp` is the Streamable HTTP MCP endpoint.

## Port assignments

Each service in the rmcp family uses a fixed port to avoid collisions:

| Service | MCP Port | Binary name |
|---|---|---|
| lab | 8765 | `labby` |
| axon_rust | 8001 | `axon` |
| syslog-mcp | 3100 | `syslog` |
| unraid-mcp (unrust) | 6970 | `unraid` |
| gotify-mcp (rustify) | 9158 | `gotify` |
| unifi-mcp (rustifi) | 7474 | `unifi` |
| tailscale-mcp (rustscale) | 7575 | `tailscale` |
| apprise-mcp | 8765 | `apprise` |
| yarr | 40070 | `yarr` |

Set the port via `YARR_MCP_PORT` or in `config.toml`. Update `EXPOSE` in the Dockerfile and the port mapping in `docker-compose.yml` to match.

## Worktree file propagation

Claude Code worktrees are fresh checkouts — gitignored files like `.env` and `config.toml` are absent by default. The `.worktreeinclude` file at the repo root tells Claude Code which gitignored files to copy into each new worktree automatically:

```
# .worktreeinclude
.env
config.toml
```

This ensures the server can start in a worktree without manual setup. Both files are one-way copied (main → worktree) at worktree creation time only.

`.gitignore` additions required alongside `.worktreeinclude`:

```gitignore
config.toml
.beagle/
```

See `docs/DOCKER.md`, `docs/SYSTEMD.md`, `docs/ENV.md`, and `docs/CONFIG.md` for deployment-specific details. See `docs/PATTERNS.md` §19, §27, §28, §46, §47, §A6 for port assignments, security, environment awareness, binary installation, and worktree patterns.
