"""
MCP Server for qBittorrent
Implements a tool set for interacting with a qBittorrent instance.
Built with FastMCP following best practices from gofastmcp.com
Transport: Determined by QBITTORRENT_MCP_TRANSPORT environment variable (stdio or sse)
"""
import asyncio
import os
import sys
from contextlib import asynccontextmanager
from fastmcp import FastMCP, Context
from dotenv import load_dotenv
import logging
from logging.handlers import RotatingFileHandler # For log rotation
from pathlib import Path # Added Path
from typing import Optional, List, Dict, Union, Any
from qbittorrentapi import Client, APIConnectionError, LoginFailed, HTTPError
from concurrent.futures import ThreadPoolExecutor
from fastapi.middleware.cors import CORSMiddleware

# --- Environment Loading & Configuration ---
SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent # Go up two levels for project root

# Load .env from project root first
project_dotenv_path = PROJECT_ROOT / ".env"
if project_dotenv_path.exists():
    load_dotenv(dotenv_path=project_dotenv_path, override=True) # Override to ensure project root takes precedence
    print(f"QbittorrentMCP: Loaded .env from project root: {project_dotenv_path}") # For debugging
else:
    print(f"QbittorrentMCP: No .env file found at project root: {project_dotenv_path}") # For debugging

# Optionally, load .env from script's directory if it exists and is different (e.g., for local dev overrides)
# This part might be less common if the project root .env is the single source of truth.
script_dotenv_path = SCRIPT_DIR / ".env"
if script_dotenv_path.exists() and script_dotenv_path != project_dotenv_path:
    load_dotenv(dotenv_path=script_dotenv_path, override=True) # Or False if project_root should always win
    print(f"QbittorrentMCP: Also loaded .env from script directory: {script_dotenv_path}") # For debugging

# --- Logging Setup ---
QBITTORRENT_LOG_LEVEL_STR = os.getenv('QBITTORRENT_LOG_LEVEL', os.getenv('LOG_LEVEL', 'INFO')).upper()
NUMERIC_LOG_LEVEL = getattr(logging, QBITTORRENT_LOG_LEVEL_STR, logging.INFO)

logger = logging.getLogger("QbittorrentMCPServer")
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
log_file_name_base = os.getenv('QBITTORRENT_NAME', 'qbittorrent').lower()
log_file_name = f"{log_file_name_base}_mcp.log"
log_file_path = SCRIPT_DIR / log_file_name
file_handler = RotatingFileHandler(log_file_path, maxBytes=5*1024*1024, backupCount=3, encoding='utf-8')
file_handler.setLevel(NUMERIC_LOG_LEVEL)
file_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(module)s - %(funcName)s - %(lineno)d - %(message)s')
file_handler.setFormatter(file_formatter)
if not any(isinstance(h, RotatingFileHandler) for h in logger.handlers):
    logger.addHandler(file_handler)

logger.info(f"Logging initialized (console and file: {log_file_path}). Effective log level: {QBITTORRENT_LOG_LEVEL_STR}")

# --- Essential Configuration ---
QBITTORRENT_URL = os.getenv("QBITTORRENT_URL")
QBITTORRENT_USER = os.getenv("QBITTORRENT_USER")
QBITTORRENT_PASS = os.getenv("QBITTORRENT_PASS")

QBITTORRENT_MCP_TRANSPORT = os.getenv("QBITTORRENT_MCP_TRANSPORT", "sse").lower()
QBITTORRENT_MCP_HOST = os.getenv("QBITTORRENT_MCP_HOST", "0.0.0.0")
QBITTORRENT_MCP_PORT = int(os.getenv("QBITTORRENT_MCP_PORT", "8000")) # Default to 8000 if not specified

