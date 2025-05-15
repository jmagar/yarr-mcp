"""
MCP Server for SABnzbd
Implements a tool set for interacting with a SABnzbd instance.
Built with FastMCP following best practices from gofastmcp.com
Transport: Determined by SABNZBD_MCP_TRANSPORT environment variable (stdio or sse)
"""
import asyncio
import os
import sys
from contextlib import asynccontextmanager # Added for lifespan
from fastmcp import FastMCP, Context # Assuming Context might be needed for tools later
from dotenv import load_dotenv
import logging
from typing import Optional, List, Dict, Union, Any, Annotated # Added Any, Annotated
from pathlib import Path # For .env loading
from logging.handlers import RotatingFileHandler # Added
from pydantic import BeforeValidator # Added for type coercion
from fastapi.middleware.cors import CORSMiddleware

# Ensure the script's directory is on sys.path for direct imports if running from elsewhere
SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR)) # Add to front for precedence

# Direct import
from client import SabnzbdApiClient

# --- Environment Loading & Configuration ---
# Load from .env in the project root if it exists
project_root = Path(__file__).resolve().parent.parent.parent # yarr-mcp/
env_path = project_root / '.env'

# Initial minimal logging for dotenv loading itself
# print(f"SabnzbdMCP: Looking for .env file at: {env_path}") # Will be logged later
found_dotenv = load_dotenv(dotenv_path=env_path, override=True)
# print(f"SabnzbdMCP: load_dotenv found project root .env: {found_dotenv}") # Will be logged later

# Also load .env from the specific mcp server directory if it exists (more specific overrides project root)
mcp_server_env_path = SCRIPT_DIR / '.env'
if mcp_server_env_path.exists() and mcp_server_env_path != env_path:
    # print(f"SabnzbdMCP: Looking for specific .env file at: {mcp_server_env_path}") # Will be logged later
    found_mcp_dotenv = load_dotenv(dotenv_path=mcp_server_env_path, override=True)
    # print(f"SabnzbdMCP: load_dotenv found server-specific .env: {found_mcp_dotenv}") # Will be logged later


SABNZBD_URL = os.getenv("SABNZBD_URL")
SABNZBD_API_KEY = os.getenv("SABNZBD_API_KEY")

# MCP Server specific configurations
SABNZBD_MCP_TRANSPORT = os.getenv("SABNZBD_MCP_TRANSPORT", "sse").lower()
SABNZBD_MCP_HOST = os.getenv("SABNZBD_MCP_HOST", "0.0.0.0")
SABNZBD_MCP_PORT = int(os.getenv("SABNZBD_MCP_PORT", "8004"))
SABNZBD_MCP_LOG_LEVEL = os.getenv("SABNZBD_MCP_LOG_LEVEL", os.getenv("LOG_LEVEL", "INFO")).upper() # Align with template's LOG_LEVEL fallback

# --- Logger Setup ---
# Use [SERVICE]_NAME for log file convention from template
SERVICE_NAME_FOR_LOG = os.getenv("SABNZBD_NAME", "sabnzbd").lower()
LOG_FILE_NAME = f"{SERVICE_NAME_FOR_LOG}_mcp.log"
LOG_FILE_PATH = SCRIPT_DIR / LOG_FILE_NAME

NUMERIC_LOG_LEVEL = getattr(logging, SABNZBD_MCP_LOG_LEVEL, logging.INFO)

logger = logging.getLogger(__name__)
logger.setLevel(NUMERIC_LOG_LEVEL)
logger.propagate = False # Prevent root logger from duplicating messages

# Console Handler
console_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
console_handler = logging.StreamHandler(sys.stdout)
console_handler.setLevel(NUMERIC_LOG_LEVEL)
console_handler.setFormatter(console_formatter)
logger.addHandler(console_handler)

# Rotating File Handler
file_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(module)s - %(funcName)s - %(lineno)d - %(message)s')
try:
    # Ensure the logs directory exists if LOG_FILE_PATH includes a path (it shouldn't if just SCRIPT_DIR / name)
    if LOG_FILE_PATH.parent != SCRIPT_DIR and not LOG_FILE_PATH.parent.exists():
        LOG_FILE_PATH.parent.mkdir(parents=True, exist_ok=True)
        # print(f"SabnzbdMCP: Created log directory: {LOG_FILE_PATH.parent}") # Logged below

    file_handler = RotatingFileHandler(
        LOG_FILE_PATH, maxBytes=5*1024*1024, backupCount=3 # 5MB per file, 3 backups
    )
    file_handler.setLevel(NUMERIC_LOG_LEVEL)
    file_handler.setFormatter(file_formatter)
    logger.addHandler(file_handler)
