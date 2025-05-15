import asyncio
import os
import sys
from pathlib import Path

# Ensure the script's directory is on sys.path for direct imports
SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from contextlib import asynccontextmanager
from fastmcp import FastMCP, Context
from dotenv import load_dotenv
import logging
from logging.handlers import RotatingFileHandler
from typing import Optional, List, Dict, Union, Any
from fastapi.middleware.cors import CORSMiddleware

from client import TautulliApiClient # Direct import

# --- Environment Loading & Configuration ---
project_root = Path(__file__).resolve().parent.parent.parent
env_path = project_root / '.env'

# Initial minimal logging for dotenv loading itself
print(f"TautulliMCP: Looking for .env file at: {env_path}")
found_dotenv = load_dotenv(dotenv_path=env_path, override=True)
print(f"TautulliMCP: load_dotenv found file: {found_dotenv}")

TAUTULLI_URL = os.getenv("TAUTULLI_URL")
TAUTULLI_API_KEY = os.getenv("TAUTULLI_API_KEY")

# Transport and Port Configuration
TAUTULLI_MCP_TRANSPORT = os.getenv("TAUTULLI_MCP_TRANSPORT", "sse").lower()
TAUTULLI_MCP_HOST = os.getenv("TAUTULLI_MCP_HOST", "0.0.0.0")
TAUTULLI_MCP_PORT = int(os.getenv("TAUTULLI_MCP_PORT", "8002")) # Defaulting to a different port for Tautulli

# Logging Configuration
TAUTULLI_LOG_LEVEL_STR = os.getenv('TAUTULLI_LOG_LEVEL', os.getenv('LOG_LEVEL', 'INFO')).upper()
NUMERIC_LOG_LEVEL = getattr(logging, TAUTULLI_LOG_LEVEL_STR, logging.INFO)

logger = logging.getLogger("TautulliMCPServer")
logger.setLevel(NUMERIC_LOG_LEVEL)
logger.propagate = False

# Console Handler
console_handler = logging.StreamHandler(sys.stdout)
console_handler.setLevel(NUMERIC_LOG_LEVEL)
console_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
console_handler.setFormatter(console_formatter)
if not any(isinstance(h, logging.StreamHandler) for h in logger.handlers):
    logger.addHandler(console_handler)

# File Handler
log_file_name = f"{os.getenv('TAUTULLI_NAME', 'tautulli').lower()}_mcp.log"
log_file_path = SCRIPT_DIR / log_file_name
file_handler = RotatingFileHandler(log_file_path, maxBytes=5*1024*1024, backupCount=3, encoding='utf-8')
file_handler.setLevel(NUMERIC_LOG_LEVEL)
file_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(module)s - %(funcName)s - %(lineno)d - %(message)s')
file_handler.setFormatter(file_formatter)
if not any(isinstance(h, RotatingFileHandler) for h in logger.handlers):
    logger.addHandler(file_handler)

logger.info(f"Logging initialized (console and file: {log_file_path}). Effective log level: {TAUTULLI_LOG_LEVEL_STR}")

# Log loaded essential configurations
logger.info(f"TAUTULLI_URL loaded: {'Yes' if TAUTULLI_URL else 'No'}")
logger.info(f"TAUTULLI_API_KEY loaded: {'Yes' if TAUTULLI_API_KEY and len(TAUTULLI_API_KEY) > 5 else 'No'}")
logger.info(f"TAUTULLI_MCP_TRANSPORT set to: {TAUTULLI_MCP_TRANSPORT}")
logger.info(f"TAUTULLI_MCP_HOST set to: {TAUTULLI_MCP_HOST}")
logger.info(f"TAUTULLI_MCP_PORT set to: {TAUTULLI_MCP_PORT}")
logger.info(f"TAUTULLI_LOG_LEVEL (effective): {TAUTULLI_LOG_LEVEL_STR}")

# --- Critical Check for Essential API Credentials/URL ---
if not TAUTULLI_URL or not TAUTULLI_API_KEY:
    logger.error("CRITICAL: TAUTULLI_URL and TAUTULLI_API_KEY must be set in environment variables. Server cannot start.")
    sys.exit(1)

@asynccontextmanager
async def tautulli_lifespan(app: FastMCP):
    logger.info("Tautulli Lifespan: Startup sequence initiated.")
    if TAUTULLI_URL and TAUTULLI_API_KEY:
        try:
            app.tau_client = TautulliApiClient(base_url=TAUTULLI_URL, api_key=TAUTULLI_API_KEY)
            logger.info("Tautulli Lifespan: API client initialized.")
        except Exception as e:
            logger.error(f"Tautulli Lifespan Error: Failed to initialize TautulliApiClient: {e}", exc_info=True)
            app.tau_client = None
    else:
        logger.error("Tautulli Lifespan Error: TAUTULLI_URL or TAUTULLI_API_KEY missing. Client not initialized.")
        app.tau_client = None
    
    logger.info("Tautulli Lifespan: Startup complete.")
    yield
    
    logger.info("Tautulli Lifespan: Shutdown sequence initiated.")
    if hasattr(app, 'tau_client') and app.tau_client:
        await app.tau_client.close()
        logger.info("Tautulli Lifespan: API client closed.")
    logger.info("Tautulli Lifespan: Shutdown complete.")

mcp = FastMCP(
    name="TautulliMCP",
    instructions="Interact with a Tautulli instance for Plex statistics and activity.",
    lifespan=tautulli_lifespan
)

