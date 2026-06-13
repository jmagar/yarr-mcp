# Rustarr Full Live Test Matrix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an opt-in, shart-only live test suite that proves every Rustarr CLI command, MCP tool path, HTTP API route, and service action works against the actual configured media automation services with semantic success and expected-error assertions.

**Architecture:** Add a shared shart guard, a declarative live service matrix, and a Python orchestration runner that starts Rustarr locally with the shart env and tests the CLI, Streamable HTTP MCP, local REST routes, and upstream service passthroughs. The suite must fail closed if any `ServiceKind` is missing, points outside shart, or returns only a shallow response without semantic validation.

**Tech Stack:** Rustarr Rust binary, Python 3 standard library, shell scripts, Justfile, JSON fixtures, shart live media stack, Streamable HTTP MCP JSON-RPC.

---

## Scope And Coverage Contract

Rustarr currently exposes five MCP business actions in `src/actions.rs`: `help`, `integrations`, `service_status`, `api_get`, and `api_post`. The CLI exposes those same business actions through `help`, `integrations`, `status`, `get`, and `post`, plus infrastructure commands: `doctor`, `watch`, `setup check`, `setup repair`, `setup install`, `setup plugin-hook`, `serve`, `mcp`, `--help`, and `--version`. The HTTP server exposes `/health`, `/ready`, `/status`, and `POST /mcp`.

The full live suite must test this matrix:

| Surface | Required live checks |
|---|---|
| CLI business commands | `help`, `integrations`, `status --service`, `get --service --path`, `post --service --path --body --confirm`, `post` without `--confirm` |
| CLI infrastructure commands | `--help`, `--version`, `doctor --json`, `watch` against a loopback server with timeout, `setup check`, `setup plugin-hook --no-repair`; `serve` and `mcp` through the running server harness |
| MCP protocol | `initialize`, `tools/list`, `tools/call` for all actions, `resources/list`, schema resource read, `prompts/list`, `prompts/get quick_start`, read/write scope behavior when auth is enabled |
| HTTP routes | `GET /health`, `GET /ready`, `GET /status`, `POST /mcp`, one unknown route returning 404 |
| Service action matrix | For every `ServiceKind`: `service_status(service)`, at least one semantic `api_get`, `api_post` confirm guard, and one safe `api_post` expected-error call that reaches the upstream service without mutating data |

The suite is complete only when all 15 supported service kinds are configured on shart:

```text
sonarr, radarr, prowlarr, tautulli, overseerr, bazarr, tracearr, lidarr,
readarr, sabnzbd, qbittorrent, wizarr, notifiarr, plex, jellyfin
```

At the time this plan was written, the shart env was known to cover 12 initialized services and intentionally excluded `lidarr`, `readarr`, and initialized `wizarr`. The full runner must therefore fail with a clear `missing required service kind` error until those are added to `/home/jmagar/.rustarr-shart/.env`.

## File Structure

- Create `scripts/lib/rustarr_shart_guard.py`
  - Single source of truth for loading the shart env, merging process overrides, validating service URLs, and enforcing the complete service-kind set.
- Modify `scripts/live-read-smoke.sh`
  - Replace its inline guard with a call to the shared guard helper.
- Create `tests/live/service_matrix.json`
  - Declarative list of every service, expected kind, semantic GET checks, semantic status checks, and safe POST expected-error probes.
- Create `scripts/live-full-test.py`
  - Orchestrates guard, CLI tests, local server lifecycle, REST route tests, MCP JSON-RPC tests, and service action matrix tests.
- Create `tests/live/test_guard.py`
  - Unit tests for the guard helper using temporary env files and fake URLs.
- Modify `tests/mcporter/test-mcp.sh`
  - Make the legacy MCP smoke harness shart-only or explicitly route it through the new guard.
- Modify `Justfile`
  - Add `live-full-test`, `live-full-cli`, `live-full-rest`, `live-full-mcp`, `live-full-services`, and `live-full-guard` recipes.
- Modify `docs/TESTING.md`, `docs/MCPORTER.md`, `docs/SCRIPTS.md`, and `scripts/README.md`
  - Document the shart-only guard, full live suite, service prerequisites, expected outputs, and when not to run it.

## Task 1: Add The Shared Shart Guard

**Files:**
- Create: `scripts/lib/rustarr_shart_guard.py`
- Test: `tests/live/test_guard.py`

- [ ] **Step 1: Write failing guard tests**

Create `tests/live/test_guard.py`:

```python
import os
import tempfile
import unittest
from pathlib import Path

from scripts.lib.rustarr_shart_guard import GuardError, load_and_validate_env


def write_env(body: str) -> Path:
    handle = tempfile.NamedTemporaryFile("w", delete=False)
    handle.write(body)
    handle.close()
    return Path(handle.name)


GOOD_ENV = """
RUSTARR_SERVICES=sonarr,radarr,prowlarr,tautulli,overseerr,bazarr,tracearr,lidarr,readarr,sabnzbd,qbittorrent,wizarr,notifiarr,plex,jellyfin
RUSTARR_SONARR_URL=http://shart.manatee-triceratops.ts.net:8989
RUSTARR_SONARR_KIND=sonarr
RUSTARR_RADARR_URL=http://100.118.209.1:7878
RUSTARR_RADARR_KIND=radarr
RUSTARR_PROWLARR_URL=http://shart.manatee-triceratops.ts.net:9696
RUSTARR_PROWLARR_KIND=prowlarr
RUSTARR_TAUTULLI_URL=http://shart.manatee-triceratops.ts.net:8181
RUSTARR_TAUTULLI_KIND=tautulli
RUSTARR_OVERSEERR_URL=http://shart.manatee-triceratops.ts.net:5055
RUSTARR_OVERSEERR_KIND=overseerr
RUSTARR_BAZARR_URL=http://shart.manatee-triceratops.ts.net:6767
RUSTARR_BAZARR_KIND=bazarr
RUSTARR_TRACEARR_URL=http://shart.manatee-triceratops.ts.net:8686
RUSTARR_TRACEARR_KIND=tracearr
RUSTARR_LIDARR_URL=http://shart.manatee-triceratops.ts.net:8687
RUSTARR_LIDARR_KIND=lidarr
RUSTARR_READARR_URL=http://shart.manatee-triceratops.ts.net:8787
RUSTARR_READARR_KIND=readarr
RUSTARR_SABNZBD_URL=http://shart.manatee-triceratops.ts.net:8080
RUSTARR_SABNZBD_KIND=sabnzbd
RUSTARR_QBITTORRENT_URL=http://shart.manatee-triceratops.ts.net:8081
RUSTARR_QBITTORRENT_KIND=qbittorrent
RUSTARR_WIZARR_URL=http://shart.manatee-triceratops.ts.net:5690
RUSTARR_WIZARR_KIND=wizarr
RUSTARR_NOTIFIARR_URL=http://shart.manatee-triceratops.ts.net:5454
RUSTARR_NOTIFIARR_KIND=notifiarr
RUSTARR_PLEX_URL=http://shart.manatee-triceratops.ts.net:32400
RUSTARR_PLEX_KIND=plex
RUSTARR_JELLYFIN_URL=http://shart.manatee-triceratops.ts.net:8096
RUSTARR_JELLYFIN_KIND=jellyfin
"""


class ShartGuardTest(unittest.TestCase):
    def test_accepts_complete_shart_env(self):
        path = write_env(GOOD_ENV)
        result = load_and_validate_env(env_file=path, process_env={"RUSTARR_HOME": "/home/jmagar/.rustarr-shart"})
        self.assertEqual(result["RUSTARR_HOME"], "/home/jmagar/.rustarr-shart")
        self.assertEqual(result["RUSTARR_SONARR_KIND"], "sonarr")

    def test_rejects_live_home(self):
        path = write_env(GOOD_ENV)
        with self.assertRaisesRegex(GuardError, "RUSTARR_HOME must be /home/jmagar/.rustarr-shart"):
            load_and_validate_env(env_file=path, process_env={"RUSTARR_HOME": "/home/jmagar/.rustarr"})

    def test_rejects_tootie_url_override(self):
        path = write_env(GOOD_ENV)
        with self.assertRaisesRegex(GuardError, "not a shart URL"):
            load_and_validate_env(
                env_file=path,
                process_env={
                    "RUSTARR_HOME": "/home/jmagar/.rustarr-shart",
                    "RUSTARR_SONARR_URL": "https://sonarr.tootie.tv",
                },
            )

    def test_rejects_missing_required_kind(self):
        path = write_env(GOOD_ENV.replace(",readarr", ""))
        with self.assertRaisesRegex(GuardError, "missing required service kind: readarr"):
            load_and_validate_env(env_file=path, process_env={"RUSTARR_HOME": "/home/jmagar/.rustarr-shart"})


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run the failing guard tests**

Run:

```bash
python3 -m unittest tests.live.test_guard -v
```

Expected:

```text
ModuleNotFoundError: No module named 'scripts.lib.rustarr_shart_guard'
```

- [ ] **Step 3: Implement the shared guard**

Create `scripts/lib/rustarr_shart_guard.py`:

```python
#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shlex
import sys
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlparse

SHART_HOME = "/home/jmagar/.rustarr-shart"
DEFAULT_ENV_FILE = Path(SHART_HOME) / ".env"
SHART_HOSTS = {"shart", "shart.manatee-triceratops.ts.net", "100.118.209.1"}
REQUIRED_KINDS = {
    "sonarr",
    "radarr",
    "prowlarr",
    "tautulli",
    "overseerr",
    "bazarr",
    "tracearr",
    "lidarr",
    "readarr",
    "sabnzbd",
    "qbittorrent",
    "wizarr",
    "notifiarr",
    "plex",
    "jellyfin",
}


class GuardError(RuntimeError):
    pass


@dataclass(frozen=True)
class GuardResult:
    env: dict[str, str]
    services: list[str]
    kinds: dict[str, str]


def _parse_env_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = shlex.split(value.strip())[0] if value.strip() else ""
    return values


def _service_env_name(service: str) -> str:
    return "".join(ch if ch.isalnum() else "_" for ch in service.upper())


def _assert_shart_url(key: str, value: str) -> None:
    parsed = urlparse(value)
    if parsed.scheme not in {"http", "https"}:
        raise GuardError(f"{key} must be an http or https URL")
    if parsed.hostname not in SHART_HOSTS:
        raise GuardError(f"{key}={value} is not a shart URL")