except Exception as e:
    # If file logging fails, we still have console logging. Log the error there.
    logger.error(f"Failed to set up file logging for {LOG_FILE_PATH}: {e}", exc_info=True)

# Log environment loading after logger is configured
logger.info(f"Looking for .env file at: {env_path}")
logger.info(f"load_dotenv found project root .env: {found_dotenv}")
if mcp_server_env_path.exists() and mcp_server_env_path != env_path:
    logger.info(f"Looking for specific .env file at: {mcp_server_env_path}")
    logger.info(f"load_dotenv found server-specific .env: {locals().get('found_mcp_dotenv', 'Not Applicable')}")

logger.info(f"Logging initialized (console and file: {LOG_FILE_PATH}). Effective log level: {SABNZBD_MCP_LOG_LEVEL}")

# --- Critical Config Check (using logger) ---
if not SABNZBD_URL:
    logger.critical("CRITICAL: SABNZBD_URL is not set. Please set it in your .env file or environment.")
    sys.exit(1)
if not SABNZBD_API_KEY:
    logger.critical("CRITICAL: SABNZBD_API_KEY is not set. Please set it in your .env file or environment.")
    sys.exit(1)

# Remove the old SABNZBD_MCP_LOG_FILE variable as it's replaced by LOG_FILE_PATH convention
# SABNZBD_MCP_LOG_FILE = os.getenv("SABNZBD_MCP_LOG_FILE", "sabnzbd-mcp.log") # User specified

# --- Old Logger Setup (to be removed by the edit) ---
# LOG_FORMAT = '''%(asctime)s - %(name)s - %(levelname)s - %(filename)s:%(lineno)d - %(message)s'''
# # Ensure the logs directory exists if SABNZBD_MCP_LOG_FILE includes a path
# log_file_path = Path(SABNZBD_MCP_LOG_FILE)
# if log_file_path.parent != Path(".") and not log_file_path.parent.exists():
#     log_file_path.parent.mkdir(parents=True, exist_ok=True)
#     print(f"SabnzbdMCP: Created log directory: {log_file_path.parent}")
# 
# logging.basicConfig(level=SABNZBD_MCP_LOG_LEVEL, format=LOG_FORMAT)
# logger = logging.getLogger("SabnzbdMCP") # Specific logger for this MCP server
# 
# # Console Handler (already configured by basicConfig, but can be customized if needed)
# # stream_handler = logging.StreamHandler(sys.stdout)
# # stream_handler.setFormatter(logging.Formatter(LOG_FORMAT))
# # logger.addHandler(stream_handler) # basicConfig handles this
# 
# # Rotating File Handler
# try:
#     file_handler = RotatingFileHandler(
#         SABNZBD_MCP_LOG_FILE, maxBytes=5*1024*1024, backupCount=2 # 5MB per file, 2 backups
#     )
#     file_handler.setFormatter(logging.Formatter(LOG_FORMAT))
#     logger.addHandler(file_handler)
# except Exception as e:
#     logger.error(f"Failed to set up file logging for {SABNZBD_MCP_LOG_FILE}: {e}")

logger.info(f"SABNZBD_URL: {SABNZBD_URL}") # Log sensitive info carefully or not at all
logger.info(f"SABNZBD_API_KEY is {'SET' if SABNZBD_API_KEY else 'NOT SET'}") # Avoid logging the key itself
logger.info(f"SABNZBD_MCP_TRANSPORT: {SABNZBD_MCP_TRANSPORT}")
logger.info(f"SABNZBD_MCP_HOST: {SABNZBD_MCP_HOST}")
logger.info(f"SABNZBD_MCP_PORT: {SABNZBD_MCP_PORT}")
logger.info(f"SABNZBD_MCP_LOG_LEVEL: {SABNZBD_MCP_LOG_LEVEL}")
logger.info(f"SABNZBD_MCP_LOG_FILE: {LOG_FILE_PATH}")

