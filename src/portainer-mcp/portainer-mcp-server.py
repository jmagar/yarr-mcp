"""
MCP Server for Portainer
Implements the approved tool set for managing Docker environments via Portainer.
Built with FastMCP following best practices from gofastmcp.com
Transport: SSE
"""

import os
import sys
import json
import httpx
import logging
from logging.handlers import RotatingFileHandler
from pathlib import Path
from typing import Optional, List, Dict, Any, Union

from fastmcp import FastMCP, Context
from pydantic import BaseModel, Field # BaseModel for schema definition if needed

# --- Environment Variable Loading & Validation ---
# Attempt to load .env file if python-dotenv is available and .env exists
try:
    from dotenv import load_dotenv
    # Try loading .env from script's directory, then one level up (project root)
    script_dir = Path(__file__).resolve().parent
    project_root_env = script_dir.parent.parent / ".env" # Path to <workspace_root>/.env updated
    script_dir_env = script_dir / ".env"         # Path to <script_dir>/.env

    # Prioritize server-specific .env, then project root .env
    # The load_dotenv function by default does not override existing environment variables.
    # To achieve override behavior (server .env > project .env > system env), 
    # we would ideally load in reverse order of precedence if `override=True` was used, 
    # or ensure `override=True` is used when loading more specific ones.
    # For now, let's assume python-dotenv default behavior (first load wins for a given var unless override=True).
    # To make sure our specific ones are loaded correctly, we can load project first, then specific with override.

    # Initial load from project root (if it exists), no override
    # This establishes base settings if not already in system env.
    if project_root_env.exists():
        load_dotenv(project_root_env, override=False)
        # logger.info(f"Loaded .env file from {project_root_env}") # Logged later
    
    # Load from script's directory (if it exists), with override to take precedence
    if script_dir_env.exists():
        load_dotenv(script_dir_env, override=True)
        # logger.info(f"Loaded .env file from {script_dir_env} (overriding project/system)") # Logged later

except ImportError:
    pass # python-dotenv not installed, or no .env file

PORTAINER_URL = os.getenv("PORTAINER_URL")
PORTAINER_API_KEY = os.getenv("PORTAINER_API_KEY")

# MCP Server specific configurations
PORTAINER_MCP_TRANSPORT = os.getenv("PORTAINER_MCP_TRANSPORT", "sse").lower()
PORTAINER_MCP_HOST = os.getenv("PORTAINER_MCP_HOST", "0.0.0.0")
PORTAINER_MCP_PORT = int(os.getenv("PORTAINER_MCP_PORT", "6971")) # Align with .env.example
PORTAINER_MCP_LOG_LEVEL = os.getenv("PORTAINER_MCP_LOG_LEVEL", "INFO").upper() # Specific name
PORTAINER_MCP_LOG_FILE = os.getenv("PORTAINER_MCP_LOG_FILE", "portainer_mcp.log") # New variable

# --- Logger Setup ---
NUMERIC_LOG_LEVEL = getattr(logging, PORTAINER_MCP_LOG_LEVEL, logging.INFO)
SCRIPT_DIR = Path(__file__).resolve().parent # Already defined, ensure it's used consistently
LOG_FILE_PATH = SCRIPT_DIR / PORTAINER_MCP_LOG_FILE # Use the new variable

logger = logging.getLogger("PortainerMCPServer")
logger.setLevel(NUMERIC_LOG_LEVEL)
logger.propagate = False

# Console Handler
console_handler = logging.StreamHandler(sys.stdout)
console_handler.setLevel(NUMERIC_LOG_LEVEL)
console_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
console_handler.setFormatter(console_formatter)
logger.addHandler(console_handler)

# File Handler with Rotation
try:
    file_handler = RotatingFileHandler(LOG_FILE_PATH, maxBytes=5*1024*1024, backupCount=3, encoding='utf-8')
    file_handler.setLevel(NUMERIC_LOG_LEVEL)
    file_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(module)s - %(funcName)s - %(lineno)d - %(message)s')
    file_handler.setFormatter(file_formatter)
    logger.addHandler(file_handler)
    logger.info(f"Logging initialized (console and file: {LOG_FILE_PATH}).")
except Exception as e:
    logger.warning(f"Could not initialize file logging to {LOG_FILE_PATH}: {e}")
    logger.info("Logging initialized (console only).")