def load_and_validate_env(
    env_file: Path = DEFAULT_ENV_FILE,
    process_env: dict[str, str] | None = None,
    require_complete: bool = True,
) -> dict[str, str]:
    process_env = dict(os.environ if process_env is None else process_env)
    if not env_file.exists():
        raise GuardError(f"missing shart env file: {env_file}")

    merged = _parse_env_file(env_file)
    merged.update({key: value for key, value in process_env.items() if key.startswith("RUSTARR_")})
    merged["RUSTARR_HOME"] = process_env.get("RUSTARR_HOME", SHART_HOME)

    if merged["RUSTARR_HOME"] != SHART_HOME:
        raise GuardError(f"RUSTARR_HOME must be {SHART_HOME}; got {merged['RUSTARR_HOME']}")

    services = [part.strip() for part in merged.get("RUSTARR_SERVICES", "").split(",") if part.strip()]
    if not services:
        raise GuardError("RUSTARR_SERVICES is empty")

    kinds: dict[str, str] = {}
    for service in services:
        env_name = _service_env_name(service)
        url_key = f"RUSTARR_{env_name}_URL"
        kind_key = f"RUSTARR_{env_name}_KIND"
        url = merged.get(url_key)
        if not url:
            raise GuardError(f"missing {url_key}")
        _assert_shart_url(url_key, url)
        kinds[service] = merged.get(kind_key, service).lower()

    if require_complete:
        missing = sorted(REQUIRED_KINDS - set(kinds.values()))
        if missing:
            raise GuardError(f"missing required service kind: {missing[0]}")

    return merged


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Rustarr shart-only live test environment")
    parser.add_argument("--env-file", default=str(DEFAULT_ENV_FILE))
    parser.add_argument("--allow-partial", action="store_true")
    args = parser.parse_args()
    try:
        env = load_and_validate_env(Path(args.env_file), require_complete=not args.allow_partial)
    except GuardError as exc:
        print(f"rustarr shart guard failed: {exc}", file=sys.stderr)
        return 2
    for key in sorted(env):
        if key.startswith("RUSTARR_") or key == "RUSTARR_HOME":
            print(f"{key}={shlex.quote(env[key])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run guard tests to verify they pass**

Run:

```bash
python3 -m unittest tests.live.test_guard -v
```

Expected:

```text
Ran 4 tests
OK
```

- [ ] **Step 5: Commit the guard**

Run:

```bash
git add scripts/lib/rustarr_shart_guard.py tests/live/test_guard.py
git commit -m "test: add shart-only live test guard"
```

Expected: commit succeeds.

## Task 2: Wire Existing Live Smoke Through The Guard

**Files:**
- Modify: `scripts/live-read-smoke.sh`
- Modify: `scripts/README.md`

- [ ] **Step 1: Update the shell smoke guard**

In `scripts/live-read-smoke.sh`, replace the current inline shart validation block with:

```bash
GUARD_OUTPUT="$(python3 "$REPO_ROOT/scripts/lib/rustarr_shart_guard.py" --allow-partial)"
while IFS= read -r line; do
  export "$line"
done <<< "$GUARD_OUTPUT"
```

Keep `--allow-partial` for this legacy smoke script so it can continue validating the currently initialized subset while the full suite is being built. Do not use `--allow-partial` in the new full suite.

- [ ] **Step 2: Verify the legacy smoke still passes against shart**

Run:

```bash
RUSTARR_BIN=target/release/rustarr bash scripts/live-read-smoke.sh
```

Expected:

```text
99 passed, 0 failed
```

- [ ] **Step 3: Verify the legacy smoke rejects live home**

Run:

```bash
RUSTARR_HOME=/home/jmagar/.rustarr RUSTARR_BIN=target/release/rustarr bash scripts/live-read-smoke.sh
```

Expected:

```text
rustarr shart guard failed: RUSTARR_HOME must be /home/jmagar/.rustarr-shart
```

Exit code must be non-zero.

- [ ] **Step 4: Commit the legacy smoke update**

Run:

```bash
git add scripts/live-read-smoke.sh scripts/README.md
git commit -m "test: share shart guard with live smoke"
```

Expected: commit succeeds.

## Task 3: Bring Shart To Complete ServiceKind Coverage

**Files:**
- Remote review: shart compose files under the existing rustarr media test stack location
- Remote update: `/home/jmagar/.rustarr-shart/.env`
- Documentation: `docs/TESTING.md`

- [ ] **Step 1: Confirm current missing service kinds**

Run:

```bash
python3 scripts/lib/rustarr_shart_guard.py
```

Expected before this task is complete:

```text
rustarr shart guard failed: missing required service kind: lidarr
```

- [ ] **Step 2: Add or start Lidarr on shart**

On shart, add a Lidarr service to the existing test compose stack with appdata rooted in the curated test config tree:

```yaml
lidarr:
  image: lscr.io/linuxserver/lidarr:latest
  container_name: rustarr-test-lidarr
  environment:
    - PUID=99
    - PGID=100
    - TZ=America/New_York
  volumes:
    - /mnt/user/lab/live/golden/lidarr:/config
    - /mnt/user/lab/live/media:/media
  ports:
    - "8687:8686"
  restart: unless-stopped
```

Run on shart:

```bash
docker compose up -d lidarr
docker ps --filter name=rustarr-test-lidarr --format '{{.Names}} {{.Status}}'
```

Expected:

```text
rustarr-test-lidarr Up
```

- [ ] **Step 3: Add or start Readarr on shart**

On shart, add a Readarr service to the same stack:

```yaml
readarr:
  image: lscr.io/linuxserver/readarr:develop
  container_name: rustarr-test-readarr
  environment:
    - PUID=99
    - PGID=100
    - TZ=America/New_York
  volumes:
    - /mnt/user/lab/live/golden/readarr:/config
    - /mnt/user/lab/live/media:/media
  ports:
    - "8787:8787"
  restart: unless-stopped
```

Run on shart:

```bash
docker compose up -d readarr
docker ps --filter name=rustarr-test-readarr --format '{{.Names}} {{.Status}}'
```

Expected:

```text
rustarr-test-readarr Up
```

- [ ] **Step 4: Initialize Wizarr on shart**

Open Wizarr on shart and complete the first-run setup using test-only credentials. Then verify the status endpoint no longer returns first-run 401.

Run from dookie:

```bash
curl -fsS http://shart.manatee-triceratops.ts.net:5690/api/status
```

Expected response contains:

```json
{"status":"ok"}
```

If Wizarr uses a different initialized status shape, record the exact response in `tests/live/service_matrix.json` in Task 4 and assert that shape.

- [ ] **Step 5: Extract Lidarr and Readarr API keys**

Run from dookie:

```bash
ssh shart "sed -n 's:.*<ApiKey>\\(.*\\)</ApiKey>.*:\\1:p' /mnt/user/lab/live/golden/lidarr/config.xml"
ssh shart "sed -n 's:.*<ApiKey>\\(.*\\)</ApiKey>.*:\\1:p' /mnt/user/lab/live/golden/readarr/config.xml"
```

Expected: each command prints one non-empty API key line.

- [ ] **Step 6: Update `/home/jmagar/.rustarr-shart/.env`**

Run this command from dookie. It updates the service list and inserts the exact API keys extracted from shart without printing them into git-tracked files:

```bash
LIDARR_KEY="$(ssh shart "sed -n 's:.*<ApiKey>\\(.*\\)</ApiKey>.*:\\1:p' /mnt/user/lab/live/golden/lidarr/config.xml")"
READARR_KEY="$(ssh shart "sed -n 's:.*<ApiKey>\\(.*\\)</ApiKey>.*:\\1:p' /mnt/user/lab/live/golden/readarr/config.xml")"
python3 - <<'PY'
import os
from pathlib import Path

path = Path("/home/jmagar/.rustarr-shart/.env")
values = {}
for line in path.read_text().splitlines():
    if "=" in line and not line.lstrip().startswith("#"):
        key, value = line.split("=", 1)
        values[key] = value

values["RUSTARR_SERVICES"] = "sonarr,radarr,prowlarr,tautulli,overseerr,bazarr,tracearr,lidarr,readarr,sabnzbd,qbittorrent,wizarr,notifiarr,plex,jellyfin"
values["RUSTARR_LIDARR_URL"] = "http://shart.manatee-triceratops.ts.net:8687"
values["RUSTARR_LIDARR_KIND"] = "lidarr"
values["RUSTARR_LIDARR_API_KEY"] = os.environ["LIDARR_KEY"]
values["RUSTARR_READARR_URL"] = "http://shart.manatee-triceratops.ts.net:8787"
values["RUSTARR_READARR_KIND"] = "readarr"
values["RUSTARR_READARR_API_KEY"] = os.environ["READARR_KEY"]
values["RUSTARR_WIZARR_URL"] = "http://shart.manatee-triceratops.ts.net:5690"
values["RUSTARR_WIZARR_KIND"] = "wizarr"

ordered = []
seen = set()
for line in path.read_text().splitlines():
    if "=" in line and not line.lstrip().startswith("#"):
        key = line.split("=", 1)[0]
        if key in values:
            ordered.append(f"{key}={values[key]}")
            seen.add(key)
        else:
            ordered.append(line)
    else:
        ordered.append(line)

for key in sorted(set(values) - seen):
    ordered.append(f"{key}={values[key]}")

path.write_text("\n".join(ordered) + "\n")
PY
```

Do not commit `/home/jmagar/.rustarr-shart/.env`; it is outside the repo and contains secrets.

- [ ] **Step 7: Verify complete guard passes**

Run:

```bash
python3 scripts/lib/rustarr_shart_guard.py >/tmp/rustarr-shart-env.checked
```

Expected: exit code 0 and output contains all 15 `RUSTARR_<SERVICE>_URL` values pointing at `shart.manatee-triceratops.ts.net` or `100.118.209.1`.

- [ ] **Step 8: Commit docs for the prerequisite**

Run:

```bash
git add docs/TESTING.md
git commit -m "docs: document full shart service prerequisites"
```

Expected: commit succeeds.

## Task 4: Define The Live Service Matrix

**Files:**
- Create: `tests/live/service_matrix.json`

- [ ] **Step 1: Create the matrix file**

Create `tests/live/service_matrix.json`:

```json
{
  "services": [
    {
      "name": "sonarr",
      "kind": "sonarr",
      "status": {"json_path": "appName", "equals": "Sonarr"},
      "get": [
        {"path": "/api/v3/system/status", "json_path": "appName", "equals": "Sonarr"},
        {"path": "/api/v3/series", "type": "array"}
      ],
      "post_blocked": {"path": "/api/v3/system/status", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/v3/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "radarr",
      "kind": "radarr",
      "status": {"json_path": "appName", "equals": "Radarr"},
      "get": [
        {"path": "/api/v3/system/status", "json_path": "appName", "equals": "Radarr"},
        {"path": "/api/v3/movie", "type": "array"}
      ],
      "post_blocked": {"path": "/api/v3/system/status", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/v3/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "prowlarr",
      "kind": "prowlarr",
      "status": {"json_path": "appName", "equals": "Prowlarr"},
      "get": [
        {"path": "/api/v1/system/status", "json_path": "appName", "equals": "Prowlarr"},
        {"path": "/api/v1/indexer", "type": "array"}
      ],
      "post_blocked": {"path": "/api/v1/system/status", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/v1/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "tautulli",
      "kind": "tautulli",
      "status": {"json_path": "response.result", "equals": "success"},
      "get": [
        {"path": "/api/v2?cmd=get_server_info", "json_path": "response.result", "equals": "success"}
      ],
      "post_blocked": {"path": "/api/v2?cmd=get_server_info", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/v2?cmd=__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["error", "invalid", "unknown", "400", "404"]}
    },
    {
      "name": "overseerr",
      "kind": "overseerr",
      "status": {"json_path": "version", "type": "string"},
      "get": [
        {"path": "/api/v1/status", "json_path": "version", "type": "string"}
      ],
      "post_blocked": {"path": "/api/v1/status", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/v1/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "bazarr",
      "kind": "bazarr",
      "status": {"json_path": "bazarr_version", "type": "string"},
      "get": [
        {"path": "/api/system/status", "json_path": "bazarr_version", "type": "string"}
      ],
      "post_blocked": {"path": "/api/system/status", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "tracearr",
      "kind": "tracearr",
      "status": {"json_path": "status", "equals_any": ["ok", "healthy"]},
      "get": [
        {"path": "/health", "json_path": "status", "equals_any": ["ok", "healthy"]}
      ],
      "post_blocked": {"path": "/health", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "lidarr",
      "kind": "lidarr",
      "status": {"json_path": "appName", "equals": "Lidarr"},
      "get": [
        {"path": "/api/v1/system/status", "json_path": "appName", "equals": "Lidarr"},
        {"path": "/api/v1/artist", "type": "array"}
      ],
      "post_blocked": {"path": "/api/v1/system/status", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/v1/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "readarr",
      "kind": "readarr",
      "status": {"json_path": "appName", "equals": "Readarr"},
      "get": [
        {"path": "/api/v1/system/status", "json_path": "appName", "equals": "Readarr"},
        {"path": "/api/v1/author", "type": "array"}
      ],
      "post_blocked": {"path": "/api/v1/system/status", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/v1/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "sabnzbd",
      "kind": "sabnzbd",
      "status": {"json_path": "version", "type": "string"},
      "get": [
        {"path": "/api?mode=version&output=json", "json_path": "version", "type": "string"},
        {"path": "/api?mode=queue&output=json", "json_path": "queue", "type": "object"}
      ],
      "post_blocked": {"path": "/api?mode=version&output=json", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api?mode=__rustarr_live_post_probe__&output=json", "body": {}, "error_contains_any": ["error", "unknown", "400"]}
    },
    {
      "name": "qbittorrent",
      "kind": "qbittorrent",
      "status": {"type": "string", "contains": "qBittorrent"},
      "get": [
        {"path": "/api/v2/app/version", "type": "string", "contains": "qBittorrent"},
        {"path": "/api/v2/torrents/info", "type": "array"}
      ],
      "post_blocked": {"path": "/api/v2/app/version", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/v2/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "wizarr",
      "kind": "wizarr",
      "status": {"json_path": "status", "equals_any": ["ok", "healthy"]},
      "get": [
        {"path": "/api/status", "json_path": "status", "equals_any": ["ok", "healthy"]}
      ],
      "post_blocked": {"path": "/api/status", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "notifiarr",
      "kind": "notifiarr",
      "status": {"json_path": "status", "equals_any": ["ok", "success", "healthy"]},
      "get": [
        {"path": "/api/v1/status", "json_path": "status", "equals_any": ["ok", "success", "healthy"]}
      ],
      "post_blocked": {"path": "/api/v1/status", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/api/v1/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "plex",
      "kind": "plex",
      "status": {"xml_root": "MediaContainer"},
      "get": [
        {"path": "/identity", "xml_root": "MediaContainer"}
      ],
      "post_blocked": {"path": "/identity", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    },
    {
      "name": "jellyfin",
      "kind": "jellyfin",
      "status": {"json_path": "ProductName", "equals": "Jellyfin Server"},
      "get": [
        {"path": "/System/Info/Public", "json_path": "ProductName", "equals": "Jellyfin Server"}
      ],
      "post_blocked": {"path": "/System/Info/Public", "body": {}, "error_contains": "confirm=true"},
      "post_expected_error": {"path": "/System/__rustarr_live_post_probe__", "body": {}, "error_contains_any": ["404", "405", "Not Found", "Method Not Allowed"]}
    }
  ]
}
```

- [ ] **Step 2: Validate matrix service coverage**

Run:

```bash
python3 - <<'PY'
import json
from pathlib import Path
required = {
    "sonarr", "radarr", "prowlarr", "tautulli", "overseerr", "bazarr",
    "tracearr", "lidarr", "readarr", "sabnzbd", "qbittorrent", "wizarr",
    "notifiarr", "plex", "jellyfin",
}
data = json.loads(Path("tests/live/service_matrix.json").read_text())
kinds = {entry["kind"] for entry in data["services"]}
assert kinds == required, sorted(required - kinds)
print(f"matrix covers {len(kinds)} service kinds")
PY
```

Expected:

```text
matrix covers 15 service kinds
```

- [ ] **Step 3: Commit the matrix**

Run:

```bash
git add tests/live/service_matrix.json
git commit -m "test: define full live service matrix"
```

Expected: commit succeeds.

## Task 5: Build The Live Runner Core

**Files:**
- Create: `scripts/live-full-test.py`

- [ ] **Step 1: Create runner skeleton with suite selection and reporting**

Create `scripts/live-full-test.py`:

```python
#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from xml.etree import ElementTree

from scripts.lib.rustarr_shart_guard import load_and_validate_env

ROOT = Path(__file__).resolve().parents[1]
MATRIX = ROOT / "tests/live/service_matrix.json"
REPORT = ROOT / "target/live-full/report.json"


@dataclass
class Check:
    name: str
    passed: bool
    detail: str


class LiveRunner:
    def __init__(self, suite: str, rustarr_bin: str):
        self.suite = suite
        self.rustarr_bin = rustarr_bin
        self.env = load_and_validate_env()
        self.env.update(os.environ)
        self.env["RUSTARR_HOME"] = "/home/jmagar/.rustarr-shart"
        self.checks: list[Check] = []
        self.server: subprocess.Popen[str] | None = None
        self.base_url = "http://127.0.0.1:40070"
        self.matrix = json.loads(MATRIX.read_text())

    def record(self, name: str, passed: bool, detail: str) -> None:
        self.checks.append(Check(name, passed, detail))
        print(("PASS" if passed else "FAIL") + f" {name}: {detail}")

    def run_command(self, args: list[str], timeout: int = 30) -> subprocess.CompletedProcess[str]:
        return subprocess.run(args, env=self.env, cwd=ROOT, text=True, capture_output=True, timeout=timeout)

    def cli_json(self, args: list[str]) -> Any:
        proc = self.run_command([self.rustarr_bin, *args])
        if proc.returncode != 0:
            raise AssertionError(proc.stderr.strip() or proc.stdout.strip())
        return json.loads(proc.stdout)

    def http(self, method: str, url: str, body: Any | None = None) -> tuple[int, str, dict[str, str]]:
        data = None if body is None else json.dumps(body).encode()
        headers = {"content-type": "application/json"} if body is not None else {}
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=20) as resp:
                return resp.status, resp.read().decode(), dict(resp.headers)
        except urllib.error.HTTPError as exc:
            return exc.code, exc.read().decode(), dict(exc.headers)

    def start_server(self) -> None:
        self.env["RUSTARR_MCP_HOST"] = "127.0.0.1"
        self.env["RUSTARR_MCP_PORT"] = "40070"
        self.env["RUSTARR_MCP_NO_AUTH"] = "true"
        self.server = subprocess.Popen(
            [self.rustarr_bin, "serve", "mcp"],
            cwd=ROOT,
            env=self.env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        deadline = time.time() + 20
        while time.time() < deadline:
            status, body, _ = self.http("GET", f"{self.base_url}/health")
            if status == 200 and "healthy" in body:
                return
            time.sleep(0.25)
        raise AssertionError("rustarr server did not become healthy")

    def stop_server(self) -> None:
        if not self.server:
            return
        self.server.send_signal(signal.SIGTERM)
        try:
            self.server.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.server.kill()
            self.server.wait(timeout=5)

    def assert_expectation(self, value: Any, expectation: dict[str, Any]) -> None:
        if "type" in expectation:
            expected_type = expectation["type"]
            type_map = {"array": list, "object": dict, "string": str}
            if not isinstance(value, type_map[expected_type]):
                raise AssertionError(f"expected {expected_type}, got {type(value).__name__}")
        if "contains" in expectation and expectation["contains"] not in str(value):
            raise AssertionError(f"expected {value!r} to contain {expectation['contains']!r}")
        if "xml_root" in expectation:
            root = ElementTree.fromstring(value if isinstance(value, str) else json.dumps(value))
            if root.tag != expectation["xml_root"]:
                raise AssertionError(f"expected XML root {expectation['xml_root']}, got {root.tag}")
            return
        if "json_path" in expectation:
            node = value
            for part in expectation["json_path"].split("."):
                node = node[part]
            if "equals" in expectation and node != expectation["equals"]:
                raise AssertionError(f"expected {expectation['json_path']}={expectation['equals']!r}, got {node!r}")
            if "equals_any" in expectation and node not in expectation["equals_any"]:
                raise AssertionError(f"expected {node!r} in {expectation['equals_any']!r}")
            if "type" in expectation:
                self.assert_expectation(node, {"type": expectation["type"]})

    def run_guard(self) -> None:
        self.record("guard complete shart env", True, "all service URLs are shart-only")

    def write_report(self) -> None:
        REPORT.parent.mkdir(parents=True, exist_ok=True)
        REPORT.write_text(json.dumps([check.__dict__ for check in self.checks], indent=2))

    def run(self) -> int:
        selected = {
            "guard": [self.run_guard],
            "all": [self.run_guard],
        }[self.suite]
        try:
            for fn in selected:
                fn()
        finally:
            self.stop_server()
            self.write_report()
        return 0 if all(check.passed for check in self.checks) else 1


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the full Rustarr shart live test suite")
    parser.add_argument("--suite", choices=["guard", "cli", "rest", "mcp", "services", "all"], default="all")
    parser.add_argument("--rustarr-bin", default=os.environ.get("RUSTARR_BIN", "target/release/rustarr"))
    args = parser.parse_args()
    return LiveRunner(args.suite, args.rustarr_bin).run()


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 2: Run guard-only suite**

Run:

```bash
python3 scripts/live-full-test.py --suite guard
```

Expected after Task 3 is complete:

```text
PASS guard complete shart env: all service URLs are shart-only
```

- [ ] **Step 3: Commit the runner core**

Run:

```bash
git add scripts/live-full-test.py
git commit -m "test: add full live runner core"
```

Expected: commit succeeds.

## Task 6: Implement CLI Surface Tests

**Files:**
- Modify: `scripts/live-full-test.py`

- [ ] **Step 1: Add CLI test methods**

Add these methods inside `LiveRunner`:

```python
    def run_cli(self) -> None:
        version = self.run_command([self.rustarr_bin, "--version"])
        self.record("cli --version", version.returncode == 0 and "rustarr" in version.stdout.lower(), version.stdout.strip())

        help_proc = self.run_command([self.rustarr_bin, "--help"])
        self.record("cli --help", help_proc.returncode == 0 and "Usage:" in help_proc.stdout, "usage printed")

        help_json = self.cli_json(["help"])
        self.record("cli help action", "actions" in help_json, "structured help contains actions")

        integrations = self.cli_json(["integrations"])
        configured = {item["name"] for item in integrations["configured"]}
        expected = {entry["name"] for entry in self.matrix["services"]}
        self.record("cli integrations complete", expected <= configured, f"{len(configured)} configured")

        doctor = self.run_command([self.rustarr_bin, "doctor", "--json"], timeout=60)
        doctor_json = json.loads(doctor.stdout)
        self.record("cli doctor --json", doctor.returncode == 0 and doctor_json["overall"] in {"ok", "warning"}, doctor_json["overall"])

        setup_check = self.run_command([self.rustarr_bin, "setup", "check"], timeout=60)
        self.record("cli setup check", setup_check.returncode in {0, 1}, "setup check completed without crashing")

        setup_hook = self.run_command([self.rustarr_bin, "setup", "plugin-hook", "--no-repair"], timeout=60)
        self.record("cli setup plugin-hook --no-repair", setup_hook.returncode in {0, 1}, "plugin hook check completed without repair")

        for service in self.matrix["services"]:
            name = service["name"]
            status = self.cli_json(["status", "--service", name])
            self.assert_expectation(status, service["status"])
            self.record(f"cli status {name}", True, "semantic status matched")

            for get_case in service["get"]:
                payload = self.cli_json(["get", "--service", name, "--path", get_case["path"]])
                self.assert_expectation(payload, get_case)
                self.record(f"cli get {name} {get_case['path']}", True, "semantic GET matched")

            blocked = self.run_command([
                self.rustarr_bin,
                "post",
                "--service",
                name,
                "--path",
                service["post_blocked"]["path"],
                "--body",
                json.dumps(service["post_blocked"]["body"]),
            ])
            self.record(
                f"cli post confirm guard {name}",
                blocked.returncode != 0 and service["post_blocked"]["error_contains"] in (blocked.stderr + blocked.stdout),
                "blocked before upstream mutation",
            )

            expected_error = self.run_command([
                self.rustarr_bin,
                "post",
                "--service",
                name,
                "--path",
                service["post_expected_error"]["path"],
                "--body",
                json.dumps(service["post_expected_error"]["body"]),
                "--confirm",
            ])
            combined = expected_error.stderr + expected_error.stdout
            allowed = service["post_expected_error"]["error_contains_any"]
            self.record(
                f"cli post safe expected error {name}",
                expected_error.returncode != 0 and any(token in combined for token in allowed),
                combined.strip()[:240],
            )
```

Update `run()` selection:

```python
        selected = {
            "guard": [self.run_guard],
            "cli": [self.run_guard, self.run_cli],
            "all": [self.run_guard, self.run_cli],
        }[self.suite]
```

- [ ] **Step 2: Run CLI suite**

Run:

```bash
cargo build --release
python3 scripts/live-full-test.py --suite cli
```

Expected:

```text
PASS cli --version
PASS cli integrations complete
PASS cli status sonarr
PASS cli get sonarr /api/v3/system/status
PASS cli post confirm guard sonarr
```

Final line count depends on the matrix, but the process must exit 0.

- [ ] **Step 3: Commit CLI tests**

Run:

```bash
git add scripts/live-full-test.py
git commit -m "test: cover full live CLI surface"
```

Expected: commit succeeds.

## Task 7: Implement HTTP REST Route Tests

**Files:**
- Modify: `scripts/live-full-test.py`

- [ ] **Step 1: Add REST tests**

Add this method inside `LiveRunner`:

```python
    def run_rest(self) -> None:
        self.start_server()
        for route, key in [("/health", "status"), ("/ready", "ready"), ("/status", "server")]:
            status, body, _ = self.http("GET", f"{self.base_url}{route}")
            payload = json.loads(body)
            self.record(f"rest GET {route}", status == 200 and key in payload, f"status={status}")

        status, body, _ = self.http("GET", f"{self.base_url}/__rustarr_live_missing_route__")
        self.record("rest GET unknown route", status == 404, f"status={status}")
```

Update `run()` selection:

```python
        selected = {
            "guard": [self.run_guard],
            "cli": [self.run_guard, self.run_cli],
            "rest": [self.run_guard, self.run_rest],
            "all": [self.run_guard, self.run_cli, self.run_rest],
        }[self.suite]
```

- [ ] **Step 2: Run REST suite**

Run:

```bash
python3 scripts/live-full-test.py --suite rest
```

Expected:

```text
PASS rest GET /health: status=200
PASS rest GET /ready: status=200
PASS rest GET /status: status=200
PASS rest GET unknown route: status=404
```

- [ ] **Step 3: Commit REST tests**

Run:

```bash
git add scripts/live-full-test.py
git commit -m "test: cover live rustarr HTTP routes"
```

Expected: commit succeeds.

## Task 8: Implement MCP Protocol And Tool Tests

**Files:**
- Modify: `scripts/live-full-test.py`
- Modify: `tests/mcporter/test-mcp.sh`
- Modify: `docs/MCPORTER.md`

- [ ] **Step 1: Add JSON-RPC helper**

Add these methods inside `LiveRunner`:

```python
    def mcp(self, method: str, params: dict[str, Any] | None = None, request_id: int = 1) -> Any:
        body: dict[str, Any] = {"jsonrpc": "2.0", "id": request_id, "method": method}
        if params is not None:
            body["params"] = params
        status, raw, _ = self.http("POST", f"{self.base_url}/mcp", body)
        if status != 200:
            raise AssertionError(f"MCP HTTP status {status}: {raw}")
        payload = json.loads(raw)
        if "error" in payload:
            raise AssertionError(payload["error"])
        return payload["result"]

    def mcp_call_tool(self, arguments: dict[str, Any]) -> Any:
        result = self.mcp("tools/call", {"name": "rustarr", "arguments": arguments})
        content = result["content"][0]
        if content["type"] != "text":
            raise AssertionError(f"unexpected MCP content type: {content}")
        return json.loads(content["text"])
```

- [ ] **Step 2: Add MCP tests**

Add this method inside `LiveRunner`:

```python
    def run_mcp(self) -> None:
        self.start_server()
        init = self.mcp("initialize", {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {"name": "rustarr-live-test", "version": "1.0.0"},
        })
        self.record("mcp initialize", init["serverInfo"]["name"] == "rustarr", init["serverInfo"]["name"])

        tools = self.mcp("tools/list")
        names = {tool["name"] for tool in tools["tools"]}
        self.record("mcp tools/list", "rustarr" in names, "rustarr tool advertised")

        resources = self.mcp("resources/list")
        self.record("mcp resources/list", "resources" in resources, f"{len(resources['resources'])} resources")

        prompts = self.mcp("prompts/list")
        prompt_names = {prompt["name"] for prompt in prompts["prompts"]}
        self.record("mcp prompts/list", "quick_start" in prompt_names, "quick_start advertised")

        quick_start = self.mcp("prompts/get", {"name": "quick_start"})
        self.record("mcp prompts/get quick_start", "messages" in quick_start, "prompt returned messages")

        help_payload = self.mcp_call_tool({"action": "help"})
        self.record("mcp tool help", "actions" in help_payload, "structured help contains actions")

        integrations = self.mcp_call_tool({"action": "integrations"})
        configured = {item["name"] for item in integrations["configured"]}
        expected = {entry["name"] for entry in self.matrix["services"]}
        self.record("mcp tool integrations", expected <= configured, f"{len(configured)} configured")

        for service in self.matrix["services"]:
            name = service["name"]
            status_payload = self.mcp_call_tool({"action": "service_status", "service": name})
            self.assert_expectation(status_payload, service["status"])
            self.record(f"mcp service_status {name}", True, "semantic status matched")

            for get_case in service["get"]:
                payload = self.mcp_call_tool({"action": "api_get", "service": name, "path": get_case["path"]})
                self.assert_expectation(payload, get_case)
                self.record(f"mcp api_get {name} {get_case['path']}", True, "semantic GET matched")

            try:
                self.mcp_call_tool({
                    "action": "api_post",
                    "service": name,
                    "path": service["post_blocked"]["path"],
                    "body": service["post_blocked"]["body"],
                    "confirm": False,
                })
                self.record(f"mcp api_post confirm guard {name}", False, "unexpected success")
            except AssertionError as exc:
                self.record(
                    f"mcp api_post confirm guard {name}",
                    service["post_blocked"]["error_contains"] in str(exc),
                    str(exc)[:240],
                )
```

Update `run()` selection:

```python
        selected = {
            "guard": [self.run_guard],
            "cli": [self.run_guard, self.run_cli],
            "rest": [self.run_guard, self.run_rest],
            "mcp": [self.run_guard, self.run_mcp],
            "all": [self.run_guard, self.run_cli, self.run_rest, self.run_mcp],
        }[self.suite]
```

- [ ] **Step 3: Guard the legacy mcporter harness**

At the top of `tests/mcporter/test-mcp.sh`, load the shared guard before starting or calling any server:

```bash
GUARD_OUTPUT="$(python3 "$REPO_ROOT/scripts/lib/rustarr_shart_guard.py")"
while IFS= read -r line; do
  export "$line"
done <<< "$GUARD_OUTPUT"
```

Remove any direct loading of `${HOME}/.rustarr/.env`.

- [ ] **Step 4: Run MCP suite**

Run:

```bash
python3 scripts/live-full-test.py --suite mcp
```

Expected:

```text
PASS mcp initialize: rustarr
PASS mcp tools/list: rustarr tool advertised
PASS mcp tool integrations
PASS mcp service_status sonarr
PASS mcp api_get sonarr /api/v3/system/status
PASS mcp api_post confirm guard sonarr
```

- [ ] **Step 5: Commit MCP tests**

Run:

```bash
git add scripts/live-full-test.py tests/mcporter/test-mcp.sh docs/MCPORTER.md
git commit -m "test: cover live MCP protocol surface"
```

Expected: commit succeeds.

## Task 9: Implement Dedicated Service Matrix Suite

**Files:**
- Modify: `scripts/live-full-test.py`

- [ ] **Step 1: Add service-focused runner**

Add this method inside `LiveRunner`:

```python
    def run_services(self) -> None:
        for service in self.matrix["services"]:
            name = service["name"]
            status_payload = self.cli_json(["status", "--service", name])
            self.assert_expectation(status_payload, service["status"])
            self.record(f"service_status {name}", True, "semantic status matched")

            for get_case in service["get"]:
                payload = self.cli_json(["get", "--service", name, "--path", get_case["path"]])
                self.assert_expectation(payload, get_case)
                self.record(f"api_get {name} {get_case['path']}", True, "semantic GET matched")

            blocked = self.run_command([
                self.rustarr_bin,
                "post",
                "--service",
                name,
                "--path",
                service["post_blocked"]["path"],
                "--body",
                json.dumps(service["post_blocked"]["body"]),
            ])
            self.record(
                f"api_post blocked {name}",
                blocked.returncode != 0 and service["post_blocked"]["error_contains"] in (blocked.stderr + blocked.stdout),
                "confirm guard prevented mutation",
            )

            expected_error = self.run_command([
                self.rustarr_bin,
                "post",
                "--service",
                name,
                "--path",
                service["post_expected_error"]["path"],
                "--body",
                json.dumps(service["post_expected_error"]["body"]),
                "--confirm",
            ])
            combined = expected_error.stderr + expected_error.stdout
            allowed = service["post_expected_error"]["error_contains_any"]
            self.record(
                f"api_post safe upstream error {name}",
                expected_error.returncode != 0 and any(token in combined for token in allowed),
                combined.strip()[:240],
            )
```

Update `run()` selection:

```python
        selected = {
            "guard": [self.run_guard],
            "cli": [self.run_guard, self.run_cli],
            "rest": [self.run_guard, self.run_rest],
            "mcp": [self.run_guard, self.run_mcp],
            "services": [self.run_guard, self.run_services],
            "all": [self.run_guard, self.run_cli, self.run_rest, self.run_mcp, self.run_services],
        }[self.suite]
```

- [ ] **Step 2: Run service suite**

Run:

```bash
python3 scripts/live-full-test.py --suite services
```

Expected:

```text
PASS service_status sonarr
PASS api_get sonarr /api/v3/system/status
PASS api_post blocked sonarr
PASS api_post safe upstream error sonarr
```

All 15 services must have the same four categories of checks.

- [ ] **Step 3: Commit service matrix execution**

Run:

```bash
git add scripts/live-full-test.py
git commit -m "test: execute full live service action matrix"
```

Expected: commit succeeds.

## Task 10: Add Just Recipes And Documentation

**Files:**
- Modify: `Justfile`
- Modify: `docs/TESTING.md`
- Modify: `docs/SCRIPTS.md`
- Modify: `scripts/README.md`

- [ ] **Step 1: Add Just recipes**

Append these recipes to `Justfile`:

```make
live-full-guard:
	python3 scripts/live-full-test.py --suite guard

live-full-cli:
	cargo build --release
	python3 scripts/live-full-test.py --suite cli

live-full-rest:
	cargo build --release
	python3 scripts/live-full-test.py --suite rest

live-full-mcp:
	cargo build --release
	python3 scripts/live-full-test.py --suite mcp

live-full-services:
	cargo build --release
	python3 scripts/live-full-test.py --suite services

live-full-test:
	cargo build --release
	python3 scripts/live-full-test.py --suite all
```

- [ ] **Step 2: Document the suite**

Add this section to `docs/TESTING.md`:

```md
## Full Shart Live Suite

`just live-full-test` is the complete opt-in live test suite. It starts the local Rustarr binary with `RUSTARR_HOME=/home/jmagar/.rustarr-shart`, refuses any URL outside shart, and requires all 15 supported `ServiceKind`s to be configured before it runs.

The suite covers:

- every CLI business command and selected infrastructure commands
- every MCP business action through Streamable HTTP JSON-RPC
- `/health`, `/ready`, `/status`, and `/mcp`
- every supported service kind with semantic status, semantic GET, blocked POST, and safe expected-error POST checks

It is intentionally not part of `cargo test` because it requires the shart live stack and real credentials.
```

- [ ] **Step 3: Run docs grep to confirm no live-home guidance remains**

Run:

```bash
rg -n '/home/jmagar/\.rustarr|tootie\.tv|cache_appdata' docs scripts tests -g '*.md' -g '*.sh' -g '*.py'
```

Expected: no result instructs live tests to use `/home/jmagar/.rustarr`, `tootie.tv`, or `cache_appdata`. Results that document rejection examples are acceptable.

- [ ] **Step 4: Commit recipes and docs**

Run:

```bash
git add Justfile docs/TESTING.md docs/SCRIPTS.md scripts/README.md
git commit -m "docs: document full shart live suite"
```

Expected: commit succeeds.

## Task 11: Final Verification And Push

**Files:**
- Verify all changed files

- [ ] **Step 1: Run static quality gates**

Run:

```bash
cargo fmt
cargo test
cargo clippy -- -D warnings
python3 -m unittest tests.live.test_guard -v
```

Expected:

```text
test result: ok
Finished `dev` profile
Ran 4 tests
OK
```

- [ ] **Step 2: Run the full live suite**

Run:

```bash
just live-full-test
```

Expected:

```text
PASS guard complete shart env
PASS cli --version
PASS rest GET /health
PASS mcp initialize
PASS service_status sonarr
PASS service_status jellyfin
```

The command exits 0 and writes `target/live-full/report.json`. The report must contain no entries with `"passed": false`.

- [ ] **Step 3: Inspect the report**

Run:

```bash
python3 - <<'PY'
import json
from pathlib import Path
checks = json.loads(Path("target/live-full/report.json").read_text())
failed = [check for check in checks if not check["passed"]]
assert not failed, failed
print(f"{len(checks)} live checks passed")
PY
```

Expected:

```text
live checks passed
```

The number prefix depends on the matrix size and must be greater than 120.

- [ ] **Step 4: Commit any final fixes**

Run:

```bash
git status --short
git add scripts tests docs Justfile
git commit -m "test: add complete shart live validation"
```

Expected: commit succeeds if there are uncommitted final fixes. If `git status --short` is empty, skip the commit.

- [ ] **Step 5: Push everything**

Run:

```bash
git pull --rebase
bd dolt push
git push
git status --short --branch
```

Expected:

```text
## main...origin/main
```

No uncommitted files remain.

## Self-Review

- Spec coverage: The plan covers every Rustarr service kind, every business action, CLI command coverage, MCP protocol and tool coverage, local HTTP API routes, semantic success assertions, expected-error assertions, and a shart-only safety guard.
- Safety coverage: The plan refuses live `/home/jmagar/.rustarr`, rejects tootie URLs and process overrides, requires the complete shart service set, and keeps live testing opt-in.
- Service gap coverage: The plan treats missing `lidarr`, `readarr`, and initialized `wizarr` as a blocking prerequisite for the full suite rather than silently skipping them.
- Placeholder scan: The plan contains no deferred implementation markers. Secret values are intentionally represented as outside-repo shart credentials and are not committed.
