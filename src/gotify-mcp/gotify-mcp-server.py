"""
MCP Server for Gotify
Implements the approved tool set from the design phase.
Built with FastMCP following best practices from gofastmcp.com
Transport: SSE
"""

import os
# This is an OS import
import sys
import httpx
import logging
from pathlib import Path
from typing import Optional, Dict, Any, List
from dotenv import load_dotenv

from fastmcp import FastMCP, Context
from fastapi.middleware.cors import CORSMiddleware

# --- Environment Loading & Configuration ---
# Load .env file from the same directory as the script first
SCRIPT_DIR = Path(__file__).resolve().parent
load_dotenv(dotenv_path=SCRIPT_DIR / ".env")

# Then load .env from the workspace root if it exists (for broader project configs)
# This allows for a general .env at project root and specific overrides in the MCP server's directory
WORKSPACE_ROOT_ENV = Path(os.getcwd()) / ".env"
if WORKSPACE_ROOT_ENV.exists() and WORKSPACE_ROOT_ENV != SCRIPT_DIR / ".env":
    load_dotenv(dotenv_path=WORKSPACE_ROOT_ENV, override=False)
    # Do not override already set vars


# --- Logging Setup ---
GOTIFY_LOG_LEVEL_STR = os.getenv('GOTIFY_LOG_LEVEL', os.getenv('LOG_LEVEL', 'INFO')).upper()
# Prefer service-specific, fallback to generic
NUMERIC_LOG_LEVEL = getattr(logging, GOTIFY_LOG_LEVEL_STR, logging.INFO)

logger = logging.getLogger("GotifyMCPServer")
logger.setLevel(NUMERIC_LOG_LEVEL)
logger.propagate = False

# Console Handler
console_handler = logging.StreamHandler(sys.stdout)
console_handler.setLevel(NUMERIC_LOG_LEVEL)
console_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
console_handler.setFormatter(console_formatter)

# Ensure handlers are not duplicated if script is reloaded (e.g., in some dev environments)
if not any(isinstance(h, logging.StreamHandler) for h in logger.handlers):
    logger.addHandler(console_handler)

# File Handler (Optional, can be enabled if needed)
log_file_name = f"{os.getenv('GOTIFY_NAME', 'gotify').lower()}_mcp.log"
log_file_path = SCRIPT_DIR / log_file_name
file_handler = logging.handlers.RotatingFileHandler(log_file_path, maxBytes=5*1024*1024, backupCount=3, encoding='utf-8')
file_handler.setLevel(NUMERIC_LOG_LEVEL)
file_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(module)s - %(funcName)s - %(lineno)d - %(message)s')
file_handler.setFormatter(file_formatter)
if not any(isinstance(h, logging.handlers.RotatingFileHandler) for h in logger.handlers):
    logger.addHandler(file_handler)

logger.info(f"Logging initialized (console and file: {log_file_path}). Effective log level: {GOTIFY_LOG_LEVEL_STR}")

# --- Essential Configuration ---
GOTIFY_URL = os.getenv("GOTIFY_URL")
GOTIFY_CLIENT_TOKEN = os.getenv("GOTIFY_CLIENT_TOKEN")
# Used for most management tools
GOTIFY_APP_TOKEN_FROM_ENV = os.getenv("GOTIFY_APP_TOKEN")
# For logging purposes only
# Transport Config
GOTIFY_MCP_TRANSPORT = os.getenv("GOTIFY_MCP_TRANSPORT", "sse").lower()
GOTIFY_MCP_HOST = os.getenv("GOTIFY_MCP_HOST", "0.0.0.0")
GOTIFY_MCP_PORT = int(os.getenv("GOTIFY_MCP_PORT", "8000"))

if not GOTIFY_URL:
    logger.error("GOTIFY_URL must be set in the environment.")
    sys.exit(1)
if not GOTIFY_CLIENT_TOKEN:
    logger.warning("GOTIFY_CLIENT_TOKEN is not set. Most management tools (application/client management, getting all messages) will fail or be restricted.")
    # Not exiting, as create_message might still work if app_token is provided

