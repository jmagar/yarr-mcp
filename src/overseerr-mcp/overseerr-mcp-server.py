import asyncio
import os
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from contextlib import asynccontextmanager
from fastmcp import FastMCP, Context
from dotenv import load_dotenv
import logging
from logging.handlers import RotatingFileHandler
from typing import Optional, List, Dict, Union, Any
from pathlib import Path # For .env loading
from fastapi.middleware.cors import CORSMiddleware

from client import OverseerrApiClient # Direct import from SCRIPT_DIR

from starlette.requests import Request # Added
from starlette.responses import JSONResponse # Added

# --- Environment Loading & Configuration ---
project_root = Path(__file__).resolve().parent.parent.parent
env_path = project_root / '.env'

# Initial minimal logging for dotenv loading itself
print(f"OverseerrMCP: Looking for .env file at: {env_path}")
found_dotenv = load_dotenv(dotenv_path=env_path, override=True)
print(f"OverseerrMCP: load_dotenv found file: {found_dotenv}")

OVERSEERR_URL = os.getenv("OVERSEERR_URL")
OVERSEERR_API_KEY = os.getenv("OVERSEERR_API_KEY")

# Transport and Port Configuration
OVERSEERR_MCP_TRANSPORT = os.getenv("OVERSEERR_MCP_TRANSPORT", "sse").lower()
OVERSEERR_MCP_HOST = os.getenv("OVERSEERR_MCP_HOST", "0.0.0.0")
OVERSEERR_MCP_PORT = int(os.getenv("OVERSEERR_MCP_PORT", "8001")) # Defaulting to a different port for Overseerr

# Logging Configuration
OVERSEERR_LOG_LEVEL_STR = os.getenv('OVERSEERR_LOG_LEVEL', os.getenv('LOG_LEVEL', 'INFO')).upper()
NUMERIC_LOG_LEVEL = getattr(logging, OVERSEERR_LOG_LEVEL_STR, logging.INFO)

logger = logging.getLogger("OverseerrMCPServer")
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
log_file_name = f"{os.getenv('OVERSEERR_NAME', 'overseerr').lower()}_mcp.log"
log_file_path = SCRIPT_DIR / log_file_name
file_handler = RotatingFileHandler(log_file_path, maxBytes=5*1024*1024, backupCount=3, encoding='utf-8')
file_handler.setLevel(NUMERIC_LOG_LEVEL)
file_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(module)s - %(funcName)s - %(lineno)d - %(message)s')
file_handler.setFormatter(file_formatter)
if not any(isinstance(h, RotatingFileHandler) for h in logger.handlers):
    logger.addHandler(file_handler)

logger.info(f"Logging initialized (console and file: {log_file_path}). Effective log level: {OVERSEERR_LOG_LEVEL_STR}")

# Log loaded essential configurations
logger.info(f"OVERSEERR_URL loaded: {'Yes' if OVERSEERR_URL else 'No'}")
logger.info(f"OVERSEERR_API_KEY loaded: {'Yes' if OVERSEERR_API_KEY and len(OVERSEERR_API_KEY) > 5 else 'No'}") # Basic check for presence
logger.info(f"OVERSEERR_MCP_TRANSPORT set to: {OVERSEERR_MCP_TRANSPORT}")
logger.info(f"OVERSEERR_MCP_HOST set to: {OVERSEERR_MCP_HOST}")
logger.info(f"OVERSEERR_MCP_PORT set to: {OVERSEERR_MCP_PORT}")
logger.info(f"OVERSEERR_LOG_LEVEL (effective): {OVERSEERR_LOG_LEVEL_STR}")

# --- Critical Check for Essential API Credentials/URL ---
if not OVERSEERR_URL or not OVERSEERR_API_KEY:
    logger.error("CRITICAL: OVERSEERR_URL and OVERSEERR_API_KEY must be set in environment variables. Server cannot start.")
    sys.exit(1)

