import asyncio
import os
import sys
from contextlib import asynccontextmanager
from fastmcp import FastMCP, Context
from plexapi.server import PlexServer
from plexapi.exceptions import NotFound, Unauthorized, BadRequest
from typing import List, Dict, Union, Optional, Any
from dotenv import load_dotenv
from pathlib import Path
import logging
from logging.handlers import RotatingFileHandler # For log rotation
from fastapi.middleware.cors import CORSMiddleware # Added for CORS

# --- Configuration Loading ---
# Explicitly find .env in the project root (assuming server.py is in src/mcplex)
project_root = Path(__file__).resolve().parent.parent.parent
env_path = project_root / '.env'
print(f"DEBUG: Looking for .env file at: {env_path}") # Keep for initial early debug

found_dotenv = load_dotenv(dotenv_path=env_path, override=True)
print(f"DEBUG: load_dotenv found file: {found_dotenv}") # Keep for initial early debug

PLEX_URL = os.getenv("PLEX_URL")
PLEX_TOKEN = os.getenv("PLEX_TOKEN")
# SSE/Transport Config
PLEX_MCP_TRANSPORT = os.getenv("PLEX_MCP_TRANSPORT", "sse").lower()
PLEX_MCP_HOST = os.getenv("PLEX_MCP_HOST", "0.0.0.0")
PLEX_MCP_PORT = int(os.getenv("PLEX_MCP_PORT", "8000"))
PLEX_LOG_LEVEL = os.getenv('PLEX_LOG_LEVEL', 'INFO').upper()

# --- Enhanced Logging Configuration (as per create-mcp-server_v2.md) ---
NUMERIC_LOG_LEVEL = getattr(logging, PLEX_LOG_LEVEL, logging.INFO)
SCRIPT_DIR = Path(__file__).resolve().parent

# Define a base logger
logger = logging.getLogger("PlexMCPServer")
logger.setLevel(NUMERIC_LOG_LEVEL)
logger.propagate = False

# Console Handler
console_handler = logging.StreamHandler(sys.stdout)
console_handler.setLevel(NUMERIC_LOG_LEVEL)
console_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
console_handler.setFormatter(console_formatter)
logger.addHandler(console_handler)

# File Handler with Rotation
log_file_name = "plex_mcp.log" # Service name is "Plex"
log_file_path = SCRIPT_DIR / log_file_name

file_handler = RotatingFileHandler(log_file_path, maxBytes=5*1024*1024, backupCount=3, encoding='utf-8')
file_handler.setLevel(NUMERIC_LOG_LEVEL)
file_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(module)s - %(funcName)s - %(lineno)d - %(message)s')
file_handler.setFormatter(file_formatter)
logger.addHandler(file_handler)

logger.info(f"Logging initialized (console and file: {log_file_path}).")
logger.info(f"PLEX_URL loaded: {'Yes' if PLEX_URL else 'No'}")
logger.info(f"PLEX_TOKEN loaded: {'Yes' if PLEX_TOKEN and len(PLEX_TOKEN) > 5 else 'No'}")
logger.info(f"PLEX_MCP_TRANSPORT set to: {PLEX_MCP_TRANSPORT}")
logger.info(f"PLEX_MCP_HOST set to: {PLEX_MCP_HOST}")
logger.info(f"PLEX_MCP_PORT set to: {PLEX_MCP_PORT}")
logger.info(f"PLEX_LOG_LEVEL (effective): {PLEX_LOG_LEVEL} ({NUMERIC_LOG_LEVEL})")


# --- Critical Check for Essential Config ---
if not PLEX_URL or not PLEX_TOKEN:
    logger.error("PLEX_URL and PLEX_TOKEN must be set in environment variables. Server cannot start.")
    sys.exit(1)

# --- Lifespan Management for Plex Connection ---
@asynccontextmanager
async def plex_lifespan(app: FastMCP):
    logger.info("Lifespan: Startup sequence initiated.")
    if not PLEX_URL or not PLEX_TOKEN: # Redundant check due to above, but good for lifespan context
        logger.error("Lifespan Error: PLEX_URL and PLEX_TOKEN must be set. Server cannot connect.")
        app.plex_server = None
    else:
        try:
            logger.info(f"Lifespan: Attempting to connect to Plex server at {PLEX_URL}...")
            app.plex_server = PlexServer(PLEX_URL, PLEX_TOKEN)
            server_name = app.plex_server.friendlyName
            logger.info(f"Lifespan: Successfully connected to Plex server: {server_name} (Version: {app.plex_server.version})")
        except (Unauthorized, NotFound, Exception) as e:
            error_type = type(e).__name__
            logger.error(f"Lifespan Error: Failed to connect to Plex during startup ({error_type}). Server will run without connection. Details: {e}", exc_info=True)
            app.plex_server = None
            
    logger.info("Lifespan: Startup complete.")
    yield
    logger.info("Lifespan: Shutdown sequence initiated.")
    if hasattr(app, 'plex_server') and app.plex_server:
         logger.info("Lifespan: Plex connection considered closed (no specific close method).")
    logger.info("Lifespan: Shutdown complete.")