logger.info(f"GOTIFY_URL loaded: {GOTIFY_URL}")
logger.info(f"GOTIFY_CLIENT_TOKEN (used as API Key for server ops) loaded: {'****' if GOTIFY_CLIENT_TOKEN else 'Not Found'}")
logger.info(f"GOTIFY_APP_TOKEN from env loaded: {'****' if GOTIFY_APP_TOKEN_FROM_ENV else 'Not Found'} (Note: This server expects app_token per create_message call, not globally)")
logger.info(f"GOTIFY_MCP_TRANSPORT set to: {GOTIFY_MCP_TRANSPORT}")
logger.info(f"GOTIFY_MCP_HOST set to: {GOTIFY_MCP_HOST}")
logger.info(f"GOTIFY_MCP_PORT set to: {GOTIFY_MCP_PORT}")
logger.info(f"GOTIFY_LOG_LEVEL (effective): {GOTIFY_LOG_LEVEL_STR}")


# --- FastMCP Server Initialization ---
mcp = FastMCP(
    name="Gotify MCP Server",
    instructions="""This server provides tools to interact with a Gotify instance.
You can send messages, manage applications, clients, and retrieve information like health and version.
For sending messages, an `app_token` is required per call.
For management tasks, a `GOTIFY_CLIENT_TOKEN` must be configured in the server's environment."""
)

# --- CORS Configuration for MCP Server ---
mcp_origins = [
    "http://localhost:5173",
    "http://127.0.0.1:5173",
    # Add other origins if your dashboard might be served from elsewhere
]

mcp.app.add_middleware(
    CORSMiddleware,
    allow_origins=mcp_origins,
    allow_credentials=True,
    allow_methods=["*"],
    # Allows all methods (GET, POST, etc.)
    allow_headers=["*"],
    # Allows all headers
)
# --- End CORS Configuration ---

# --- HTTP Client Utility ---
async def _request(
    method: str,
    endpoint: str,
    token: Optional[str] = None,
    params: Optional[Dict[str, Any]] = None,
    json_data: Optional[Dict[str, Any]] = None,
    is_app_token: bool = False
) -> Dict[str, Any]:
    """Helper function to make HTTP requests to the Gotify API."""
    headers = {}
    actual_token = token if token else GOTIFY_CLIENT_TOKEN

    if not actual_token:
        error_msg = "No token provided for API request. For create_message, pass app_token. For others, configure GOTIFY_CLIENT_TOKEN."
        logger.error(error_msg)
        return {"error": error_msg, "errorCode": 401, "errorDescription": "Authentication token missing."}

    # Gotify accepts token in query param or header. Header is generally preferred.
    # X-Gotify-Key is one way, Authorization: Bearer <token> is another.
    # The API spec shows `X-Gotify-Key` for appToken and clientToken security schemes.
    # It also shows `Authorization` with `Bearer` prefix for appToken and clientToken.
    # Let's use X-Gotify-Key for simplicity as it's explicitly named for both.
    headers["X-Gotify-Key"] = actual_token

    url = f"{GOTIFY_URL.rstrip('/')}/{endpoint.lstrip('/')}"
    logger.debug(f"Requesting {method} {url} with params {params} and json {json_data}")

    async with httpx.AsyncClient(timeout=20.0) as client:
        try:
            response = await client.request(method, url, params=params, json=json_data, headers=headers)
            response.raise_for_status()
            # Raises HTTPStatusError for 4xx/5xx
            if response.status_code == 204 or not response.content:
                # No content for some successful DELETEs
                return {"status": "success", "message": "Operation successful, no content returned."}
            return response.json()
        except httpx.HTTPStatusError as e:
            logger.error(f"HTTP error for {method} {url}: {e.response.status_code} - {e.response.text}")
            try:
                error_details = e.response.json()
            except Exception:
                error_details = {"errorDescription": e.response.text}
            return {
                "error": error_details.get("error", f"HTTP {e.response.status_code}"),
                "errorCode": error_details.get("errorCode", e.response.status_code),
                "errorDescription": error_details.get("errorDescription", "Failed to call Gotify API.")
            }
        except httpx.RequestError as e:
            logger.error(f"Request error for {method} {url}: {e}")
            return {"error": "RequestError", "errorCode": 500, "errorDescription": str(e)}
        except Exception as e:
            logger.error(f"Unexpected error for {method} {url}: {e}", exc_info=True)
            return {"error": "UnexpectedError", "errorCode": 500, "errorDescription": str(e)}

# --- MCP Tools ---

@mcp.tool()
async def create_message(
    app_token: str,
    message: str,
    title: Optional[str] = None,
    priority: Optional[int] = None,
    extras: Optional[Dict[str, Any]] = None
) -> Dict[str, Any]:
    """
    Sends a new message to Gotify using a specific application token.
    - app_token (required, string): The application token (appToken) for sending the message.
    - message (required, string): The content of the message. Markdown is allowed.
    - title (optional, string): The title of the message.
    - priority (optional, integer): The priority of the message.
    - extras (optional, dict): Additional data to send with the message.
    """
    logger.info(f"Creating message with title: {title}")
    payload = {"message": message}
    if title:
        payload["title"] = title
    if priority is not None:
        # priority can be 0
        payload["priority"] = priority
    if extras:
        payload["extras"] = extras
    
    # POST /message requires an appToken
    return await _request("POST", "message", token=app_token, json_data=payload, is_app_token=True)