# Now log these values after they are defined
logger.info(f"QBITTORRENT_MCP_TRANSPORT set to: {QBITTORRENT_MCP_TRANSPORT}")
logger.info(f"QBITTORRENT_MCP_HOST set to: {QBITTORRENT_MCP_HOST}")
logger.info(f"QBITTORRENT_MCP_PORT set to: {QBITTORRENT_MCP_PORT}")
logger.info(f"QBITTORRENT_LOG_LEVEL (effective): {QBITTORRENT_LOG_LEVEL_STR}")
logger.info(f"QBITTORRENT_NAME for log file (effective): {log_file_name_base}")

logger.info(f"QBITTORRENT_URL loaded: {'Yes' if QBITTORRENT_URL else 'No'}")
logger.info(f"QBITTORRENT_USER loaded: {'Yes' if QBITTORRENT_USER else 'No'}")
logger.info(f"QBITTORRENT_PASS loaded: {'****' if QBITTORRENT_PASS else 'No'}")

if not all([QBITTORRENT_URL, QBITTORRENT_USER, QBITTORRENT_PASS]):
    logger.error("QBITTORRENT_URL, QBITTORRENT_USER, and QBITTORRENT_PASS must all be set in environment variables. Server cannot start.")
    sys.exit(1)

# Thread pool for running synchronous qbittorrent-api calls
executor = ThreadPoolExecutor(max_workers=3)

@asynccontextmanager
async def qbittorrent_lifespan(app: FastMCP):
    logger.info("qBittorrent Lifespan: Startup sequence initiated.")
    app.qb_client = None # Initialize to None
    if QBITTORRENT_URL and QBITTORRENT_USER and QBITTORRENT_PASS: # Already checked above, but good for clarity
        try:
            qb_client_instance = Client(
                host=QBITTORRENT_URL,
                username=QBITTORRENT_USER,
                password=QBITTORRENT_PASS,
            )
            logger.info("qBittorrent Lifespan: Attempting to login...")
            await asyncio.get_event_loop().run_in_executor(executor, qb_client_instance.auth_log_in)
            logger.info(f"qBittorrent Lifespan: Successfully logged in. API Version: {qb_client_instance.app_version()}, qBT Version: {qb_client_instance.app_web_api_version()}")
            app.qb_client = qb_client_instance
        except LoginFailed:
            logger.error("qBittorrent Lifespan Error: Login failed. Check credentials.")
        except APIConnectionError as e:
            logger.error(f"qBittorrent Lifespan Error: API Connection failed: {e}")
        except HTTPError as e:
            logger.error(f"qBittorrent Lifespan Error: HTTP Error during login: {e}")
        except Exception as e:
            logger.error(f"qBittorrent Lifespan Error: Unexpected error during client init/login: {e}", exc_info=True)
    else: # Should not be reached due to sys.exit above
        logger.error("qBittorrent Lifespan Error: URL, User, or Pass missing. Client not initialized.")
    
    logger.info("qBittorrent Lifespan: Startup complete.")
    yield
    
    logger.info("qBittorrent Lifespan: Shutdown sequence initiated.")
    if hasattr(app, 'qb_client') and app.qb_client:
        logger.info("qBittorrent Lifespan: Client session presumed managed internally.")
    logger.info("qBittorrent Lifespan: Shutdown complete.")

mcp = FastMCP(
    name="qBittorrent MCP Server",
    instructions="Interact with a qBittorrent instance for managing torrents.",
    lifespan=qbittorrent_lifespan
)