# Log loaded environment variables (cautiously)
# Also indicate where .env was loaded from, if it was.
env_load_path_info_project = " (project root .env found)" if "project_root_env" in locals() and project_root_env.exists() else ""
env_load_path_info_specific = " (server .env found, overrides project/system)" if "script_dir_env" in locals() and script_dir_env.exists() else ""

logger.info(f"PORTAINER_URL loaded{env_load_path_info_project or env_load_path_info_specific}: {PORTAINER_URL[:20] if PORTAINER_URL else 'Not set'}...")
logger.info(f"PORTAINER_API_KEY loaded{env_load_path_info_project or env_load_path_info_specific}: {'****' if PORTAINER_API_KEY else 'Not Found'}")
logger.info(f"PORTAINER_MCP_TRANSPORT: {PORTAINER_MCP_TRANSPORT}")
logger.info(f"PORTAINER_MCP_HOST: {PORTAINER_MCP_HOST}")
logger.info(f"PORTAINER_MCP_PORT: {PORTAINER_MCP_PORT}")
logger.info(f"PORTAINER_MCP_LOG_LEVEL: {PORTAINER_MCP_LOG_LEVEL}")
logger.info(f"PORTAINER_MCP_LOG_FILE: {PORTAINER_MCP_LOG_FILE}")


# --- Critical Checks ---
if not PORTAINER_URL:
    logger.error("PORTAINER_URL environment variable must be set.")
    sys.exit(1)
if not PORTAINER_API_KEY:
    logger.error("PORTAINER_API_KEY environment variable must be set.")
    sys.exit(1)

# --- Initialize FastMCP Server ---
mcp = FastMCP(
    name="Portainer MCP Server",
    instructions="This server provides tools to interact with a Portainer instance for managing Docker environments. Ensure PORTAINER_URL and PORTAINER_API_KEY are set in the environment."
)

# --- HTTP Client Utility ---
async def _portainer_request(method: str, path: str, params: Optional[Dict] = None, json_data: Optional[Dict] = None) -> Dict[str, Any]:
    """Helper function to make requests to the Portainer API."""
    headers = {"X-API-Key": PORTAINER_API_KEY}
    url = f"{PORTAINER_URL.rstrip('/')}/api{path}"
    try:
        async with httpx.AsyncClient(timeout=30.0) as client: # Increased timeout
            logger.debug(f"Making Portainer API request: {method} {url} Params: {params} JSON: {json_data}")
            response = await client.request(method, url, headers=headers, params=params, json=json_data)
            response.raise_for_status()
            if response.status_code == 204: # No content
                return {"status": "success", "message": "Operation successful, no content returned."}
            return response.json()
    except httpx.HTTPStatusError as e:
        logger.error(f"Portainer API request failed to {url} with status {e.response.status_code}: {e.response.text}")
        error_details = {"error": f"API request failed with status {e.response.status_code}", "details": "No response body" }
        try:
            error_details["details"] = e.response.json()
        except json.JSONDecodeError:
            error_details["details"] = e.response.text
        return error_details
    except httpx.RequestError as e:
        logger.error(f"Portainer API request failed to {url}: {e}")
        return {"error": "Request to Portainer API failed", "details": str(e)}
    except json.JSONDecodeError as e:
        logger.error(f"Failed to decode JSON response from {url}: {e}")
        return {"error": "Invalid JSON response from Portainer API", "details": str(e)}

# --- Tool Definitions ---

