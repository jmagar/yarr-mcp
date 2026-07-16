#!/usr/bin/env python3
"""Audit binary-owned plugin hook setup contracts across Rust MCP servers."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKSPACE = ROOT.parent
REQUIRED_FIELDS = {
    "exit_policy",
    "ran_repair",
    "no_repair",
    "blocking_failures",
    "advisory_failures",
}
EXIT_POLICIES = {"success", "advisory_failure", "blocking_failure"}


@dataclass(frozen=True)
class Server:
    name: str
    repo: Path
    binary: str
    hook: str | None
    package_args: tuple[str, ...] = ()
    setup_args: tuple[str, ...] = ("setup", "plugin-hook", "--no-repair")
    env: tuple[tuple[str, str], ...] = ()
    appdata_env: str = "CLAUDE_PLUGIN_DATA"
    make_appdata: bool = True


SERVERS = [
    Server(
        "cortex",
        WORKSPACE / "cortex",
        "cortex",
        "scripts/plugin-setup.sh",
        setup_args=("setup", "plugin-hook", "--no-repair", "--json"),
        env=(("CORTEX_TOKEN", "test-token"),),
    ),
    Server(
        "gotify",
        WORKSPACE / "gotify-rmcp",
        "rgotify",
        "plugins/gotify/scripts/plugin-setup.sh",
        setup_args=("--json", "setup", "plugin-hook", "--no-repair"),
        appdata_env="GOTIFY_MCP_HOME",
    ),
    Server(
        "unifi",
        WORKSPACE / "unifi-rmcp",
        "runifi",
        "plugins/unifi/scripts/plugin-setup.sh",
        setup_args=("--json", "setup", "plugin-hook", "--no-repair"),
        appdata_env="UNIFI_MCP_HOME",
    ),
    Server(
        "tailscale",
        WORKSPACE / "tailscale-rmcp",
        "rtailscale",
        "plugins/tailscale/scripts/plugin-setup.sh",
        setup_args=("--json", "setup", "plugin-hook", "--no-repair"),
        appdata_env="TAILSCALE_MCP_HOME",
    ),
    Server(
        "apprise",
        WORKSPACE / "apprise-rmcp",
        "rapprise",
        "plugins/apprise/scripts/plugin-setup.sh",
        env=(("APPRISE_URL", "http://apprise.yarr:8000"), ("APPRISE_MCP_TOKEN", "test-token")),
    ),
    Server(
        "unraid",
        WORKSPACE / "unraid-rmcp",
        "runraid",
        "plugins/unraid/scripts/plugin-setup.sh",
        env=(
            ("UNRAID_API_URL", "https://tower.yarr/graphql"),
            ("UNRAID_API_KEY", "test-key"),
            ("UNRAID_MCP_TOKEN", "test-token"),
        ),
        appdata_env="UNRAID_HOME",
    ),
    Server(
        "yarr",
        ROOT,
        "yarr",
        None,
        env=(
            ("YARR_SERVICES", "sonarr"),
            ("YARR_SONARR_URL", "https://sonarr.yarr.test"),
            ("YARR_SONARR_API_KEY", "test-key"),
            ("YARR_MCP_TOKEN", "test-token"),
        ),
        appdata_env="YARR_HOME",
    ),
    Server(
        "labby",
        WORKSPACE / "lab",
        "labby",
        None,
        package_args=("-p", "labby", "--all-features"),
        setup_args=("setup", "plugin-hook", "--no-repair", "--json"),
        appdata_env="LAB_HOME",
    ),
]


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def check_hook(server: Server) -> None:
    if server.hook is None:
        return
    hook = server.repo / server.hook
    if not hook.is_file():
        fail(f"{server.name}: missing hook {hook}")
    text = hook.read_text()
    expected = f"{server.binary} setup plugin-hook \"$@\""
    delegates_via_resolved_binary = "}\" setup plugin-hook \"$@\"" in text and "command -v" in text
    if expected not in text and not delegates_via_resolved_binary:
        fail(
            f"{server.name}: hook must delegate with `{expected}` "
            "or a command-v-resolved binary"
        )
    forbidden = ["docker compose", "systemctl"]
    found = [token for token in forbidden if token in text]
    if found:
        fail(f"{server.name}: hook contains forbidden bootstrap tokens: {', '.join(found)}")
    subprocess.run(["bash", "-n", str(hook)], check=True)


def check_binary(server: Server) -> None:
    with tempfile.TemporaryDirectory(prefix=f"{server.name}-plugin-contract-") as temp:
        appdata = Path(temp) / "appdata"
        log_dir = Path(temp) / "logs"
        if server.make_appdata:
            appdata.mkdir()
        log_dir.mkdir()
        env = {
            "PATH": f"{server.repo / 'target' / 'debug'}:{os.environ.get('PATH', '')}",
            "RUST_LOG": "warn",
            "LAB_LOG_DIR": str(log_dir),
            server.appdata_env: str(appdata),
            "CLAUDE_PLUGIN_DATA": str(appdata),
            **dict(server.env),
        }
        command = ["cargo", "run", "--quiet", *server.package_args, "--", *server.setup_args]
        output = subprocess.run(
            command,
            cwd=server.repo,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    stdout = output.stdout.strip()
    if not stdout.startswith("{"):
        stderr = output.stderr.strip()
        fail(
            f"{server.name}: setup command did not emit clean JSON on stdout: "
            f"{stdout[:120]!r}; stderr: {stderr[:240]!r}"
        )
    try:
        payload = json.loads(stdout)
    except json.JSONDecodeError as error:
        fail(f"{server.name}: setup stdout is not JSON: {error}")
    missing = REQUIRED_FIELDS.difference(payload)
    if missing:
        fail(f"{server.name}: JSON missing fields: {', '.join(sorted(missing))}")
    if payload["exit_policy"] not in EXIT_POLICIES:
        fail(f"{server.name}: invalid exit_policy {payload['exit_policy']!r}")
    if not isinstance(payload["blocking_failures"], list):
        fail(f"{server.name}: blocking_failures must be an array")
    if not isinstance(payload["advisory_failures"], list):
        fail(f"{server.name}: advisory_failures must be an array")
    if output.returncode != 0 and payload["exit_policy"] != "blocking_failure":
        fail(f"{server.name}: nonzero exit with non-blocking policy")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--execute", action="store_true", help="run each binary setup command via cargo run")
    args = parser.parse_args()

    for server in SERVERS:
        check_hook(server)
        if args.execute:
            check_binary(server)
        print(f"ok {server.name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