@mcp.tool()
async def list_torrents(ctx: Context, filter: Optional[str] = 'all', category: Optional[str] = None, tag: Optional[str] = None) -> Union[List[Dict], str]:
    """Lists torrents from qBittorrent with optional filters.

    Args:
        filter: Optional. Filter torrents by status (e.g., 'all', 'downloading', 'seeding', 'completed', 'paused', 'active', 'inactive', 'errored'). Default: 'all'.
        category: Optional. Filter by category name.
        tag: Optional. Filter by tag.

    Returns:
        A list of torrent information dictionaries or an error string.
    """
    logger.info(f"Executing tool: list_torrents(filter='{filter}', category='{category}', tag='{tag}')")
    qb_client = getattr(ctx.fastmcp, 'qb_client', None)
    if not qb_client or not qb_client.is_logged_in:
        logger.error("list_torrents: qBittorrent client is not available or not logged in.")
        return "Error: qBittorrent client is not available or not logged in. Check server startup logs."

    try:
        api_params = {}
        if filter and filter != 'all':
            api_params['filter'] = filter
        if category:
            api_params['category'] = category
        if tag:
            api_params['tag'] = tag
        
        torrents_list = await run_sync_qb_tool(qb_client.torrents_info, **api_params)
        
        results = []
        for torrent in torrents_list:
            results.append({
                "hash": torrent.hash,
                "name": torrent.name,
                "size_gb": round(torrent.size / (1024**3), 2),
                "progress_percent": round(torrent.progress * 100, 2),
                "status": torrent.state,
                "category": torrent.category,
                "tags": list(torrent.tags),
                "ratio": round(torrent.ratio, 2),
                "eta_seconds": torrent.eta,
                "num_seeds": torrent.num_seeds,
                "num_leechers": torrent.num_leechs,
                "added_on_timestamp": torrent.added_on,
                "save_path": torrent.save_path
            })
        
        if not results:
            logger.info(f"No torrents found matching criteria (filter: {filter}, category: {category}, tag: {tag}).")
            return f"No torrents found matching criteria (filter: {filter}, category: {category}, tag: {tag})."
        logger.info(f"Found {len(results)} torrents matching criteria.")
        return results

    except APIConnectionError as e:
        logger.error(f"qBittorrent API connection error in list_torrents: {e}")
        return f"Error: qBittorrent API connection error: {e}"
    except HTTPError as e:
        logger.error(f"qBittorrent HTTP error in list_torrents: {e}")
        return f"Error: qBittorrent HTTP error: {e}"
    except Exception as e:
        logger.error(f"Unexpected error in list_torrents: {e}", exc_info=True)
        return f"Error: An unexpected error occurred while listing torrents: {e}"

@mcp.tool()
async def add_torrent_url(
    ctx: Context, 
    torrent_url: str, 
    save_path: Optional[str] = None, 
    category: Optional[str] = None, 
    tags: Optional[List[str]] = None, 
    is_paused: Optional[bool] = False,
    upload_limit_kib: Optional[int] = None, 
    download_limit_kib: Optional[int] = None
) -> str:
    """Adds a new torrent to qBittorrent using a URL.

    Args:
        torrent_url: URL of the .torrent file or a magnet link.
        save_path: Optional. Path where the torrent should be saved.
        category: Optional. Category to assign to the torrent.
        tags: Optional. List of tags to assign.
        is_paused: Optional. If True, add torrent in a paused state. Default False.
        upload_limit_kib: Optional. Upload speed limit in KiB/s.
        download_limit_kib: Optional. Download speed limit in KiB/s.

    Returns:
        A success or error message string.
    """
    logger.info(f"Executing tool: add_torrent_url(url='{torrent_url}', category='{category}', tags={tags}, paused={is_paused})")
    qb_client = getattr(ctx.fastmcp, 'qb_client', None)
    if not qb_client or not qb_client.is_logged_in:
        logger.error("add_torrent_url: qBittorrent client is not available or not logged in.")
        return "Error: qBittorrent client is not available or not logged in. Check server startup logs."

    add_params = {
        "urls": torrent_url,
        "savepath": save_path,
        "category": category,
        "tags": tags,
        "paused": str(is_paused).lower(),
        "upLimit": upload_limit_kib * 1024 if upload_limit_kib is not None else None,
        "dlLimit": download_limit_kib * 1024 if download_limit_kib is not None else None
    }
    add_params = {k: v for k, v in add_params.items() if v is not None}

    try:
        result = await run_sync_qb_tool(qb_client.torrents_add, **add_params)
        
        if isinstance(result, str) and result.lower().strip() == "ok.":
            logger.info(f"Successfully added torrent: {torrent_url}")
            return f"Torrent added successfully: {torrent_url}"
        elif result is None:
             logger.info(f"Successfully added torrent (assumed by None response): {torrent_url}")
             return f"Torrent added successfully (assumed by None response): {torrent_url}"
        else:
            logger.warning(f"Torrent add for {torrent_url} returned: {result}. Assuming success if no exception.")
            return f"Torrent add command sent for {torrent_url}. Result: {result}"

    except APIConnectionError as e:
        logger.error(f"qBittorrent API connection error in add_torrent_url: {e}")
        return f"Error: qBittorrent API connection error: {e}"
    except HTTPError as e:
        logger.error(f"qBittorrent API error in add_torrent_url: {e}")
        return f"Error: qBittorrent API error - {e.description if hasattr(e, 'description') else str(e)}"
    except Exception as e:
        logger.error(f"Unexpected error in add_torrent_url: {e}", exc_info=True)
        return f"Error: An unexpected error occurred while adding torrent: {e}"