@mcp.tool()
async def list_endpoints(
    ctx: Context,
    search: Optional[str] = Field(None, description="Keyword to filter endpoints by name."),
    group_ids: Optional[List[int]] = Field(None, description="Filter by group IDs."),
    tag_ids: Optional[List[int]] = Field(None, description="Filter by tag IDs."),
    types: Optional[List[int]] = Field(None, description="Filter by endpoint types (1: Docker, 2: Agent on Docker, 3: Azure ACI, 4: Edge Agent, 5: Kubernetes Local).")
) -> Dict[str, Any]:
    """
    Retrieves a list of all configured Portainer environments (endpoints)
    that the authenticated user has access to.
    """
    logger.info(f"Listing endpoints with search: {search}, groups: {group_ids}, tags: {tag_ids}, types: {types}")
    params = {}
    if search:
        params["search"] = search
    if group_ids:
        params["groupIds"] = json.dumps(group_ids) # Portainer API expects JSON array for list params
    if tag_ids:
        params["tagIds"] = json.dumps(tag_ids)
    if types:
        params["types"] = json.dumps(types)

    response = await _portainer_request("GET", "/endpoints", params=params)

    if "error" in response:
        return {"status": "error", "result": response}

    if not isinstance(response, list):
        logger.error(f"Unexpected response format from /endpoints: {response}")
        return {"status": "error", "result": "Unexpected response format from Portainer API."}

    readable_endpoints = []
    for ep in response:
        ep_type_map = {1: "Docker", 2: "Agent on Docker", 3: "Azure ACI", 4: "Edge Agent", 5: "Kubernetes Local", 6: "Edge Kubernetes", 7: "KaaS"}
        status_map = {1: "Up", 2: "Down", 3: "Troubled"}
        readable_endpoints.append({
            "id": ep.get("Id"),
            "name": ep.get("Name"),
            "type": ep_type_map.get(ep.get("Type"), "Unknown"),
            "status": status_map.get(ep.get("Status"), "Unknown"),
            "url": ep.get("URL", "N/A")
        })
    return {"status": "success", "result": readable_endpoints}

@mcp.tool()
async def get_endpoint_details(ctx: Context, endpoint_id: int = Field(..., description="The ID of the environment to inspect.")) -> Dict[str, Any]:
    """
    Retrieves detailed information about a specific Portainer environment (endpoint).
    """
    logger.info(f"Getting details for endpoint ID: {endpoint_id}")
    response = await _portainer_request("GET", f"/endpoints/{endpoint_id}")

    if "error" in response:
        return {"status": "error", "result": response}

    ep_type_map = {1: "Docker", 2: "Agent on Docker", 3: "Azure ACI", 4: "Edge Agent", 5: "Kubernetes Local", 6: "Edge Kubernetes", 7: "KaaS"}
    status_map = {1: "Up", 2: "Down", 3: "Troubled"}

    details = {
        "id": response.get("Id"),
        "name": response.get("Name"),
        "type": ep_type_map.get(response.get("Type"), "Unknown"),
        "status": status_map.get(response.get("Status"), "Unknown"),
        "url": response.get("URL"),
        "public_url": response.get("PublicURL"),
        "group_id": response.get("GroupId"),
        "tags": response.get("TagIds", []),
        "platform": "N/A", # Default platform
    }

    snapshot = None # Initialize snapshot variable
    if response.get("Snapshots") and isinstance(response["Snapshots"], list) and len(response["Snapshots"]) > 0:
        snapshot = response["Snapshots"][0]
        details["docker_version"] = snapshot.get("DockerVersion") # Common for Docker-based snapshots
        details["total_cpu"] = snapshot.get("TotalCPU")
        details["total_memory_gb"] = round(snapshot.get("TotalMemory", 0) / (1024**3), 2) if snapshot.get("TotalMemory") else 0
        details["container_count"] = snapshot.get("ContainerCount")
        details["image_count"] = snapshot.get("ImageCount")
        details["volume_count"] = snapshot.get("VolumeCount")
        details["stack_count"] = snapshot.get("StackCount")

    # Platform information for Docker type endpoints
    if response.get("Type") == 1: # Docker
        if response.get("Platform"):
            details["platform"] = response["Platform"]
        elif snapshot and snapshot.get("Platform"): # Check the Docker snapshot
            details["platform"] = snapshot["Platform"]

    # Kubernetes specific details
    if response.get("Type") in [5, 6, 7] and response.get("Kubernetes", {}).get("Snapshots"):
         kube_snapshot = response["Kubernetes"]["Snapshots"][0]
         details["kubernetes_version"] = kube_snapshot.get("KubernetesVersion")
         details["node_count"] = kube_snapshot.get("NodeCount")
         # Platform for K8s might also be available, though less common in this specific structure
         # For instance, if K8s nodes had platform info, it would be aggregated differently.

    return {"status": "success", "result": details}