@asynccontextmanager
async def overseerr_lifespan(app: FastMCP):
    logger.info("Overseerr Lifespan: Startup sequence initiated.")
    if not OVERSEERR_URL or not OVERSEERR_API_KEY:
        logger.error("Overseerr Lifespan Error: OVERSEERR_URL and OVERSEERR_API_KEY must be set. API client will not be initialized.")
        app.overseerr_client = None
    else:
        try:
            logger.info(f"Overseerr Lifespan: Initializing OverseerrApiClient for {OVERSEERR_URL}")
            app.overseerr_client = OverseerrApiClient(base_url=OVERSEERR_URL, api_key=OVERSEERR_API_KEY)
            # Optionally, make a test call to ensure client is working, e.g., get settings
            # test_settings = await app.overseerr_client.get("/settings/main") 
            # if isinstance(test_settings, str): # Error string
            #     log.error(f"Overseerr Lifespan Error: Test call to Overseerr failed: {test_settings}")
            #     await app.overseerr_client.close()
            #     app.overseerr_client = None
            # else:
            #     log.info("Overseerr Lifespan: API client initialized and test call successful.")
            logger.info("Overseerr Lifespan: API client initialized.") # Simplified for now
        except Exception as e:
            logger.error(f"Overseerr Lifespan Error: Failed to initialize OverseerrApiClient: {e}", exc_info=True)
            app.overseerr_client = None
    
    logger.info("Overseerr Lifespan: Startup complete.")
    yield
    # Cleanup on shutdown
    logger.info("Overseerr Lifespan: Shutdown sequence initiated.")
    if hasattr(app, 'overseerr_client') and app.overseerr_client:
        await app.overseerr_client.close()
        logger.info("Overseerr Lifespan: Overseerr API client closed.")
    logger.info("Overseerr Lifespan: Shutdown complete.")

mcp = FastMCP(
    name="OverseerrMCP",
    instructions="Interact with an Overseerr instance for media requests and discovery.",
    lifespan=overseerr_lifespan
)

# --- Tools will be added here ---
@mcp.tool()
async def search_media(ctx: Context, query: str, media_type: Optional[str] = None) -> Union[List[Dict], str]:
    """Searches Overseerr for movies or TV shows.

    Args:
        query: The search term.
        media_type: Optional. 'movie' or 'tv' to limit search. Searches both if None.

    Returns:
        A list of search results or an error string.
    """
    logger.info(f"Executing tool: search_media(query='{query}', media_type='{media_type}')")
    client = getattr(ctx.fastmcp, 'overseerr_client', None)
    if not client:
        return "Error: Overseerr API client is not available. Check server startup logs."

    params = {"query": query}
    # Overseerr's /search endpoint doesn't directly filter by media_type in query params.
    # It returns mixed results. We will filter client-side if media_type is specified.
    # Alternatively, one could make separate calls to /movie/discover or /tv/discover with query, 
    # but /search is simpler for a general query.

    api_response = await client.get("/search", params=params)

    if isinstance(api_response, str): # Error string from client
        return api_response 
    
    results = []
    if isinstance(api_response, dict) and 'results' in api_response:
        processed_items = 0
        for item in api_response['results']:
            item_media_type = item.get('mediaType') # 'movie' or 'tv'
            
            # Client-side filtering if media_type is specified in args
            if media_type and item_media_type != media_type:
                continue

            details = {
                "tmdbId": item.get('id'),
                "mediaType": item_media_type,
                "title": item.get('title') or item.get('name') or item.get('originalName'),
                "year": item.get('releaseDate', '').split('-')[0] if item.get('releaseDate') else (item.get('firstAirDate', '').split('-')[0] if item.get('firstAirDate') else None),
                "overview": item.get('overview'),
                "posterPath": item.get('posterPath'),
                "backdropPath": item.get('backdropPath')
            }
            results.append(details)
            processed_items += 1
        
        if not results and processed_items == 0 and media_type:
             return f"No '{media_type}' results found for query '{query}'."
        elif not results and processed_items == 0:
            return f"No results found for query '{query}'."
            
        return results
    else:
        logger.warning(f"Unexpected API response structure from /search: {api_response}")
        return "Error: Received unexpected data structure from Overseerr search."