@mcp.tool()
async def pause_torrent(ctx: Context, torrent_hash: str) -> str:
    """Pauses a specific torrent.

    Args:
        torrent_hash: The hash of the torrent to pause.

    Returns:
        A success or error message string.
    """
    logger.info(f"Executing tool: pause_torrent(hash='{torrent_hash}')")
    qb_client = getattr(ctx.fastmcp, 'qb_client', None)
    if not qb_client or not qb_client.is_logged_in:
        logger.error("pause_torrent: qBittorrent client is not available or not logged in.")
        return "Error: qBittorrent client is not available or not logged in. Check server startup logs."

    try:
        result = await run_sync_qb_tool(qb_client.torrents_pause, torrent_hashes=torrent_hash)
        logger.info(f"Pause command sent for torrent hash: {torrent_hash}. Result: {result}")
        return f"Pause command sent for torrent: {torrent_hash}."
    except APIConnectionError as e:
        logger.error(f"qBittorrent API connection error in pause_torrent: {e}")
        return f"Error: qBittorrent API connection error: {e}"
    except HTTPError as e:
        logger.error(f"qBittorrent API error in pause_torrent: {e}")
        return f"Error: qBittorrent API error - {e.description if hasattr(e, 'description') else str(e)}"
    except Exception as e:
        logger.error(f"Unexpected error in pause_torrent: {e}", exc_info=True)
        return f"Error: An unexpected error occurred while pausing torrent: {e}"

@mcp.tool()
async def resume_torrent(ctx: Context, torrent_hash: str) -> str:
    """Resumes a specific torrent.

    Args:
        torrent_hash: The hash of the torrent to resume.

    Returns:
        A success or error message string.
    """
    logger.info(f"Executing tool: resume_torrent(hash='{torrent_hash}')")
    qb_client = getattr(ctx.fastmcp, 'qb_client', None)
    if not qb_client or not qb_client.is_logged_in:
        logger.error("resume_torrent: qBittorrent client is not available or not logged in.")
        return "Error: qBittorrent client is not available or not logged in. Check server startup logs."

    try:
        result = await run_sync_qb_tool(qb_client.torrents_resume, torrent_hashes=torrent_hash)
        logger.info(f"Resume command sent for torrent hash: {torrent_hash}. Result: {result}")
        return f"Resume command sent for torrent: {torrent_hash}."
    except APIConnectionError as e:
        logger.error(f"qBittorrent API connection error in resume_torrent: {e}")
        return f"Error: qBittorrent API connection error: {e}"
    except HTTPError as e:
        logger.error(f"qBittorrent API error in resume_torrent: {e}")
        return f"Error: qBittorrent API error - {e.description if hasattr(e, 'description') else str(e)}"
    except Exception as e:
        logger.error(f"Unexpected error in resume_torrent: {e}", exc_info=True)
        return f"Error: An unexpected error occurred while resuming torrent: {e}"