@mcp.tool()
async def list_docker_containers(
    ctx: Context,
    endpoint_id: int = Field(..., description="The ID of the Docker environment."),
    all_containers: bool = Field(False, description="If true, lists all containers. If false, only running containers."),
    filters: Optional[str] = Field(None, description="JSON string for Docker API filters (e.g., '{\"name\": [\"my-container\"]}')")
) -> Dict[str, Any]:
    """
    Lists containers within a specific Docker environment.
    Uses the proxied Docker API endpoint: /endpoints/{id}/docker/containers/json
    """
    logger.info(f"Listing Docker containers for endpoint_id: {endpoint_id}, all_containers: {all_containers}, filters: {filters}")
    params = {"all": "1" if all_containers else "0"}
    if filters:
        try:
            # Ensure filters is valid JSON
            json.loads(filters)
            params["filters"] = filters
        except json.JSONDecodeError as e:
            logger.error(f"Invalid JSON provided for filters: {filters} - Error: {e}")
            # Consider raising an HTTPException for bad client input
            pass # Or raise error

    response_json = await _portainer_request("get", f"/endpoints/{endpoint_id}/docker/containers/json", params=params)
    return response_json


@mcp.tool()
async def inspect_docker_container(
    ctx: Context,
    endpoint_id: int = Field(..., description="The ID of the Docker environment."),
    container_id: str = Field(..., description="The ID or name of the container.")
) -> Dict[str, Any]:
    """
    Retrieves detailed information about a specific container in a Docker environment.
    Uses the proxied Docker API endpoint: /endpoints/{id}/docker/containers/{container_id}/json
    """
    logger.info(f"Inspecting container ID: {container_id} in endpoint ID: {endpoint_id}")
    response = await _portainer_request("GET", f"/endpoints/{endpoint_id}/docker/containers/{container_id}/json")

    if "error" in response:
        return {"status": "error", "result": response}

    # Extracting key human-readable information
    name = response.get("Name", "N/A").lstrip('/')
    image_name = response.get("Config", {}).get("Image", "N/A")
    state = response.get("State", {})
    created = response.get("Created", "N/A")

    env_vars = response.get("Config", {}).get("Env", [])
    mounts = []
    for m in response.get("Mounts", []):
        mounts.append(f"{m.get('Source', 'N/A')} -> {m.get('Destination', 'N/A')} ({'ro' if m.get('RW') == False else 'rw'})")

    network_settings = response.get("NetworkSettings", {})
    ip_address = network_settings.get("IPAddress", "N/A")
    if not ip_address and network_settings.get("Networks"): # For containers on custom networks
        first_net = next(iter(network_settings["Networks"].values()), None)
        if first_net:
            ip_address = first_net.get("IPAddress", "N/A")

    ports = {}
    if network_settings.get("Ports"):
        for container_port_protocol, host_bindings in network_settings["Ports"].items():
            if host_bindings:
                formatted_bindings = [f"{b.get('HostIp', '0.0.0.0')}:{b.get('HostPort')}" for b in host_bindings]
                ports[container_port_protocol] = ", ".join(formatted_bindings)
            else:
                ports[container_port_protocol] = "Not published"


    readable_details = {
        "id": response.get("Id", "N/A")[:12],
        "name": name,
        "image": image_name,
        "created": created,
        "state": {
            "status": state.get("Status", "N/A"),
            "running": state.get("Running", False),
            "started_at": state.get("StartedAt", "N/A"),
            "finished_at": state.get("FinishedAt", "N/A") if not state.get("Running") else "N/A",
        },
        "command": " ".join(response.get("Config", {}).get("Cmd", []) or []) or "N/A",
        "entrypoint": " ".join(response.get("Config", {}).get("Entrypoint", []) or []) or "N/A",
        "ip_address": ip_address,
        "ports": ports if ports else "N/A",
        "mounts": mounts if mounts else "N/A",
        "env_vars": env_vars if env_vars else "N/A",
    }
    return {"status": "success", "result": readable_details}