# --- Type Coercion Helper ---
def coerce_to_int_or_none(v: Any) -> Optional[int]:
    if v is None:
        return None
    try:
        return int(v)
    except (ValueError, TypeError):
        # Log or handle appropriately if needed, for now, pass non-convertible as None
        # or raise error if strictness is required.
        logger.warning(f"Could not coerce value '{v}' to int, returning None.")
        return None

IntOrNone = Annotated[Optional[int], BeforeValidator(coerce_to_int_or_none)]

# --- Lifespan Manager for API Client ---
@asynccontextmanager
async def sabnzbd_api_client_lifespan(app: FastMCP):
    logger.info("Initializing Sabnzbd API client...")
    if not SABNZBD_URL or not SABNZBD_API_KEY: # Redundant check, but good for lifespan context
        logger.critical("SABNZBD_URL or SABNZBD_API_KEY is not configured. MCP server cannot function.")
        # FastMCP doesn't have a direct way to prevent startup from lifespan,
        # but we can log and the client will be None.
        # The initial sys.exit(1) should prevent this from being reached if run as main.
        app.sabnzbd_client = None
        yield
        return

    try:
        client = SabnzbdApiClient(base_url=SABNZBD_URL, api_key=SABNZBD_API_KEY)
        # Test connection (optional, but good practice)
        # For Sabnzbd, a simple call like getting server_stats can test connectivity.
        # test_response = await client.get_server_stats() # Assuming such a method exists and is async
        # logger.info(f"Successfully connected to Sabnzbd. Version: {test_response.get('version', 'N/A')}")
        app.sabnzbd_client = client
    except Exception as e:
        logger.exception(f"Failed to initialize SabnzbdApiClient: {e}")
        app.sabnzbd_client = None # Ensure it's None if init fails
    
    yield
    
    if app.sabnzbd_client:
        logger.info("Closing Sabnzbd API client...")
        await app.sabnzbd_client.close_session() # Assuming client has a close_session method
        logger.info("Sabnzbd API client closed.")

mcp = FastMCP(
    name="SabnzbdMCP", # Restored name
    instructions="Interact with a SABnzbd instance for managing downloads.", # Restored instructions
    lifespan=sabnzbd_api_client_lifespan
)

# --- Tool Definitions ---

@mcp.tool()
async def get_server_stats(ctx: Context) -> Union[Dict, str]:
    """Retrieves various statistics from the SABnzbd server, including queue details.
    This effectively calls the 'queue' mode of the SABnzbd API.

    Returns:
        A dictionary containing server statistics or an error string.
    """
    logger.info(f"Executing tool: get_server_stats (calling client.get_queue)")
    client = getattr(ctx.fastmcp, 'sabnzbd_client', None) # Corrected attribute name
    if not client:
        logger.error("Sabnzbd API client not available in context.")
        return "Error: SABnzbd API client is not available. Check server startup logs."

    try:
        # Assuming client.get_queue() with no args fetches the main queue/server status
        api_response = await client.get_queue(start=0, limit=0) # Use start=0, limit=0 for full status from queue endpoint
        if isinstance(api_response, str): # Error string from client
            logger.error(f"get_server_stats (via get_queue) failed: {api_response}")
            return api_response
        elif isinstance(api_response, dict):
            logger.info(f"Successfully retrieved SABnzbd server stats (via get_queue).")
            return api_response
        else:
            logger.warning(f"Unexpected API response structure from SABnzbd get_server_stats (via get_queue): {api_response}")
            return "Error: Received unexpected data structure from SABnzbd get_server_stats (via get_queue)."
    except Exception as e:
        logger.exception("Exception in get_server_stats (via get_queue)")
        return f"Error during get_server_stats (via get_queue): {e}"

@mcp.tool()
async def get_sab_queue(
    ctx: Context,
    start: Optional[int] = None,
    limit: Optional[int] = None,
    category: Optional[str] = None,
) -> Dict[str, Any]:
    """Retrieves the current download queue from SABnzbd.

    Args:
        start: Optional. Index of the job to start at (for pagination).
        limit: Optional. Number of jobs to display (for pagination).
        category: Optional. Filter queue by a specific category.

    Returns:
        A dictionary containing the queue information or an error string.
    """
    logger.info(f"Executing tool: get_sab_queue(start={start}, limit={limit}, category='{category}')")
    client = getattr(ctx.fastmcp, 'sabnzbd_client', None)
    if not client:
        return "Error: SABnzbd API client is not available. Check server startup logs."

    api_response = await client.get_queue(start=start, limit=limit, category=category)
    
    # The client method already returns a Dict or an error string.
    # We can do additional parsing/simplification here if needed, but for now, return directly.
    if isinstance(api_response, str): # Error string from client
        return api_response
    elif isinstance(api_response, dict) and 'queue' in api_response:
        logger.info(f"Successfully retrieved SABnzbd queue. Jobs: {len(api_response['queue'].get('slots', []))}")
        return api_response # Return the full queue object as per SABnzbd API
    else:
        logger.warning(f"Unexpected API response structure from SABnzbd queue: {api_response}")
        return "Error: Received unexpected data structure from SABnzbd queue."