@mcp.tool()
async def get_messages(
    limit: Optional[int] = 100,
    since: Optional[int] = None
) -> Dict[str, Any]:
    """
    Retrieves messages. Uses the globally configured GOTIFY_CLIENT_TOKEN.
    - limit (optional, int): Max messages to return (1-200, default 100).
    - since (optional, int): Return messages with ID less than this value (for pagination).
    """
    logger.info(f"Getting messages with limit: {limit}, since: {since}")
    params = {}
    if limit is not None:
        params["limit"] = limit
    if since is not None:
        params["since"] = since
    return await _request("GET", "message", params=params)

@mcp.tool()
async def delete_message(message_id: int) -> Dict[str, Any]:
    """
    Deletes a specific message by its ID. Uses the globally configured GOTIFY_CLIENT_TOKEN.
    - message_id (required, int): The ID of the message to delete.
    """
    logger.info(f"Deleting message with ID: {message_id}")
    return await _request("DELETE", f"message/{message_id}")

@mcp.tool()
async def create_application(
    name: str,
    description: Optional[str] = None,
    default_priority: Optional[int] = None
) -> Dict[str, Any]:
    """
    Creates a new application. Uses the globally configured GOTIFY_CLIENT_TOKEN.
    - name (required, string): The name of the application.
    - description (optional, string): A description for the application.
    - default_priority (optional, integer): Default priority for messages sent by this application.
    """
    logger.info(f"Creating application with name: {name}")
    payload = {"name": name}
    if description:
        payload["description"] = description
    if default_priority is not None:
        payload["defaultPriority"] = default_priority
    return await _request("POST", "application", json_data=payload)

@mcp.tool()
async def get_applications() -> Dict[str, Any]:
    """Retrieves a list of all applications. Uses the globally configured GOTIFY_CLIENT_TOKEN."""
    logger.info("Getting all applications.")
    return await _request("GET", "application")

@mcp.tool()
async def update_application(
    app_id: int,
    name: Optional[str] = None,
    description: Optional[str] = None,
    default_priority: Optional[int] = None
) -> Dict[str, Any]:
    """
    Updates an existing application's details. Uses the globally configured GOTIFY_CLIENT_TOKEN.
    - app_id (required, int): The ID of the application to update.
    - name (optional, string): New name for the application.
    - description (optional, string): New description for the application.
    - default_priority (optional, int): New default priority.
    """
    logger.info(f"Updating application ID: {app_id}")
    payload = {}
    if name:
        payload["name"] = name
    if description:
        payload["description"] = description
    if default_priority is not None:
        payload["defaultPriority"] = default_priority
    
    if not payload:
        return {"error": "NoUpdateFields", "errorCode": 400, "errorDescription": "No fields provided to update."}
    return await _request("PUT", f"application/{app_id}", json_data=payload)

@mcp.tool()
async def delete_application(app_id: int) -> Dict[str, Any]:
    """
    Deletes an application by its ID. Uses the globally configured GOTIFY_CLIENT_TOKEN.
    - app_id (required, int): The ID of the application to delete.
    """
    logger.info(f"Deleting application ID: {app_id}")
    return await _request("DELETE", f"application/{app_id}")

@mcp.tool()
async def create_client(name: str) -> Dict[str, Any]:
    """
    Creates a new client. Uses the globally configured GOTIFY_CLIENT_TOKEN.
    - name (required, string): The name of the client.
    """
    logger.info(f"Creating client with name: {name}")
    payload = {"name": name}
    return await _request("POST", "client", json_data=payload)

@mcp.tool()
async def get_clients() -> Dict[str, Any]:
    """Retrieves a list of all clients. Uses the globally configured GOTIFY_CLIENT_TOKEN."""
    logger.info("Getting all clients.")
    return await _request("GET", "client")

@mcp.tool()
async def get_health() -> Dict[str, Any]:
    """Checks the health status of the Gotify server."""
    logger.info("Getting Gotify server health.")
    # Health endpoint does not require authentication
    return await _request("GET", "health", token="dummy_token_not_used_for_health")
    # Pass dummy to satisfy _request logic