@mcp.tool()
async def manage_docker_container(
    ctx: Context,
    endpoint_id: int = Field(..., description="The ID of the Docker environment."),
    container_id: str = Field(..., description="The ID or name of the container."),
    action: str = Field(..., description="Action to perform. Valid values: 'start', 'stop', 'restart', 'pause', 'unpause', 'kill', 'remove'.")
) -> Dict[str, Any]:
    """
    Manages a container: start, stop, restart, pause, unpause, kill, or remove.
    Uses proxied Docker API endpoints.
    """
    logger.info(f"Performing action '{action}' on container ID: {container_id} in endpoint ID: {endpoint_id}")
    valid_actions = ["start", "stop", "restart", "pause", "unpause", "kill", "remove"]
    if action not in valid_actions:
        return {"status": "error", "result": f"Invalid action '{action}'. Valid actions are: {', '.join(valid_actions)}"}

    path = f"/endpoints/{endpoint_id}/docker/containers/{container_id}/{action}"
    method = "POST"
    if action == "remove":
        method = "DELETE"
        path = f"/endpoints/{endpoint_id}/docker/containers/{container_id}" # Docker API uses DELETE on the container resource itself
        # Add force=true for removal to avoid issues with running containers, common UX
        response = await _portainer_request(method, path, params={"force": "true"})
    else:
        response = await _portainer_request(method, path)


    if "error" in response:
        # Try to get a more specific error if possible
        container_name = container_id # Fallback if inspect fails
        try:
            inspect_resp = await _portainer_request("GET", f"/endpoints/{endpoint_id}/docker/containers/{container_id}/json")
            if "Name" in inspect_resp:
                container_name = inspect_resp["Name"].lstrip('/')
        except:
            pass

        error_msg = response.get("details", {}).get("message", response.get("error", "Unknown error"))
        return {"status": "error", "message": f"Failed to {action} container '{container_name}': {error_msg}", "details": response}

    return {"status": "success", "message": f"Container '{container_id}' action '{action}' executed successfully.", "details": response if response else "No content returned."}


@mcp.tool()
async def get_docker_container_logs(
    ctx: Context,
    endpoint_id: int = Field(..., description="The ID of the Docker environment."),
    container_id: str = Field(..., description="The ID or name of the container."),
    timestamps: bool = Field(False, description="Show timestamps in logs."),
    tail: str = Field("100", description="Number of lines to show from the end of the logs (e.g., '100', or 'all')."),
    since: Optional[str] = Field(None, description="Show logs since a UNIX timestamp (integer) or relative time (e.g., '10m', '1h').")
) -> Dict[str, Any]:
    """
    Fetches logs from a specific container in a Docker environment.
    Uses the proxied Docker API endpoint: /endpoints/{id}/docker/containers/{container_id}/logs
    """
    logger.info(f"Fetching logs for container ID: {container_id} in endpoint ID: {endpoint_id}, timestamps: {timestamps}, tail: {tail}, since: {since}")
    params: Dict[str, Any] = {
        "stdout": "true",
        "stderr": "true",
        "timestamps": str(timestamps).lower(),
        "tail": tail
    }
    if since:
        params["since"] = since # Docker API handles integer (unix timestamp) or string (duration)

    # This endpoint returns raw logs, not JSON
    headers = {"X-API-Key": PORTAINER_API_KEY, "Accept": "text/plain"} # Ensure we get plain text
    url = f"{PORTAINER_URL.rstrip('/')}/api/endpoints/{endpoint_id}/docker/containers/{container_id}/logs"

    try:
        async with httpx.AsyncClient(timeout=30.0) as client:
            response = await client.get(url, headers=headers, params=params)
            response.raise_for_status()
            # Logs can contain non-UTF-8 characters, especially control characters.
            # We need to sanitize or replace them.
            raw_logs = response.text
            # A simple way to remove most control characters except for common whitespace like \n, \r, \t
            sanitized_logs = "".join(ch if ch.isprintable() or ch in ['\\n', '\\r', '\\t'] else '' for ch in raw_logs)

            return {"status": "success", "logs": sanitized_logs if sanitized_logs else "No logs returned or logs are empty."}
    except httpx.HTTPStatusError as e:
        logger.error(f"Portainer API request for logs failed with status {e.response.status_code}: {e.response.text}")
        return {"status": "error", "result": f"API request for logs failed with status {e.response.status_code}", "details": e.response.text}
    except httpx.RequestError as e:
        logger.error(f"Portainer API request for logs failed: {e}")
        return {"status": "error", "result": "Request to Portainer API for logs failed", "details": str(e)}