# --- Tools will be added here ---
@mcp.tool()
async def get_tautulli_activity(ctx: Context) -> Union[Dict, str]:
    """Retrieves current Plex activity from Tautulli."""
    logger.info("Executing tool: get_tautulli_activity")
    client = getattr(ctx.fastmcp, 'tau_client', None)
    if not client:
        return "Error: Tautulli API client is not available. Check server startup logs."

    api_response = await client.get_activity()
    
    if isinstance(api_response, str): # Error string from client
        return api_response
    elif isinstance(api_response, dict):
        logger.info(f"Successfully retrieved Tautulli activity. Stream count: {api_response.get('stream_count', 'N/A')}")
        return api_response 
    else:
        logger.warning(f"Unexpected API response structure from Tautulli get_activity: {api_response}")
        return "Error: Received unexpected data structure from Tautulli for activity."

@mcp.tool()
async def get_tautulli_home_stats(ctx: Context, stats_count: Optional[int] = None) -> Union[Dict, str]:
    """Retrieves home page statistics from Tautulli.
    
    Args:
        stats_count: Optional. The number of items to return for list-based statistics.

    Returns:
        A dictionary containing home statistics or an error string.
    """
    logger.info(f"Executing tool: get_tautulli_home_stats(stats_count={stats_count})")
    client = getattr(ctx.fastmcp, 'tau_client', None)
    if not client:
        return "Error: Tautulli API client is not available. Check server startup logs."

    params = {}
    if stats_count is not None:
        params["stats_count"] = stats_count

    api_response = await client._request(cmd="get_home_stats", params=params if params else None)
    
    if isinstance(api_response, str): # Error string from client
        return api_response
    elif isinstance(api_response, list): # get_home_stats returns a list of stat objects
        logger.info(f"Successfully retrieved Tautulli home stats. Number of stat groups: {len(api_response)}")
        # The response is a list of dictionaries, each representing a stat card (e.g., Most Watched Movie)
        return {"stats": api_response} # Wrap in a dict for a more structured MCP response
    elif isinstance(api_response, dict): # Sometimes it might be a single dict if error or specific case
        logger.info(f"Retrieved Tautulli home stats (single dict): {api_response}")
        return api_response
    else:
        logger.warning(f"Unexpected API response structure from Tautulli get_home_stats: {api_response}")
        return "Error: Received unexpected data structure from Tautulli for home stats."

@mcp.tool()
async def get_tautulli_history(ctx: Context, section_id: Optional[str] = None, length: int = 25) -> Union[Dict, str]:
    """Retrieves watch history from Tautulli, with optional filters for library.

    Args:
        section_id: Optional. Filter history by a specific Plex library ID (section_id from Tautulli).
        length: Optional. Number of history entries to return (default 25).

    Returns:
        A dictionary containing watch history data or an error string.
    """
    logger.info(f"Executing tool: get_tautulli_history(section_id='{section_id}', length={length})")
    client = getattr(ctx.fastmcp, 'tau_client', None)
    if not client:
        return "Error: Tautulli API client is not available. Check server startup logs."

    api_response = await client.get_history(user_id=None, section_id=section_id, length=length)
    
    if isinstance(api_response, str): # Error string from client
        return api_response
    # Tautulli's get_history returns a dict like: { "draw": ..., "recordsTotal": ..., "recordsFiltered": ..., "data": [...] }
    elif isinstance(api_response, dict) and 'data' in api_response:
        logger.info(f"Successfully retrieved Tautulli history. Records: {len(api_response['data'])}")
        return api_response 
    else:
        logger.warning(f"Unexpected API response structure from Tautulli get_history: {api_response}")
        return "Error: Received unexpected data structure from Tautulli for history."

@mcp.tool()
async def get_tautulli_users(ctx: Context) -> Union[List[Dict], str]:
    """Retrieves a list of users known to Tautulli."""
    logger.info("Executing tool: get_tautulli_users")
    client = getattr(ctx.fastmcp, 'tau_client', None)
    if not client:
        return "Error: Tautulli API client is not available. Check server startup logs."

    api_response = await client.get_users()
    
    if isinstance(api_response, str): # Error string from client
        return api_response
    # Tautulli's get_users returns a list of user dicts.
    elif isinstance(api_response, list):
        logger.info(f"Successfully retrieved Tautulli users. Count: {len(api_response)}")
        return api_response 
    else:
        logger.warning(f"Unexpected API response structure from Tautulli get_users: {api_response}")
        return "Error: Received unexpected data structure from Tautulli for users."

def main():
    logger.info(f"Starting Tautulli MCP Server with transport: {TAUTULLI_MCP_TRANSPORT}")
    # Critical check for TAUTULLI_URL and TAUTULLI_API_KEY is at the top of the script.

    if TAUTULLI_MCP_TRANSPORT == "stdio":
        mcp.run()
    elif TAUTULLI_MCP_TRANSPORT == "sse":
        mcp.run(
            transport="sse",
            host=TAUTULLI_MCP_HOST,
            port=TAUTULLI_MCP_PORT,
            path="/mcp"  # Standardized path
        )
    else:
        logger.error(f"Invalid TAUTULLI_MCP_TRANSPORT: '{TAUTULLI_MCP_TRANSPORT}'. Must be 'stdio' or 'sse'. Defaulting to stdio.")
        mcp.run() # Fallback to stdio

if __name__ == "__main__":
    main()