@mcp.tool()
async def get_sab_history(
    ctx: Context,
    start: Optional[int] = None,
    limit: Optional[int] = None,
) -> Dict[str, Any]:
    """Retrieves the download history from SABnzbd.

    Args:
        start: Optional. Index of the history entry to start at (for pagination).
        limit: Optional. Number of history entries to display (for pagination).

    Returns:
        A dictionary containing the history information or an error string.
    """
    logger.info(f"Executing tool: get_sab_history(start={start}, limit={limit})")
    client = getattr(ctx.fastmcp, 'sabnzbd_client', None)
    if not client:
        return "Error: SABnzbd API client is not available. Check server startup logs."

    api_response = await client.get_history(start=start, limit=limit)
    
    if isinstance(api_response, str): # Error string from client
        return api_response
    elif isinstance(api_response, dict) and 'history' in api_response:
        logger.info(f"Successfully retrieved SABnzbd history. Entries: {len(api_response['history'].get('slots', []))}")
        return api_response # Return the full history object as per SABnzbd API
    else:
        logger.warning(f"Unexpected API response structure from SABnzbd history: {api_response}")
        return "Error: Received unexpected data structure from SABnzbd history."

@mcp.tool()
async def pause_sab_queue(ctx: Context) -> Union[Dict, str]:
    """Pauses the entire SABnzbd download queue."""
    logger.info(f"Executing tool: pause_sab_queue")
    client = getattr(ctx.fastmcp, 'sabnzbd_client', None)
    if not client:
        return "Error: SABnzbd API client is not available. Check server startup logs."
    
    api_response = await client.pause_queue()
    # SABnzbd typically returns { "status": true } on success for these
    if isinstance(api_response, dict) and api_response.get("status") is True:
        logger.info("SABnzbd queue paused successfully.")
        return {"status": "success", "message": "SABnzbd queue paused."}
    elif isinstance(api_response, str): # Error string from client
        return api_response
    else:
        logger.warning(f"Unexpected response from pause_sab_queue: {api_response}")
        return "Error: Received unexpected response when attempting to pause queue."

@mcp.tool()
async def resume_sab_queue(ctx: Context) -> Union[Dict, str]:
    """Resumes the entire SABnzbd download queue."""
    logger.info(f"Executing tool: resume_sab_queue")
    client = getattr(ctx.fastmcp, 'sabnzbd_client', None)
    if not client:
        return "Error: SABnzbd API client is not available. Check server startup logs."

    api_response = await client.resume_queue()
    if isinstance(api_response, dict) and api_response.get("status") is True:
        logger.info("SABnzbd queue resumed successfully.")
        return {"status": "success", "message": "SABnzbd queue resumed."}
    elif isinstance(api_response, str): # Error string from client
        return api_response
    else:
        logger.warning(f"Unexpected response from resume_sab_queue: {api_response}")
        return "Error: Received unexpected response when attempting to resume queue."

@mcp.tool()
async def add_nzb_url(ctx: Context, nzb_url: str, category: Optional[str] = None) -> Union[Dict, str]:
    """Adds an NZB to the SABnzbd queue by its URL.

    Args:
        nzb_url: The direct URL to the .nzb file.
        category: Optional. Category to assign to the download.

    Returns:
        A dictionary containing the API response (usually includes status and nzo_ids) or an error string.
    """
    logger.info(f"Executing tool: add_nzb_url(nzb_url='{nzb_url}', category='{category}')")
    client = getattr(ctx.fastmcp, 'sabnzbd_client', None)
    if not client:
        return "Error: SABnzbd API client is not available. Check server startup logs."

    api_response = await client.add_nzb_by_url(nzb_url=nzb_url, category=category)
    
    # SABnzbd response for addurl: { "status": true, "nzo_ids": ["SABnzbd_nzo_xxxxx"] }
    # or { "status": false, "error": "Invalid URL" }
    if isinstance(api_response, str): # Error string from our client wrapper
        return api_response
    elif isinstance(api_response, dict) and api_response.get("status") is True and "nzo_ids" in api_response:
        logger.info(f"Successfully added NZB URL '{nzb_url}'. NZO IDs: {api_response.get('nzo_ids')}")
        return api_response 
    elif isinstance(api_response, dict) and api_response.get("status") is False:
        error_msg = api_response.get("error", "Failed to add NZB URL.")
        logger.error(f"Failed to add NZB URL '{nzb_url}': {error_msg}")
        return f"Error: {error_msg}"
    else:
        logger.warning(f"Unexpected response from add_nzb_url: {api_response}")
        return "Error: Received unexpected response when adding NZB URL."