@mcp.tool()
async def list_stacks(
    ctx: Context,
    filters: Optional[str] = Field(None, description="JSON encoded map for filtering (e.g., '{\\\"SwarmID\\\": \\\"abc...\\\", \\\"EndpointID\\\": 1}')")
) -> Dict[str, Any]:
    """
    Lists all stacks (Swarm or Compose) that the user has access to.
    """
    logger.info(f"Listing stacks with filters: {filters}")
    params = {}
    if filters:
        try:
            params["filters"] = json.loads(filters) # The API expects the filter object directly as JSON string
        except json.JSONDecodeError as e:
            return {"status": "error", "result": f"Invalid JSON in filters parameter: {e}"}

    response = await _portainer_request("GET", "/stacks", params=params)

    if "error" in response:
        return {"status": "error", "result": response}

    if not isinstance(response, list):
        logger.error(f"Unexpected response from /stacks: {response}")
        return {"status": "error", "result": "Unexpected response format from Portainer API."}

    readable_stacks = []
    stack_type_map = {1: "Swarm", 2: "Compose", 3: "Kubernetes"} # As per API spec for customTemplate, assuming similar for stacks
    status_map = {1: "Active", 2: "Inactive"} # Common status mapping

    for s in response:
        readable_stacks.append({
            "id": s.get("Id"),
            "name": s.get("Name"),
            "type": stack_type_map.get(s.get("Type"), "Unknown"),
            "endpoint_id": s.get("EndpointId"),
            "status": status_map.get(s.get("Status"), "Unknown")
        })
    return {"status": "success", "result": readable_stacks}

@mcp.tool()
async def inspect_stack(ctx: Context, stack_id: int = Field(..., description="The ID of the stack.")) -> Dict[str, Any]:
    """
    Retrieves detailed information about a specific stack.
    """
    logger.info(f"Inspecting stack ID: {stack_id}")
    response = await _portainer_request("GET", f"/stacks/{stack_id}")

    if "error" in response:
        return {"status": "error", "result": response}

    stack_type_map = {1: "Swarm", 2: "Compose", 3: "Kubernetes"}
    status_map = {1: "Active", 2: "Inactive"}

    details = {
        "id": response.get("Id"),
        "name": response.get("Name"),
        "type": stack_type_map.get(response.get("Type"), "Unknown"),
        "endpoint_id": response.get("EndpointId"),
        "status": status_map.get(response.get("Status"), "Unknown"),
        "creation_date": response.get("CreationDate"), # Assuming this is a timestamp or ISO string
        "created_by": response.get("CreatedBy", "N/A"),
        "updated_date": response.get("UpdateDate"),
        "updated_by": response.get("UpdatedBy", "N/A"),
        "entry_point": response.get("EntryPoint", "N/A"),
        "project_path": response.get("ProjectPath", "N/A"),
        "env_vars_count": len(response.get("Env", [])),
        "is_git_stack": "GitConfig" in response and response["GitConfig"] is not None,
        "git_url": response.get("GitConfig", {}).get("URL") if response.get("GitConfig") else "N/A",
    }
    return {"status": "success", "result": details}


@mcp.tool()
async def get_stack_file(
    ctx: Context,
    stack_id: int = Field(..., description="The ID of the stack.")
) -> Dict[str, Any]:
    """
    Retrieves the compose file content for a specific stack.
    """
    logger.info(f"Getting stack file for stack_id: {stack_id}")
    response_data = await _portainer_request("get", f"/stacks/{stack_id}/file")

    if isinstance(response_data, str):
        # If the response is already a string, it's likely the direct file content
        return {"stack_file_content": response_data}
    elif isinstance(response_data, dict) and "StackFileContent" in response_data:
        # If it's a dict and has the expected key
        return {"stack_file_content": response_data.get("StackFileContent")}
    else:
        # Unexpected response format
        logger.error(f"Unexpected response format from /stacks/{stack_id}/file: {type(response_data)} - {response_data}")
        # Consider raising an error or returning a more specific error message
        return {"error": "Unexpected response format from Portainer API for stack file."}


# --- Main Execution ---
if __name__ == "__main__":
    logger.info(f"Starting Portainer MCP Server...")
    
    if PORTAINER_MCP_TRANSPORT == 'sse':
        mcp.run(
            transport='sse',
            host=PORTAINER_MCP_HOST,
            port=PORTAINER_MCP_PORT,
            path='/mcp'  # Standardized path for MCP over SSE
        )
    elif PORTAINER_MCP_TRANSPORT == 'stdio':
        mcp.run() # Defaults to stdio
    else:
        logger.error(f"Invalid PORTAINER_MCP_TRANSPORT: '{PORTAINER_MCP_TRANSPORT}'. Must be 'sse' or 'stdio'. Defaulting to STDIO.")
        mcp.run()
    logger.info("Portainer MCP Server stopped.") 