@mcp.tool()
async def get_movie_details(ctx: Context, tmdb_id: int) -> Union[Dict, str]:
    """Retrieves detailed information for a specific movie from Overseerr using its TMDB ID.

    Args:
        tmdb_id: The TheMovieDB ID for the movie.

    Returns:
        A dictionary containing movie details or an error string.
    """
    logger.info(f"Executing tool: get_movie_details(tmdb_id={tmdb_id})")
    client = getattr(ctx.fastmcp, 'overseerr_client', None)
    if not client:
        return "Error: Overseerr API client is not available. Check server startup logs."

    api_response = await client.get(f"/movie/{tmdb_id}")

    if isinstance(api_response, str): # Error string from client
        return api_response
    
    # Assuming api_response is a dict with movie details
    # We can select/rename fields if needed, or return as-is if structure is good.
    # For now, let's return it directly, assuming it's JSON-friendly.
    if isinstance(api_response, dict):
        # Basic check for some expected movie fields
        if 'id' in api_response and 'title' in api_response:
            logger.info(f"Successfully retrieved details for movie TMDB ID: {tmdb_id}")
            return api_response 
        else:
            logger.warning(f"Overseerr movie details for {tmdb_id} missing expected fields: {api_response}")
            return f"Error: Received incomplete movie data structure from Overseerr for TMDB ID {tmdb_id}."
    else:
        logger.warning(f"Unexpected API response structure from /movie/{tmdb_id}: {api_response}")
        return "Error: Received unexpected data structure from Overseerr for movie details."

@mcp.tool()
async def get_tv_show_details(ctx: Context, tmdb_id: int) -> Union[Dict, str]:
    """Retrieves detailed information for a specific TV show from Overseerr using its TMDB ID.

    Args:
        tmdb_id: The TheMovieDB ID for the TV show.

    Returns:
        A dictionary containing TV show details (including seasons) or an error string.
    """
    logger.info(f"Executing tool: get_tv_show_details(tmdb_id={tmdb_id})")
    client = getattr(ctx.fastmcp, 'overseerr_client', None)
    if not client:
        return "Error: Overseerr API client is not available. Check server startup logs."

    # The /tv/{id} endpoint in Overseerr API returns details along with season information.
    api_response = await client.get(f"/tv/{tmdb_id}")

    if isinstance(api_response, str): # Error string from client
        return api_response
    
    if isinstance(api_response, dict):
        # Basic check for some expected TV show fields
        if 'id' in api_response and ('name' in api_response or 'originalName' in api_response):
            logger.info(f"Successfully retrieved details for TV show TMDB ID: {tmdb_id}")
            # The response often includes a 'seasons' list directly.
            return api_response 
        else:
            logger.warning(f"Overseerr TV show details for {tmdb_id} missing expected fields: {api_response}")
            return f"Error: Received incomplete TV show data structure from Overseerr for TMDB ID {tmdb_id}."
    else:
        logger.warning(f"Unexpected API response structure from /tv/{tmdb_id}: {api_response}")
        return "Error: Received unexpected data structure from Overseerr for TV show details."

@mcp.tool()
async def request_movie(ctx: Context, tmdb_id: int) -> Union[Dict, str]:
    """Requests a movie on Overseerr using its TMDB ID.

    Args:
        tmdb_id: The TheMovieDB ID for the movie to request.

    Returns:
        A dictionary containing the request details upon success, or an error string.
    """
    logger.info(f"Executing tool: request_movie(tmdb_id={tmdb_id})")
    client = getattr(ctx.fastmcp, 'overseerr_client', None)
    if not client:
        return "Error: Overseerr API client is not available. Check server startup logs."

    payload = {
        "mediaType": "movie",
        "mediaId": tmdb_id
    }

    api_response = await client.post("/request", json_data=payload)

    if isinstance(api_response, str): # Error string from client
        # More specific error check for existing requests might be needed based on API behavior
        if "already been requested" in api_response.lower() or "already exists" in api_response.lower():
            logger.warning(f"Movie with TMDB ID {tmdb_id} might already be requested or available: {api_response}")
            # Return the original error message as it might contain useful info
        return api_response 
    
    if isinstance(api_response, dict):
        # A successful request usually returns the created/updated request object
        # Check for core request fields and matching tmdbId in the nested media object
        if ('id' in api_response and 
            'status' in api_response and 
            isinstance(api_response.get('media'), dict) and
            api_response['media'].get('tmdbId') == tmdb_id):
            logger.info(f"Successfully requested movie with TMDB ID: {tmdb_id}. Request ID: {api_response['id']}")
            return api_response
        else:
            logger.warning(f"Overseerr movie request for TMDB ID {tmdb_id} returned unexpected data structure: {api_response}")
            return f"Error: Movie request for TMDB ID {tmdb_id} completed, but response data was unexpected. Response: {str(api_response)[:200]}..."
    else:
        logger.warning(f"Unexpected API response type from /request for movie TMDB ID {tmdb_id}: {type(api_response)}. Response: {str(api_response)[:200]}...")
        return "Error: Received unexpected data type from Overseerr after movie request."

