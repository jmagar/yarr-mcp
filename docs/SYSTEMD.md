---
title: "systemd Deployment"
doc_type: "guide"
status: "active"
owner: "rustarr"
audience:
  - "contributors"
  - "agents"
scope: "template"
source_of_truth: false
last_reviewed: "2026-05-15"
---

# systemd

The template supports user-level systemd deployments when a unit named `rustarr-mcp.service` is installed by the derived service.

## Install the binary

```bash
curl -fsSL https://raw.githubusercontent.com/jmagar/rustarr-mcp/main/scripts/install.sh | bash
```

Or install through npm:

```bash
npm i -g yarr-mcp
```

The npm package installs `yarr` and `rustarr` shims. The curl installer writes
`rustarr` to `~/.local/bin/` and creates a `yarr` symlink. Verify the command is
in `$PATH`:

```bash
yarr --version
yarr doctor
```

## Unit file pattern

```ini
[Unit]
Description=rustarr MCP server
After=network.target

[Service]
Type=simple
ExecStart=%h/.local/bin/yarr serve mcp
Restart=on-failure
RestartSec=5
EnvironmentFile=%h/.rustarr/.env

[Install]
WantedBy=default.target
```

Key points:
- The unit example assumes the curl installer. If you use `npm i -g yarr-mcp`, set
  `ExecStart` to the absolute path returned by `command -v yarr`.
- Use `EnvironmentFile` pointing at `~/.rustarr/.env` — never hardcode tokens in unit files.
- `%h` expands to the user home directory.
- `serve mcp` is the canonical Streamable HTTP mode (see `docs/DEPLOYMENT.md`).

## Restart flow

```bash
systemctl --user daemon-reload
systemctl --user restart rustarr-mcp.service
systemctl --user status rustarr-mcp.service
```

## Runtime verification

`just runtime-current` detects stale running processes. The checker compares:

- `/proc/<pid>/exe` for the running service process
- the unit `ExecStart` binary
- optional `--expected-binary`

```bash
scripts/check-runtime-current.sh --mode systemd --expected-binary target/release/rustarr
just runtime-current
```

If hashes differ, install the new binary and restart the unit.

## Logging

With systemd, logs go to the journal:

```bash
journalctl --user -u rustarr-mcp.service -f
journalctl --user -u rustarr-mcp.service --since "1h ago"
```

The binary also writes structured JSON logs to `~/.rustarr/logs/rustarr.log` regardless of deployment mode (see `docs/OBSERVABILITY.md`).

## Doctor pre-flight

Run `rustarr doctor` before starting the unit to validate the environment:

```bash
rustarr doctor
```

Exit code 0 = ready to start. Exit code 1 = one or more issues found.

## Environment

Prefer an `EnvironmentFile` that points at the repo or appdata `.env`. See `docs/ENV.md` for variable meanings.

See `docs/PATTERNS.md` §46, §47, §48 for binary commands, installation, and the doctor command patterns.