@mcp.tool()
async def get_qb_transfer_info(ctx: Context) -> Union[Dict, str]:
    """Retrieves global transfer information from qBittorrent."""
    logger.info(f"Executing tool: get_qb_transfer_info")
    qb_client = getattr(ctx.fastmcp, 'qb_client', None)
    if not qb_client or not qb_client.is_logged_in:
        logger.error("get_qb_transfer_info: qBittorrent client is not available or not logged in.")
        return "Error: qBittorrent client is not available or not logged in. Check server startup logs."

    try:
        info = await run_sync_qb_tool(qb_client.transfer_info)
        return dict(info)
    except APIConnectionError as e:
        logger.error(f"qBittorrent API connection error in get_qb_transfer_info: {e}")
        return f"Error: qBittorrent API connection error: {e}"
    except HTTPError as e:
        logger.error(f"qBittorrent API error in get_qb_transfer_info: {e}")
        return f"Error: qBittorrent API error - {e.description if hasattr(e, 'description') else str(e)}"
    except Exception as e:
        logger.error(f"Unexpected error in get_qb_transfer_info: {e}", exc_info=True)
        return f"Error: An unexpected error occurred: {e}"

@mcp.tool()
async def get_qb_app_preferences(ctx: Context) -> Union[Dict, str]:
    """Retrieves qBittorrent application preferences."""
    logger.info(f"Executing tool: get_qb_app_preferences")
    qb_client = getattr(ctx.fastmcp, 'qb_client', None)
    if not qb_client or not qb_client.is_logged_in:
        logger.error("get_qb_app_preferences: qBittorrent client is not available or not logged in.")
        return "Error: qBittorrent client is not available or not logged in. Check server startup logs."

    try:
        prefs = await run_sync_qb_tool(qb_client.app_preferences)
        return dict(prefs)
    except APIConnectionError as e:
        logger.error(f"qBittorrent API connection error in get_qb_app_preferences: {e}")
        return f"Error: qBittorrent API connection error: {e}"
    except HTTPError as e:
        logger.error(f"qBittorrent API error in get_qb_app_preferences: {e}")
        return f"Error: qBittorrent API error - {e.description if hasattr(e, 'description') else str(e)}"
    except Exception as e:
        logger.error(f"Unexpected error in get_qb_app_preferences: {e}", exc_info=True)
        return f"Error: An unexpected error occurred: {e}"

async def run_sync_qb_tool(func, *args, **kwargs):
    """Helper to run synchronous qb client methods in executor."""
    loop = asyncio.get_event_loop()
    return await loop.run_in_executor(executor, lambda: func(*args, **kwargs))

def main():
    # The critical check for URL, USER, PASS is done globally now and will sys.exit if they are missing.
    # So, the print statement below is redundant.
    # if not all([QBITTORRENT_URL, QBITTORRENT_USER, QBITTORRENT_PASS]):
    #     print("CRITICAL: QBITTORRENT_URL, QBITTORRENT_USER, or QBITTORRENT_PASS are not set. Server cannot start effectively.", file=sys.stderr)
    
    logger.info(f"Starting qBittorrent MCP Server with transport: {QBITTORRENT_MCP_TRANSPORT}")
    if QBITTORRENT_MCP_TRANSPORT == "stdio":
        mcp.run()
    elif QBITTORRENT_MCP_TRANSPORT == "sse":
        mcp.run(
            transport="sse",
            host=QBITTORRENT_MCP_HOST,
            port=QBITTORRENT_MCP_PORT,
            path="/mcp"
        )
    else:
        logger.error(f"Invalid QBITTORRENT_MCP_TRANSPORT: '{QBITTORRENT_MCP_TRANSPORT}'. Must be 'stdio' or 'sse'. Defaulting to stdio.")
        mcp.run()

if __name__ == "__main__":
    main() 