@mcp.tool()
async def set_sab_speedlimit(ctx: Context, percentage: int) -> Union[Dict, str]:
    """Sets the global download speed limit in SABnzbd as a percentage.

    Args:
        percentage: The speed limit percentage (0-100). 0 means no limit.

    Returns:
        A dictionary indicating success or an error string.
    """
    logger.info(f"Executing tool: set_sab_speedlimit(percentage={percentage})")
    client = getattr(ctx.fastmcp, 'sabnzbd_client', None)
    if not client:
        return "Error: SABnzbd API client is not available. Check server startup logs."

    if not 0 <= percentage <= 100:
        return "Error: Percentage must be between 0 and 100."

    # The client's set_speedlimit method expects a string (e.g., "50" for 50%)
    api_response = await client.set_speedlimit(percentage_str=str(percentage))
    
    # SABnzbd typically returns { "status": true } on success
    if isinstance(api_response, dict) and api_response.get("status") is True:
        logger.info(f"SABnzbd speed limit set to {percentage}% successfully.")
        return {"status": "success", "message": f"SABnzbd speed limit set to {percentage}%."}
    elif isinstance(api_response, str): # Error string from client
        return api_response
    else:
        logger.warning(f"Unexpected response from set_sab_speedlimit: {api_response}")
        return "Error: Received unexpected response when setting speed limit."

@mcp.tool()
async def toggle_pause_sabnzbd(ctx: Context) -> Union[Dict, str]:
    """Toggles the pause state of the SABnzbd download queue."""
    logger.info(f"Executing tool: toggle_pause_sabnzbd")
    client = getattr(ctx.fastmcp, 'sabnzbd_client', None)
    if not client:
        return "Error: SABnzbd API client is not available. Check server startup logs."

    try:
        target_pause_state = not await client.is_paused()
        logger.info(f"Toggling SABnzbd pause status to: {target_pause_state}")
        api_response = await client.pause_queue() if target_pause_state else await client.resume_queue()

        if isinstance(api_response, str): # Error string from client
            logger.error(f"toggle_pause_sabnzbd failed: {api_response}")
            return api_response
        elif isinstance(api_response, dict) and api_response.get("status") is True:
            logger.info(f"Successfully toggled SABnzbd pause to {target_pause_state}. Current state: Paused = {api_response.get('paused', 'Unknown')}")
            return api_response
        else:
            logger.warning(f"Unexpected API response from SABnzbd toggle_pause: {api_response}")
            return "Error: Received unexpected data structure or failed to toggle SABnzbd pause state."
    except Exception as e:
        logger.exception("Exception in toggle_pause_sabnzbd")
        return f"Error during toggle_pause_sabnzbd: {e}"

def main():
    logger.info(f"Starting Sabnzbd MCP Server with transport: {SABNZBD_MCP_TRANSPORT}")
    # Critical check for SABNZBD_URL and SABNZBD_API_KEY is at the top.

    if SABNZBD_MCP_TRANSPORT == "stdio":
        mcp.run()
    elif SABNZBD_MCP_TRANSPORT == "sse":
        mcp.run(
            transport="sse",
            host=SABNZBD_MCP_HOST,
            port=SABNZBD_MCP_PORT,
            path="/mcp"  # Standardized path
        )
    else:
        logger.error(f"Invalid SABNZBD_MCP_TRANSPORT: '{SABNZBD_MCP_TRANSPORT}'. Must be 'stdio' or 'sse'. Defaulting to stdio.")
        mcp.run() # Fallback to stdio if invalid config

if __name__ == "__main__":
    main() 