@mcp.tool()
async def request_tv_show(ctx: Context, tmdb_id: int, seasons: Optional[List[int]] = None) -> Union[Dict, str]:
    """Requests a TV show, or specific seasons of a TV show, on Overseerr using its TMDB ID.

    Args:
        tmdb_id: The TheMovieDB ID for the TV show.
        seasons: Optional list of season numbers to request. If None or empty, requests the entire show (typically all unrequested seasons).

    Returns:
        A dictionary containing the request details upon success, or an error string.
    """
    logger.info(f"Executing tool: request_tv_show(tmdb_id={tmdb_id}, seasons={seasons})")
    client = getattr(ctx.fastmcp, 'overseerr_client', None)
    if not client:
        return "Error: Overseerr API client is not available. Check server startup logs."

    payload = {
        "mediaType": "tv",
        "mediaId": tmdb_id
    }
    if seasons: # If seasons list is provided and not empty
        payload["seasons"] = seasons
    else: # Explicitly ask for all seasons if none are specified, as per API spec allowing "all"
        payload["seasons"] = "all"

    api_response = await client.post("/request", json_data=payload)

    if isinstance(api_response, str): # Error string from client
        if "already been requested" in api_response.lower() or "already exists" in api_response.lower():
            logger.warning(f"TV show TMDB ID {tmdb_id} (seasons: {seasons if seasons else 'all'}) might already be requested/available: {api_response}")
        return api_response
    
    if isinstance(api_response, dict):
        # Check for core request fields and matching tmdbId in the nested media object
        if ('id' in api_response and 
            'status' in api_response and
            isinstance(api_response.get('media'), dict) and
            api_response['media'].get('tmdbId') == tmdb_id):
            logger.info(f"Successfully requested TV show TMDB ID: {tmdb_id} (Seasons: {payload['seasons']}). Request ID: {api_response['id']}")
            return api_response
        else:
            logger.warning(f"Overseerr TV show request for TMDB ID {tmdb_id} returned unexpected data structure: {api_response}")
            return f"Error: TV show request for TMDB ID {tmdb_id} completed, but response data was unexpected. Response: {str(api_response)[:200]}..."
    else:
        logger.warning(f"Unexpected API response type from /request for TV show TMDB ID {tmdb_id}: {type(api_response)}. Response: {str(api_response)[:200]}...")
        return "Error: Received unexpected data type from Overseerr after TV show request."

@mcp.tool()
async def list_failed_requests(ctx: Context, count: int = 10, skip: int = 0) -> Union[List[Dict], str]:
    """Lists failed media requests from Overseerr.

    Args:
        count: Number of requests to retrieve (default 10).
        skip: Number of requests to skip (for pagination, default 0).

    Returns:
        A list of failed requests or an error string.
    """
    logger.info(f"Executing tool: list_failed_requests(count={count}, skip={skip})")
    client = getattr(ctx.fastmcp, 'overseerr_client', None)
    if not client:
        return "Error: Overseerr API client is not available. Check server startup logs."

    params = {
        "take": count,
        "skip": skip,
        "sort": "modified", # Sort by when they were last modified, might be more relevant for failures
        "filter": "failed"   # Filter for failed requests
    }

    api_response = await client.get("/request", params=params)

    if isinstance(api_response, str): # Error string from client
        return api_response
    
    results = []
    if isinstance(api_response, dict) and 'results' in api_response:
        for req in api_response['results']:
            media_info = req.get('media', {})
            details = {
                "requestId": req.get('id'),
                "status": req.get('status'), 
                "type": media_info.get('mediaType'), 
                "tmdbId": media_info.get('tmdbId'),
                "title": media_info.get('title') or media_info.get('name'),
                "requested_by": req.get('requestedBy', {}).get('displayName'),
                "requested_at": req.get('createdAt'),
                "modified_at": req.get('modifiedAt')
            }
            results.append(details)
        
        if not results:
            return "No failed requests found."
        return results
    else:
        logger.warning(f"Unexpected API response structure from /request for failed list: {api_response}")
        return "Error: Received unexpected data structure from Overseerr for failed requests."