@mcp.tool()
async def get_version() -> Dict[str, Any]:
    """Retrieves version information of the Gotify server."""
    logger.info("Getting Gotify server version.")
    # Version endpoint does not require authentication
    return await _request("GET", "version", token="dummy_token_not_used_for_version")
    # Pass dummy

# --- New Health Endpoint for Dashboard ---
@mcp.app.get("/health", tags=["mcp_server_health"])
async def mcp_server_health_check() -> Dict[str, Any]:
    """
    Provides a health check for the MCP server itself and its ability to connect to Gotify.
    This is intended for use by monitoring dashboards.
    """
    logger.info("MCP server health check requested.")
    
    if not GOTIFY_URL:
        logger.error("GOTIFY_URL is not configured.")
        return {"status": "error", "service_accessible": False, "reason": "GOTIFY_URL not configured for MCP server."}

    # Perform a lightweight check against the Gotify instance, e.g., by getting its version or health.
    # Reusing the logic from the get_health tool or _request directly.
    # The get_health tool already uses GOTIFY_CLIENT_TOKEN if available, or can work without for basic health check.
    
    # A simple GET to /health should work even without a token for basic connectivity.
    # The Gotify /health endpoint itself doesn't strictly require a token.
    health_check_url = f"{GOTIFY_URL.rstrip('/')}/health"
    async with httpx.AsyncClient(timeout=10.0) as client:
        try:
            response = await client.get(health_check_url)
            if response.status_code == 200:
                # Further check if GOTIFY_CLIENT_TOKEN is set and try a token-requiring endpoint if needed for deeper check
                # For now, basic connectivity is enough for 'service_accessible'
                logger.info(f"Gotify instance accessible at {health_check_url}.")
                return {"status": "ok", "service_accessible": True, "reason": "Gotify instance is responsive."}
            else:
                logger.warning(f"Gotify instance at {health_check_url} returned status {response.status_code}: {response.text}")
                return {"status": "error", "service_accessible": False, "reason": f"Gotify instance returned HTTP {response.status_code}"}
        except httpx.RequestError as e:
            logger.error(f"RequestError while checking Gotify health at {health_check_url}: {e}")
            return {"status": "error", "service_accessible": False, "reason": f"RequestError: {str(e)}"}
        except Exception as e:
            logger.error(f"Unexpected error while checking Gotify health at {health_check_url}: {e}", exc_info=True)
            return {"status": "error", "service_accessible": False, "reason": f"Unexpected error: {str(e)}"}

# --- MCP Resources ---
@mcp.resource(uri="gotify://application/{app_id}/messages")
async def application_messages(
    app_id: int,
    limit: Optional[int] = 100,
    since_id: Optional[int] = None
    # Renamed from 'since' to avoid conflict with 'since' keyword and match PagedMessages 'since' field usage
) -> Dict[str, Any]:
    """
    Retrieves messages for a specific application ID. Uses GOTIFY_CLIENT_TOKEN.
    - app_id (required, int): The ID of the application.
    - limit (optional, int): Max messages (default 100).
    - since_id (optional, int): Return messages with ID less than this (for pagination).
    """
    logger.info(f"Getting messages for application ID: {app_id}, limit: {limit}, since: {since_id}")
    params = {}
    if limit is not None:
        params["limit"] = limit
    if since_id is not None:
        params["since"] = since_id
        # API expects 'since'
    return await _request("GET", f"application/{app_id}/message", params=params)


@mcp.resource(uri="gotify://currentuser")
async def current_user_info() -> Dict[str, Any]:
    """Retrieves details about the currently authenticated user (via GOTIFY_CLIENT_TOKEN)."""
    logger.info("Getting current user info.")
    return await _request("GET", "current/user")


# --- Server Runner ---
if __name__ == "__main__":
    logger.info(f"Starting Gotify MCP Server with transport: {GOTIFY_MCP_TRANSPORT}")
    if GOTIFY_MCP_TRANSPORT == "stdio":
        mcp.run()  # Default stdio transport
    elif GOTIFY_MCP_TRANSPORT == "sse":
        mcp.run(
            transport="sse",
            host=GOTIFY_MCP_HOST,
            port=GOTIFY_MCP_PORT,
            path="/mcp"
            # Standard MCP path for SSE
            # allow_introspection=True # Consider for debugging, but disable for production
        )
    else:
        logger.error(f"Invalid GOTIFY_MCP_TRANSPORT: '{GOTIFY_MCP_TRANSPORT}'. Must be 'stdio' or 'sse'. Defaulting to stdio.")
        mcp.run()