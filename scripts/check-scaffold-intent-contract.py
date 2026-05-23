#!/usr/bin/env python3
"""Validate scaffold intent contract JSON and checked-in rustarrs.

This intentionally avoids third-party Python dependencies so it can run in CI and
fresh template checkouts. It validates the parts of the contract we rely on for
scaffold planning; it is not a full JSON Schema implementation.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "docs/contracts/scaffold-intent.schema.json"
RUSTARRS = ROOT / "docs/contracts/rustarrs"

SURFACES = {"api", "cli", "mcp", "web"}
AUTH_KINDS = {"none", "api-key", "bearer", "oauth", "both", "other"}
TRANSPORTS = {"stdio", "http", "dual"}
PRIMITIVES = {"tools", "resources", "prompts", "elicitation"}
DEPLOYMENTS = {"none", "systemd", "docker"}
PLUGINS = {"claude", "codex", "gemini"}
CRATE_RE = re.compile(r"^[a-z][a-z0-9-]*$")
IDENT_RE = re.compile(r"^[a-z][a-z0-9_]*$")
ENV_RE = re.compile(r"^[A-Z][A-Z0-9_]*$")
API_URL_ENV_RE = re.compile(r"^[A-Z][A-Z0-9_]*_API_URL$")


def load_json(path: Path) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise AssertionError(f"{path}: invalid JSON: {exc}") from exc


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def require_keys(obj: dict, path: str, keys: set[str]) -> None:
    missing = keys - set(obj)
    require(not missing, f"{path}: missing required keys: {sorted(missing)}")


def require_no_extra(obj: dict, path: str, keys: set[str]) -> None:
    extra = set(obj) - keys
    require(not extra, f"{path}: unexpected keys: {sorted(extra)}")


def is_uri(value: str) -> bool:
    parsed = urlparse(value)
    return parsed.scheme in {"http", "https", "ssh", "git"} and bool(parsed.netloc)


def unique_list(value: object, path: str, allowed: set[str] | None = None) -> list:
    require(isinstance(value, list), f"{path}: expected list")
    require(len(value) == len(set(value)), f"{path}: duplicate values are not allowed")
    if allowed is not None:
        invalid = [item for item in value if item not in allowed]
        require(not invalid, f"{path}: invalid values: {invalid}; allowed={sorted(allowed)}")
    return value


def validate_schema() -> None:
    schema = load_json(SCHEMA)
    require(isinstance(schema, dict), f"{SCHEMA}: schema root must be an object")
    require(schema.get("$schema") == "https://json-schema.org/draft/2020-12/schema", f"{SCHEMA}: expected JSON Schema draft 2020-12")
    require(schema.get("properties", {}).get("kind", {}).get("const") == "rustarr_scaffold_intent", f"{SCHEMA}: kind const drifted")
    required = set(schema.get("required", []))
    expected_required = {
        "kind",
        "schema_version",
        "server_category",
        "required_surfaces",
        "project",
        "upstream",
        "runtime",
        "mcp_primitives",
        "deployment",
        "plugins",
        "publish_mcp",
        "crawl_docs",
        "handoff",
        "policy",
    }
    require(required == expected_required, f"{SCHEMA}: root required fields drifted: {sorted(required)}")
    properties = set(schema.get("properties", {}))
    require(expected_required <= properties, f"{SCHEMA}: root properties missing required fields")
    require("actions" not in properties, f"{SCHEMA}: legacy actions property must not exist")
    require("resource_groups" not in json.dumps(schema), f"{SCHEMA}: legacy resource_groups field must not exist")
    auth_enum = set(schema.get("properties", {}).get("upstream", {}).get("properties", {}).get("auth_kind", {}).get("enum", []))
    require(auth_enum == AUTH_KINDS, f"{SCHEMA}: auth_kind enum mismatch: {sorted(auth_enum)}")


def validate_payload(payload: object, source: Path) -> None:
    require(isinstance(payload, dict), f"{source}: root must be an object")
    root_keys = {
        "kind",
        "schema_version",
        "server_category",
        "required_surfaces",
        "project",
        "upstream",
        "runtime",
        "mcp_primitives",
        "deployment",
        "plugins",
        "publish_mcp",
        "crawl_docs",
        "handoff",
        "policy",
    }
    require_keys(payload, str(source), root_keys)
    require_no_extra(payload, str(source), root_keys)
    require(payload["kind"] == "rustarr_scaffold_intent", f"{source}: invalid kind")
    require(payload["schema_version"] == 1, f"{source}: invalid schema_version")

    category = payload["server_category"]
    require(category in {"upstream-client", "application-platform"}, f"{source}: invalid server_category")
    surfaces = unique_list(payload["required_surfaces"], f"{source}: required_surfaces", SURFACES)
    if category == "upstream-client":
        require(surfaces == ["mcp", "cli"], f"{source}: upstream-client must use ['mcp', 'cli']")
    else:
        require(set(surfaces) == {"api", "cli", "mcp", "web"}, f"{source}: application-platform must include api, cli, mcp, web")

    project = payload["project"]
    require(isinstance(project, dict), f"{source}: project must be object")
    project_keys = {"display_name", "crate_name", "binary_name", "service_name", "env_prefix"}
    require_keys(project, f"{source}: project", project_keys)
    require_no_extra(project, f"{source}: project", project_keys)
    require(project["display_name"], f"{source}: project.display_name required")
    require(CRATE_RE.match(project["crate_name"]), f"{source}: invalid crate_name")
    require(CRATE_RE.match(project["binary_name"]), f"{source}: invalid binary_name")
    require(IDENT_RE.match(project["service_name"]), f"{source}: invalid service_name")
    require(ENV_RE.match(project["env_prefix"]), f"{source}: invalid env_prefix")

    upstream = payload["upstream"]
    require(isinstance(upstream, dict), f"{source}: upstream must be object")
    require_no_extra(upstream, f"{source}: upstream", {"base_url_env", "auth_kind"})
    require(API_URL_ENV_RE.match(upstream["base_url_env"]), f"{source}: invalid upstream.base_url_env")
    require(upstream["auth_kind"] in AUTH_KINDS, f"{source}: invalid auth_kind")

    runtime = payload["runtime"]
    require(isinstance(runtime, dict), f"{source}: runtime must be object")
    require_no_extra(runtime, f"{source}: runtime", {"host", "port", "mcp_transport"})
    require(isinstance(runtime["host"], str) and runtime["host"], f"{source}: runtime.host required")
    require(isinstance(runtime["port"], int) and 1 <= runtime["port"] <= 65535, f"{source}: runtime.port out of range")
    require(runtime["mcp_transport"] in TRANSPORTS, f"{source}: invalid runtime.mcp_transport")

    unique_list(payload["mcp_primitives"], f"{source}: mcp_primitives", PRIMITIVES)
    require(payload["deployment"] in DEPLOYMENTS, f"{source}: invalid deployment")
    unique_list(payload["plugins"], f"{source}: plugins", PLUGINS)
    require(isinstance(payload["publish_mcp"], bool), f"{source}: publish_mcp must be boolean")

    crawl = payload["crawl_docs"]
    require(isinstance(crawl, dict), f"{source}: crawl_docs must be object")
    require_no_extra(crawl, f"{source}: crawl_docs", {"urls", "repos", "search_topics"})
    for key in ("urls", "repos", "search_topics"):
        values = unique_list(crawl[key], f"{source}: crawl_docs.{key}")
        require(all(isinstance(item, str) and item for item in values), f"{source}: crawl_docs.{key} entries must be non-empty strings")
    for key in ("urls", "repos"):
        require(all(is_uri(item) for item in crawl[key]), f"{source}: crawl_docs.{key} entries must be URIs")

    handoff = payload["handoff"]
    require(isinstance(handoff, dict), f"{source}: handoff must be object")
    handoff_keys = {"recommended_skill", "instructions"}
    require_keys(handoff, f"{source}: handoff", handoff_keys)
    require_no_extra(handoff, f"{source}: handoff", handoff_keys)
    require(handoff.get("recommended_skill") == "scaffold-project", f"{source}: handoff.recommended_skill must be scaffold-project")
    require("approve" in handoff.get("instructions", "").lower(), f"{source}: handoff instructions must mention approval")

    policy = payload["policy"]
    require(isinstance(policy, dict), f"{source}: policy must be object")
    policy_keys = {"business_action_minimum_surfaces", "upstream_client_surfaces", "application_platform_surfaces"}
    require_keys(policy, f"{source}: policy", policy_keys)
    require_no_extra(policy, f"{source}: policy", policy_keys)
    require(policy.get("business_action_minimum_surfaces") == ["mcp", "cli"], f"{source}: business action minimum must be ['mcp', 'cli']")
    require(policy.get("upstream_client_surfaces") == ["mcp", "cli"], f"{source}: upstream policy mismatch")
    require(set(policy.get("application_platform_surfaces", [])) == {"api", "cli", "mcp", "web"}, f"{source}: application policy mismatch")


def main() -> int:
    try:
        validate_schema()
        rustarrs = sorted(RUSTARRS.glob("scaffold-intent-*.json"))
        require(bool(rustarrs), f"{RUSTARRS}: no scaffold intent rustarrs found")
        for rustarr in rustarrs:
            validate_payload(load_json(rustarr), rustarr)
    except AssertionError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    print("scaffold intent contract and rustarrs are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