# --- New Health Endpoint for Dashboard ---
@mcp.custom_route("/health", methods=["GET"])
async def mcp_server_health_check(request: Request) -> JSONResponse:
    logger.info("MCP server health check requested for Overseerr (custom_route).")
    service_name = "overseerr"
    # Access client via request.app, which should be the 'mcp' instance
    client: Optional[OverseerrApiClient] = getattr(request.app, 'overseerr_client', None)

    mcp_configured = all([OVERSEERR_URL, OVERSEERR_API_KEY])
    if not mcp_configured:
        logger.error("Overseerr URL or API Key not configured for the MCP server.")
        return JSONResponse({"status": "error", "service_name": service_name, "service_accessible": False, "mcp_server_configured": False, "reason": "Overseerr URL or API Key not configured for MCP server."}, status_code=500)
    
    if not client:
        logger.warning("Overseerr client not initialized on MCP server. Check startup logs.")
        return JSONResponse({"status": "error", "service_name": service_name, "service_accessible": False, "mcp_server_configured": True, "reason": "Overseerr client not initialized on MCP server. Check startup logs."}, status_code=503)
        
    try:
        logger.debug("Attempting to call Overseerr /settings/main endpoint for health check...")
        response = await client.get("/settings/main")
        
        if isinstance(response, dict) and response.get("appVersion"):
            app_version = response.get("appVersion", "N/A")
            logger.info(f"Overseerr instance accessible. App Version: {app_version}.")
            return JSONResponse({"status": "ok", "service_name": service_name, "service_accessible": True, "mcp_server_configured": True, "details": {"app_version": app_version, "message": "Overseerr instance is responsive."}})
        elif isinstance(response, dict):
            logger.warning(f"Overseerr accessible but health check returned unexpected data: {response}")
            return JSONResponse({"status": "ok", "service_name": service_name, "service_accessible": True, "mcp_server_configured": True, "details": {"message": "Overseerr responsive but health data partial.", "raw_response": response}})
        else:
            error_detail = response if isinstance(response, str) else "Unknown error structure from API"
            logger.warning(f"Overseerr health check failed. Client returned: {error_detail}")
            return JSONResponse({"status": "error", "service_name": service_name, "service_accessible": False, "mcp_server_configured": True, "reason": f"Failed to connect/API error: {error_detail}"}, status_code=503)
            
    except Exception as e:
        logger.exception("Unexpected exception during Overseerr health check")
        return JSONResponse({"status": "error", "service_name": service_name, "service_accessible": False, "mcp_server_configured": True, "reason": f"Unexpected health check exception: {str(e)}"}, status_code=500)

def main():
    logger.info(f"Starting Overseerr MCP Server with transport: {OVERSEERR_MCP_TRANSPORT}")
    # The critical check for OVERSEERR_URL and OVERSEERR_API_KEY is now at the top of the script.
    # The lifespan manager will also log if the client fails to initialize.

    if OVERSEERR_MCP_TRANSPORT == "stdio":
        mcp.run()
    elif OVERSEERR_MCP_TRANSPORT == "sse":
        mcp.run(
            transport="sse",
            host=OVERSEERR_MCP_HOST,
            port=OVERSEERR_MCP_PORT,
            path="/mcp"  # Standardized path
        )
    else:
        logger.error(f"Invalid OVERSEERR_MCP_TRANSPORT: '{OVERSEERR_MCP_TRANSPORT}'. Must be 'stdio' or 'sse'. Defaulting to stdio.")
        mcp.run() # Fallback to stdio

if __name__ == "__main__":
    main() 