# Instantiate the FastMCP server with the lifespan context
mcp = FastMCP(
    name="Plex MCP Server",
    instructions="Interact with a Plex Media Server using the plexapi library.",
    lifespan=plex_lifespan
)

# --- CORS Configuration for MCP Server --- Added
mcp_origins = [
    "http://localhost:5173",
    "http://127.0.0.1:5173",
]
mcp.add_middleware(
    CORSMiddleware,
    allow_origins=mcp_origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
# --- End CORS Configuration ---

# --- Helper for Connection Check (Uses context state) ---
def _get_plex_server_from_app_state(app: FastMCP) -> Optional[PlexServer]:
    """Safely retrieves the PlexServer instance from the FastMCP app state."""
    return getattr(app, 'plex_server', None)

# --- Tools (Now use injected Context) ---

@mcp.tool()
def get_libraries(ctx: Context) -> Union[List[str], str]:
    """Retrieves a list of all library section names from the connected Plex server."""
    logger.info(f"Executing tool: get_libraries") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("get_libraries: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."
    
    try:
        libraries = plex_server.library.sections()
        library_names = [section.title for section in libraries]
        if not library_names:
            logger.warn("get_libraries: No libraries found on the Plex server.") # Changed log to logger
            return "No libraries found on the Plex server."
        logger.info(f"get_libraries: Found libraries: {library_names}") # Changed log to logger
        return library_names
    except Exception as e:
        logger.error(f"Error retrieving libraries: {e}", exc_info=True) # Changed log to logger, added exc_info
        return f"Error: Failed to retrieve libraries from Plex. Details: {e}"

@mcp.tool()
def search_library(ctx: Context, query: str, library_name: Optional[str] = None) -> Union[List[Dict], str]:
    """Searches for media items.

    Searches within a specific library if library_name is provided, otherwise searches
    across all libraries on the connected Plex server.

    Args:
        query: The search term (e.g., movie title, show name, artist).
        library_name: Optional name of the library section to limit the search.

    Returns:
        A list of dictionaries, each containing basic info ('title', 'type', 'year', 'summary', 'library') 
        for a found media item, or an error string if the search fails or yields no results.
    """
    logger.info(f"Executing tool: search_library(query='{query}', library_name='{library_name}')") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
         logger.error("search_library: Plex server is not connected.") # Changed log to logger
         return "Error: Plex server is not connected. Check configuration and server startup logs."

    results = []
    try:
        if library_name:
            try:
                logger.debug(f"Searching in library: {library_name}") # Changed log to logger
                section = plex_server.library.section(library_name)
                search_results = section.search(query)
            except NotFound:
                logger.warn(f"search_library: Library '{library_name}' not found.") # Changed log to logger
                return f"Error: Library '{library_name}' not found."
        else:
            logger.debug(f"Searching all libraries for: {query}") # Changed log to logger
            search_results = plex_server.search(query)
        
        for item in search_results:
            item_info = {
                "title": getattr(item, 'title', 'N/A'),
                "type": getattr(item, 'type', 'N/A'),
                "year": getattr(item, 'year', None),
                "summary": getattr(item, 'summary', None)
            }
            if hasattr(item, 'librarySectionTitle'):
                 item_info["library"] = item.librarySectionTitle
            results.append(item_info)
            
        if not results:
            logger.info(f"No results found for '{query}'" + (f" in library '{library_name}'" if library_name else "")) # Changed log to logger
            return f"No results found for '{query}'" + (f" in library '{library_name}'" if library_name else "") + "."
        
        logger.info(f"search_library: Found {len(results)} items for query '{query}'.") # Changed log to logger
        return results

    except Exception as e:
        logger.error(f"Error during search for '{query}': {e}", exc_info=True) # Changed log to logger, added exc_info
        return f"Error: Failed to perform search on Plex. Details: {e}"

@mcp.tool()
def play_media(ctx: Context, item_title: str, client_name: str) -> str:
    """Plays a media item identified by its title on a specified Plex client by its name.

    Note: This plays the *first* item found matching the title.

    Args:
        item_title: The exact title of the movie, episode, track, etc., to play.
        client_name: The name of the target Plex client (case-sensitive).

    Returns:
        A success message string upon successful playback initiation, or an error string.
    """
    logger.info(f"Executing tool: play_media(item_title='{item_title}', client_name='{client_name}')") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("play_media: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."

    try:
        logger.debug(f"Searching for media item: {item_title}") # Changed log to logger
        search_results = plex_server.search(item_title)
        if not search_results:
            logger.warn(f"play_media: Media item title '{item_title}' not found.") # Changed log to logger
            return f"Error: Media item title '{item_title}' not found."
        item_to_play = search_results[0]
        logger.info(f"Found media item: {getattr(item_to_play, 'title', 'Unknown Title')}") # Changed log to logger

        logger.debug(f"Searching for client: {client_name}") # Changed log to logger
        client = plex_server.client(client_name)
        logger.info(f"Found client: {getattr(client, 'title', 'Unknown Client')}") # Changed log to logger

        logger.info(f"Attempting to play '{getattr(item_to_play, 'title', '?')}' on '{getattr(client, 'title', '?')}'") # Changed log to logger
        client.playMedia(item_to_play)
        return f"Command sent to play '{getattr(item_to_play, 'title', '?')}' on '{getattr(client, 'title', '?')}'."
    
    except NotFound:
        logger.warn(f"play_media: Plex client '{client_name}' not found or media '{item_title}' not found.") # Changed log to logger
        return f"Error: Plex client '{client_name}' not found or is not available, or media title '{item_title}' not found."
    except BadRequest as e:
        logger.error(f"BadRequest during play_media for '{item_title}' on '{client_name}': {e}", exc_info=True) # Changed log to logger, added exc_info
        return f"Error: Bad request trying to play media. Details: {e}"
    except Exception as e:
        logger.error(f"Error during play_media for '{item_title}' on '{client_name}': {e}", exc_info=True) # Changed log to logger, added exc_info
        return f"Error: Unexpected failure trying to play media on Plex. Details: {e}"

@mcp.tool()
def get_server_info(ctx: Context) -> Union[Dict, str]:
    """Retrieves basic information and status about the connected Plex server."""
    logger.info(f"Executing tool: get_server_info") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("get_server_info: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."
    
    try:
        info = {
            "friendlyName": getattr(plex_server, 'friendlyName', 'N/A'),
            "version": getattr(plex_server, 'version', 'N/A'),
            "platform": getattr(plex_server, 'platform', 'N/A'),
            "platformVersion": getattr(plex_server, 'platformVersion', 'N/A'),
            "activeSessions": plex_server.transcoderActiveVideoSessions if hasattr(plex_server, 'transcoderActiveVideoSessions') else 0, # Corrected access
            "myPlexUsername": plex_server.myPlexUsername if hasattr(plex_server, 'myPlexUsername') else None
        }
        logger.info(f"get_server_info: Retrieved server info: {info['friendlyName']}") # Changed log to logger
        return info
    except Exception as e:
        logger.error(f"Error retrieving server info: {e}", exc_info=True) # Changed log to logger, added exc_info
        return f"Error: Failed to retrieve server info from Plex. Details: {e}"

@mcp.tool()
def list_clients(ctx: Context) -> Union[List[Dict], str]:
    """Lists available Plex clients connected to the server."""
    logger.info(f"Executing tool: list_clients") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("list_clients: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."

    try:
        clients = plex_server.clients()
        client_info = [
            {
                "name": getattr(client, 'title', 'N/A'),
                "product": getattr(client, 'product', 'N/A'),
                "platform": getattr(client, 'platform', 'N/A'),
                "device": getattr(client, 'device', None),
                "address": getattr(client, 'address', None),
                "port": getattr(client, 'port', None)
            }
            for client in clients
        ]
        if not client_info:
            logger.info("list_clients: No clients found connected to the server.") # Changed log to logger
            return "No clients found connected to the server."
        logger.info(f"list_clients: Found {len(client_info)} clients.") # Changed log to logger
        return client_info
    except Exception as e:
        logger.error(f"Error listing clients: {e}", exc_info=True)
        return f"Error: Failed to list clients. Details: {e}"

@mcp.tool()
def get_active_sessions(ctx: Context) -> Union[List[Dict], str]:
    """Retrieves information about current playback sessions on the server."""
    logger.info(f"Executing tool: get_active_sessions") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("get_active_sessions: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."
    
    try:
        sessions = plex_server.sessions()
        session_info = []
        for session in sessions:
            user_title = getattr(session.user, 'title', 'Unknown User')
            player_title = getattr(session.player, 'title', 'Unknown Player')
            session_info.append({
                "user": user_title,
                "client": player_title,
                "media_title": getattr(session, 'title', 'N/A'),
                "state": getattr(session, 'state', 'N/A'),
                "progress_ms": getattr(session, 'viewOffset', 0),
                "media_type": getattr(session, 'type', 'N/A')
            })
        
        if not session_info:
            logger.info("get_active_sessions: No active playback sessions found.") # Changed log to logger
            return "No active playback sessions found."
        logger.info(f"get_active_sessions: Found {len(session_info)} active sessions.") # Changed log to logger
        return session_info
    except Exception as e:
        logger.error(f"Error getting active sessions: {e}", exc_info=True)
        return f"Error: Failed to get active sessions. Details: {e}"

@mcp.tool()
def control_playback(ctx: Context, client_name: str, action: str, offset_ms: Optional[int] = None) -> str:
    """Controls playback on a specific client.

    Args:
        client_name: The name of the target Plex client.
        action: The action to perform: 'play', 'pause', 'stop', 'seek'.
        offset_ms: Required for 'seek' action. Playback position in milliseconds.

    Returns:
        A success or error message string.
    """
    logger.info(f"Executing tool: control_playback(client_name='{client_name}', action='{action}', offset_ms={offset_ms})") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("control_playback: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."

    valid_actions = ['play', 'pause', 'stop', 'seek']
    if action not in valid_actions:
        logger.warn(f"control_playback: Invalid action '{action}'.") # Changed log to logger
        return f"Error: Invalid action '{action}'. Must be one of: {valid_actions}"
    
    if action == 'seek' and offset_ms is None:
        logger.warn("control_playback: offset_ms parameter is required for 'seek' action.") # Changed log to logger
        return "Error: offset_ms parameter is required for 'seek' action."
        
    try:
        client = plex_server.client(client_name)
        logger.info(f"Found client '{client.title}' for playback control.") # Changed log to logger

        if action == 'play':
            client.play()
            logger.info(f"Sent 'play' command to '{client_name}'.") # Changed log to logger
            return f"Sent 'play' command to '{client_name}'."
        elif action == 'pause':
            client.pause()
            logger.info(f"Sent 'pause' command to '{client_name}'.") # Changed log to logger
            return f"Sent 'pause' command to '{client_name}'."
        elif action == 'stop':
            client.stop()
            logger.info(f"Sent 'stop' command to '{client_name}'.") # Changed log to logger
            return f"Sent 'stop' command to '{client_name}'."
        elif action == 'seek':
            if offset_ms is not None: 
              client.seekTo(offset_ms)
              logger.info(f"Sent 'seek' command to {offset_ms}ms for '{client_name}'.") # Changed log to logger
              return f"Sent 'seek' command to {offset_ms}ms for '{client_name}'."
            else:
              logger.error("control_playback: Internal error - offset_ms is None for seek action.") # Changed log to logger
              return "Error: Internal error - offset_ms is None for seek action."
        else:
             logger.error(f"control_playback: Internal error - Unhandled action '{action}'.") # Changed log to logger
             return f"Error: Internal error - Unhandled action '{action}'." 

    except NotFound:
        logger.warn(f"control_playback: Plex client '{client_name}' not found.") # Changed log to logger
        return f"Error: Plex client '{client_name}' not found or is not available."
    except Exception as e:
        logger.error(f"Error controlling playback on '{client_name}': {e}", exc_info=True)
        return f"Error: Failed to control playback. Details: {e}"

@mcp.tool()
def get_recently_added(ctx: Context, library_name: str, limit: Optional[int] = 10) -> Union[List[Dict], str]:
    """Retrieves a list of recently added items from a specific library.

    Args:
        library_name: The name of the library to fetch recently added items from.
        limit: Optional maximum number of items to return (default is 10).

    Returns:
        A list of dictionaries, each representing a recently added media item,
        or an error string.
    """
    logger.info(f"Executing tool: get_recently_added(library_name='{library_name}', limit={limit})") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("get_recently_added: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."

    if limit is not None and limit <= 0:
        logger.warn("get_recently_added: Limit must be a positive integer if provided.") # Changed log to logger
        return "Error: Limit must be a positive integer if provided."

    results = []
    try:
        section = plex_server.library.section(library_name)
        recently_added_items = section.recentlyAdded(maxresults=limit)
        
        for item in recently_added_items:
            item_info = {
                "title": getattr(item, 'title', 'N/A'),
                "type": getattr(item, 'type', 'N/A'),
                "year": getattr(item, 'year', None),
                "summary": getattr(item, 'summary', None),
                "added_at": getattr(item, 'addedAt', None).isoformat() if hasattr(item, 'addedAt') and getattr(item, 'addedAt', None) else None
            }
            results.append(item_info)
            
        if not results:
            logger.info(f"get_recently_added: No recently added items found in library '{library_name}'.") # Changed log to logger
            return f"No recently added items found in library '{library_name}'."
        
        logger.info(f"get_recently_added: Found {len(results)} recently added items in '{library_name}'.") # Changed log to logger
        return results

    except NotFound:
        logger.warn(f"get_recently_added: Library '{library_name}' not found.") # Changed log to logger
        return f"Error: Library '{library_name}' not found."
    except Exception as e:
        logger.error(f"Error retrieving recently added for library '{library_name}': {e}", exc_info=True)
        return f"Error: Failed to retrieve recently added items. Details: {e}"

@mcp.tool()
def get_library_size(ctx: Context, library_name: str) -> Union[Dict, str]:
    """Retrieves the total number of items in a specific library.

    Args:
        library_name: The name of the library to get the size of.

    Returns:
        A dictionary with library name and item count, or an error string.
    """
    logger.info(f"Executing tool: get_library_size(library_name='{library_name}')") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("get_library_size: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."

    try:
        section = plex_server.library.section(library_name)
        item_count = section.totalSize 
        result = {
            "library_name": library_name,
            "item_count": item_count
        }
        logger.info(f"get_library_size: Library '{library_name}' has {item_count} items.") # Changed log to logger
        return result
    except NotFound:
        logger.warn(f"get_library_size: Library '{library_name}' not found.") # Changed log to logger
        return f"Error: Library '{library_name}' not found."
    except Exception as e:
        logger.error(f"Error retrieving size for library '{library_name}': {e}", exc_info=True)
        return f"Error: Failed to retrieve library size. Details: {e}"

@mcp.tool()
def list_all_library_titles(ctx: Context, library_name: str) -> Union[List[str], str]:
    """Retrieves a list of all item titles from a specific library.

    Args:
        library_name: The name of the library to list all item titles from.

    Returns:
        A list of strings, where each string is the title of an item in the library,
        or an error string.
    """
    logger.info(f"Executing tool: list_all_library_titles(library_name='{library_name}')") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("list_all_library_titles: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."

    try:
        section = plex_server.library.section(library_name)
        all_items = section.all()
        
        titles = [getattr(item, 'title', 'N/A') for item in all_items]
            
        if not titles:
            logger.info(f"list_all_library_titles: No items found in library '{library_name}'.") # Changed log to logger
            return f"No items found in library '{library_name}'."
        
        logger.info(f"Returning {len(titles)} titles from library '{library_name}'.")
        return titles

    except NotFound:
        logger.warn(f"list_all_library_titles: Library '{library_name}' not found.") # Changed log to logger
        return f"Error: Library '{library_name}' not found."
    except Exception as e:
        logger.error(f"Error listing all titles for library '{library_name}': {e}", exc_info=True)
        return f"Error: Failed to list all titles. Details: {e}"

@mcp.tool()
def get_library_episodes_count(ctx: Context, library_name: str) -> Union[Dict, str]:
    """Retrieves the total number of episodes in a TV library.

    Args:
        library_name: The name of the TV library to get the episode count from.

    Returns:
        A dictionary with library name and episode count, or an error string.
    """
    logger.info(f"Executing tool: get_library_episodes_count(library_name='{library_name}')") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("get_library_episodes_count: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."

    try:
        section = plex_server.library.section(library_name)
        if section.type != 'show':
            logger.warn(f"get_library_episodes_count: Library '{library_name}' is not a TV show library (type: {section.type}).") # Changed log to logger
            return f"Error: Library '{library_name}' is not a TV show library."
        
        episode_count = section.totalViewSize(libtype='episode')
        
        result = {
            "library_name": library_name,
            "episode_count": episode_count
        }
        logger.info(f"get_library_episodes_count: Library '{library_name}' has {episode_count} episodes.") # Changed log to logger
        return result
    except NotFound:
        logger.warn(f"get_library_episodes_count: Library '{library_name}' not found.") # Changed log to logger
        return f"Error: Library '{library_name}' not found."
    except Exception as e:
        logger.error(f"Error retrieving episode count for library '{library_name}': {e}", exc_info=True)
        return f"Error: Failed to retrieve episode count. Details: {e}"

@mcp.tool()
def get_music_library_stats(ctx: Context, library_name: str) -> Union[Dict, str]:
    """Retrieves statistics for a music library including artists, albums, and tracks counts.

    Args:
        library_name: The name of the music library to get statistics for.

    Returns:
        A dictionary with library name and counts for artists, albums, and tracks,
        or an error string.
    """
    logger.info(f"Executing tool: get_music_library_stats(library_name='{library_name}')") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("get_music_library_stats: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."

    try:
        section = plex_server.library.section(library_name)
        if section.type != 'artist': # Music libraries are of type 'artist'
            logger.warn(f"get_music_library_stats: Library '{library_name}' is not a music library (type: {section.type}).") # Changed log to logger
            return f"Error: Library '{library_name}' is not a music library."
        
        artist_count = section.totalSize
        album_count = section.totalViewSize(libtype='album')
        track_count = section.totalViewSize(libtype='track')
        
        result = {
            "library_name": library_name,
            "artist_count": artist_count,
            "album_count": album_count,
            "track_count": track_count,
            "total_duration_ms": section.totalDuration if hasattr(section, 'totalDuration') else None
        }
        logger.info(f"get_music_library_stats: Stats for '{library_name}': Artists={artist_count}, Albums={album_count}, Tracks={track_count}") # Changed log to logger
        return result
    except NotFound:
        logger.warn(f"get_music_library_stats: Library '{library_name}' not found.") # Changed log to logger
        return f"Error: Library '{library_name}' not found."
    except Exception as e:
        logger.error(f"Error retrieving music stats for library '{library_name}': {e}", exc_info=True)
        return f"Error: Failed to retrieve music library statistics. Details: {e}"

@mcp.tool()
def media_stats(ctx: Context) -> str:
    """Retrieves comprehensive statistics about all media libraries on the Plex server.
    
    Returns:
        A human-readable string containing detailed statistics for all media types including:
        - Movies: count, total duration, storage size
        - TV Shows: shows count, seasons count, episodes count, total duration
        - Music: artists count, albums count, tracks count, total duration
        - Other media types if present
        - Overall totals and storage information
    """
    logger.info(f"Executing tool: media_stats()") # Changed log to logger
    plex_server = getattr(ctx.fastmcp, 'plex_server', None)
    if plex_server is None:
        logger.error("media_stats: Plex server is not connected.") # Changed log to logger
        return "Error: Plex server is not connected. Check configuration and server startup logs."
    
    try:
        # Initialize counters for different media types
        totals = {
            "libraries": 0,
            "movies": 0,
            "shows": 0,
            "seasons": 0,
            "episodes": 0,
            "artists": 0,
            "albums": 0,
            "tracks": 0,
            "photos": 0,
            "other_items": 0,
            "total_duration_ms": 0,
            "total_storage_bytes": 0
        }
        
        # Get server info
        server_name = getattr(plex_server, 'friendlyName', 'N/A')
        server_version = getattr(plex_server, 'version', 'N/A')
        server_platform = getattr(plex_server, 'platform', 'N/A')
        
        # Get all library sections
        sections = plex_server.library.sections()
        totals["libraries"] = len(sections)
        
        # Lists to store library details by type
        movie_libraries = []
        tv_libraries = []
        music_libraries = []
        photo_libraries = []
        other_libraries = []
        
        # Process each library section
        for section in sections:
            # Add type-specific statistics
            if section.type == 'movie':
                movie_count = section.totalSize
                duration_ms = getattr(section, 'totalDuration', 0)
                storage_bytes = getattr(section, 'totalStorage', 0)
                
                # Format duration
                duration_days = duration_ms / 1000 / 86400
                
                # Format storage
                if storage_bytes >= 1024**4:
                    storage_str = f"{storage_bytes / (1024**4):.2f} TB"
                else:
                    storage_str = f"{storage_bytes / (1024**3):.2f} GB"
                
                movie_libraries.append({
                    "name": section.title,
                    "count": movie_count,
                    "duration_days": duration_days,
                    "storage": storage_str
                })
                
                totals["movies"] += movie_count
                totals["total_duration_ms"] += duration_ms
                totals["total_storage_bytes"] += storage_bytes
                
            elif section.type == 'show':
                shows_count = section.totalSize
                seasons_count = section.totalViewSize(libtype='season')
                episodes_count = section.totalViewSize(libtype='episode')
                duration_ms = getattr(section, 'totalDuration', 0)
                storage_bytes = getattr(section, 'totalStorage', 0)
                
                # Format duration
                duration_days = duration_ms / 1000 / 86400
                
                # Format storage
                if storage_bytes >= 1024**4:
                    storage_str = f"{storage_bytes / (1024**4):.2f} TB"
                else:
                    storage_str = f"{storage_bytes / (1024**3):.2f} GB"
                
                tv_libraries.append({
                    "name": section.title,
                    "shows": shows_count,
                    "seasons": seasons_count,
                    "episodes": episodes_count,
                    "duration_days": duration_days,
                    "storage": storage_str
                })
                
                totals["shows"] += shows_count
                totals["seasons"] += seasons_count
                totals["episodes"] += episodes_count
                totals["total_duration_ms"] += duration_ms
                totals["total_storage_bytes"] += storage_bytes
                
            elif section.type == 'artist':
                artists_count = section.totalSize
                albums_count = section.totalViewSize(libtype='album')
                tracks_count = section.totalViewSize(libtype='track')
                duration_ms = getattr(section, 'totalDuration', 0)
                storage_bytes = getattr(section, 'totalStorage', 0)
                
                # Format duration
                duration_days = duration_ms / 1000 / 86400
                
                # Format storage
                if storage_bytes >= 1024**4:
                    storage_str = f"{storage_bytes / (1024**4):.2f} TB"
                else:
                    storage_str = f"{storage_bytes / (1024**3):.2f} GB"
                
                music_libraries.append({
                    "name": section.title,
                    "artists": artists_count,
                    "albums": albums_count,
                    "tracks": tracks_count,
                    "duration_days": duration_days,
                    "storage": storage_str
                })
                
                totals["artists"] += artists_count
                totals["albums"] += albums_count
                totals["tracks"] += tracks_count
                totals["total_duration_ms"] += duration_ms
                totals["total_storage_bytes"] += storage_bytes
                
            elif section.type == 'photo':
                photos_count = section.totalSize
                storage_bytes = getattr(section, 'totalStorage', 0)
                
                # Format storage
                if storage_bytes >= 1024**4:
                    storage_str = f"{storage_bytes / (1024**4):.2f} TB"
                else:
                    storage_str = f"{storage_bytes / (1024**3):.2f} GB"
                
                photo_libraries.append({
                    "name": section.title,
                    "photos": photos_count,
                    "storage": storage_str
                })
                
                totals["photos"] += photos_count
                totals["total_storage_bytes"] += storage_bytes
                
            else:
                items_count = section.totalSize
                storage_bytes = getattr(section, 'totalStorage', 0)
                
                # Format storage
                if storage_bytes >= 1024**4:
                    storage_str = f"{storage_bytes / (1024**4):.2f} TB"
                else:
                    storage_str = f"{storage_bytes / (1024**3):.2f} GB"
                
                other_libraries.append({
                    "name": section.title,
                    "items": items_count,
                    "storage": storage_str
                })
                
                totals["other_items"] += items_count
                totals["total_storage_bytes"] += storage_bytes
        
        # Format total duration
        total_seconds = totals["total_duration_ms"] / 1000
        days = int(total_seconds // 86400)
        hours = int((total_seconds % 86400) // 3600)
        minutes = int((total_seconds % 3600) // 60)
        total_duration_human = f"{days:,} days, {hours} hours, {minutes} minutes"
        
        # Format total storage
        if totals["total_storage_bytes"] >= 1024**4:
            total_storage_human = f"{totals['total_storage_bytes'] / (1024**4):.2f} TB"
        else:
            total_storage_human = f"{totals['total_storage_bytes'] / (1024**3):.2f} GB"
        
        # Build the human-readable output
        output = []
        
        # Server information
        output.append(f"PLEX MEDIA SERVER STATISTICS")
        output.append(f"===========================")
        output.append(f"Server: {server_name}")
        output.append(f"Version: {server_version}")
        output.append(f"Platform: {server_platform}")
        output.append("")
        
        # Movies
        if movie_libraries:
            output.append(f"MOVIES")
            output.append(f"------")
            for lib in movie_libraries:
                output.append(f"Library: {lib['name']}")
                output.append(f"  Movies: {lib['count']:,}")
                output.append(f"  Duration: {lib['duration_days']:.1f} days")
                output.append(f"  Storage: {lib['storage']}")
                output.append("")
            
            output.append(f"Total Movies: {totals['movies']:,}")
            output.append("")
        
        # TV Shows
        if tv_libraries:
            output.append(f"TV SHOWS")
            output.append(f"--------")
            for lib in tv_libraries:
                output.append(f"Library: {lib['name']}")
                output.append(f"  Shows: {lib['shows']:,}")
                output.append(f"  Seasons: {lib['seasons']:,}")
                output.append(f"  Episodes: {lib['episodes']:,}")
                output.append(f"  Duration: {lib['duration_days']:.1f} days")
                output.append(f"  Storage: {lib['storage']}")
                output.append("")
            
            output.append(f"Total TV Shows: {totals['shows']:,}")
            output.append(f"Total Seasons: {totals['seasons']:,}")
            output.append(f"Total Episodes: {totals['episodes']:,}")
            output.append("")
        
        # Music
        if music_libraries:
            output.append(f"MUSIC")
            output.append(f"-----")
            for lib in music_libraries:
                output.append(f"Library: {lib['name']}")
                output.append(f"  Artists: {lib['artists']:,}")
                output.append(f"  Albums: {lib['albums']:,}")
                output.append(f"  Tracks: {lib['tracks']:,}")
                output.append(f"  Duration: {lib['duration_days']:.1f} days")
                output.append(f"  Storage: {lib['storage']}")
                output.append("")
            
            output.append(f"Total Artists: {totals['artists']:,}")
            output.append(f"Total Albums: {totals['albums']:,}")
            output.append(f"Total Tracks: {totals['tracks']:,}")
            output.append("")
        
        # Photos
        if photo_libraries:
            output.append(f"PHOTOS")
            output.append(f"------")
            for lib in photo_libraries:
                output.append(f"Library: {lib['name']}")
                output.append(f"  Photos: {lib['photos']:,}")
                output.append(f"  Storage: {lib['storage']}")
                output.append("")
            
            output.append(f"Total Photos: {totals['photos']:,}")
            output.append("")
        
        # Other
        if other_libraries:
            output.append(f"OTHER LIBRARIES")
            output.append(f"--------------")
            for lib in other_libraries:
                output.append(f"Library: {lib['name']}")
                output.append(f"  Items: {lib['items']:,}")
                output.append(f"  Storage: {lib['storage']}")
                output.append("")
            
            output.append(f"Total Other Items: {totals['other_items']:,}")
            output.append("")
        
        # Overall totals
        output.append(f"OVERALL TOTALS")
        output.append(f"--------------")
        output.append(f"Total Libraries: {totals['libraries']}")
        if totals['movies'] > 0:
            output.append(f"Total Movies: {totals['movies']:,}")
        if totals['shows'] > 0:
            output.append(f"Total TV Shows: {totals['shows']:,}")
            output.append(f"Total Seasons: {totals['seasons']:,}")
            output.append(f"Total Episodes: {totals['episodes']:,}")
        if totals['artists'] > 0:
            output.append(f"Total Artists: {totals['artists']:,}")
            output.append(f"Total Albums: {totals['albums']:,}")
            output.append(f"Total Tracks: {totals['tracks']:,}")
        if totals['photos'] > 0:
            output.append(f"Total Photos: {totals['photos']:,}")
        if totals['other_items'] > 0:
            output.append(f"Total Other Items: {totals['other_items']:,}")
        
        output.append(f"Total Duration: {total_duration_human}")
        output.append(f"Total Storage: {total_storage_human}")
        
        final_output_string = "\n".join(output) # Corrected from \n to \n for multiline string representation in edit
        logger.info(f"media_stats: Successfully generated statistics string (length: {len(final_output_string)}).")
        return final_output_string
        
    except Exception as e:
        logger.error(f"Error retrieving media stats: {e}", exc_info=True)
        return f"Error: Failed to retrieve media statistics. Details: {e}"

# --- New Health Endpoint for Dashboard ---
@mcp.get("/health", tags=["mcp_server_health"])
async def mcp_server_health_check(ctx: Context) -> Dict[str, Any]:
    """
    Provides a health check for the MCP server itself and its ability to connect to Plex.
    This is intended for use by monitoring dashboards.
    """
    logger.info("MCP server health check requested for Plex.")
    service_name = "plex" # Define service name
    plex_server: Optional[PlexServer] = getattr(ctx.fastmcp, 'plex_server', None)

    mcp_configured = all([PLEX_URL, PLEX_TOKEN])
    if not mcp_configured:
        logger.error("Plex URL or Token not configured for the MCP server.")
        return {"status": "error", "service_name": service_name, "service_accessible": False, "mcp_server_configured": False, "reason": "Plex URL or Token not configured for MCP server."}
    
    if not plex_server:
        logger.warning("Plex server (plex_server) not found on app context. Likely failed to connect during startup.")
        return {"status": "error", "service_name": service_name, "service_accessible": False, "mcp_server_configured": True, "reason": "Plex server client not initialized on MCP server. Check startup logs for connection errors to Plex."}
    
    try:
        # Accessing a simple attribute like friendlyName or version should confirm the connection is alive.
        # The PlexServer object is instantiated at startup. If it exists, connection was successful then.
        # Re-check by fetching a lightweight property to confirm current state.
        logger.debug("Attempting to fetch Plex server friendlyName and version for health check...")
        server_name = plex_server.friendlyName
        server_version = plex_server.version
        logger.info(f"Plex instance accessible: {server_name} (Version: {server_version})")
        return {
            "status": "ok", 
            "service_name": service_name,
            "service_accessible": True, 
            "mcp_server_configured": True,
            "details": {
                "server_name": server_name,
                "server_version": server_version,
                "message": "Plex instance is responsive."
            }
        }
    except (Unauthorized, NotFound, BadRequest) as e: # Specific plexapi exceptions
        logger.error(f"Plex API error during health check: {type(e).__name__} - {e}", exc_info=True)
        return {"status": "error", "service_name": service_name, "service_accessible": False, "mcp_server_configured": True, "reason": f"Plex API error: {type(e).__name__} - {e}"}
    except Exception as e: # Catch any other exceptions, like network issues if the connection dropped
        logger.error(f"Unexpected error during Plex health check: {e}", exc_info=True)
        return {"status": "error", "service_name": service_name, "service_accessible": False, "mcp_server_configured": True, "reason": f"Unexpected error during health check: {str(e)}"}


# --- Main Execution ---
def main():
    """Main entry point for running the MCP server."""
    logger.info(f"Starting FastMCP server (Plex MCP Server) with transport: {PLEX_MCP_TRANSPORT}")
    if PLEX_MCP_TRANSPORT == "stdio":
        mcp.run() 
    elif PLEX_MCP_TRANSPORT == "sse":
        mcp.run(
            transport="sse",
            host=PLEX_MCP_HOST,
            port=PLEX_MCP_PORT,
            path="/mcp" # As per create-mcp-server_v2.md recommendation
        )
    else:
        logger.error(f"Invalid PLEX_MCP_TRANSPORT: '{PLEX_MCP_TRANSPORT}'. Must be 'stdio' or 'sse'. Defaulting to stdio.")
        mcp.run()


if __name__ == "__main__":
    main()
