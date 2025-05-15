"""
MCP Server for Prowlarr
Implements the approved tool set for Prowlarr using an SSE transport.
Built with FastMCP following best practices from gofastmcp.com
Based on Prowlarr API v1 (as per prowlarr-api.json)
"""

import os
import sys
import httpx # For making API calls
import logging
import math # Moved math import to top level
from logging.handlers import RotatingFileHandler
from pathlib import Path
from typing import Optional, List, Dict, Any, Union

from dotenv import load_dotenv # Added import
from fastmcp import FastMCP, Context
from fastapi.middleware.cors import CORSMiddleware

# --- Constants ---
API_VERSION = "v1"

# --- Logging Setup ---
# MCP Server specific configurations for logging and transport
PROWLARR_MCP_LOG_LEVEL = os.getenv('PROWLARR_MCP_LOG_LEVEL', os.getenv('LOG_LEVEL', 'INFO')).upper() # Prioritize specific, fallback to general
PROWLARR_MCP_LOG_FILE = os.getenv('PROWLARR_MCP_LOG_FILE', "prowlarr_mcp.log")
PROWLARR_MCP_TRANSPORT = os.getenv('PROWLARR_MCP_TRANSPORT', 'sse').lower()
PROWLARR_MCP_HOST = os.getenv('PROWLARR_MCP_HOST', '0.0.0.0')
PROWLARR_MCP_PORT = int(os.getenv('PROWLARR_MCP_PORT', '6973'))

NUMERIC_LOG_LEVEL = getattr(logging, PROWLARR_MCP_LOG_LEVEL, logging.INFO)
SCRIPT_DIR = Path(__file__).resolve().parent

# --- Environment Variable & API Client Setup ---
# Determine project root and .env path
project_root = SCRIPT_DIR.parent.parent # Assuming src/prowlarr-mcp, so up two levels to src, then one more to project root
env_path = project_root / '.env'

# Initial minimal logging for dotenv loading itself
print(f"ProwlarrMCP: Looking for .env file at: {env_path}")
found_dotenv = load_dotenv(dotenv_path=env_path, override=True)
print(f"ProwlarrMCP: load_dotenv found file: {found_dotenv}")

PROWLARR_URL = os.getenv('PROWLARR_URL')
PROWLARR_API_KEY = os.getenv('PROWLARR_API_KEY')

logger = logging.getLogger("ProwlarrMCPServer")
logger.setLevel(NUMERIC_LOG_LEVEL)
logger.propagate = False

# Console Handler
console_handler = logging.StreamHandler(sys.stdout)
console_handler.setLevel(NUMERIC_LOG_LEVEL)
console_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
console_handler.setFormatter(console_formatter)
logger.addHandler(console_handler)

# File Handler
log_file_path = SCRIPT_DIR / PROWLARR_MCP_LOG_FILE # Use the variable
file_handler = RotatingFileHandler(log_file_path, maxBytes=5*1024*1024, backupCount=3, encoding='utf-8')
file_handler.setLevel(NUMERIC_LOG_LEVEL)
file_formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(module)s - %(funcName)s - %(lineno)d - %(message)s')
file_handler.setFormatter(file_formatter)
logger.addHandler(file_handler)

logger.info(f"Logging initialized (console and file: {log_file_path}). Log level: {PROWLARR_MCP_LOG_LEVEL}")
logger.info(f"Prowlarr MCP Transport: {PROWLARR_MCP_TRANSPORT}, Host: {PROWLARR_MCP_HOST}, Port: {PROWLARR_MCP_PORT}")

if not PROWLARR_URL:
    logger.error("PROWLARR_URL environment variable must be set (or found in .env).") # Updated log message
    sys.exit(1)
if not PROWLARR_API_KEY:
    logger.error("PROWLARR_API_KEY environment variable must be set (or found in .env).") # Updated log message
    sys.exit(1)

# Ensure API URL doesn't end with a slash, as endpoints start with one
if PROWLARR_URL.endswith('/'):
    PROWLARR_URL = PROWLARR_URL[:-1]

logger.info(f"Prowlarr API URL: {PROWLARR_URL}")
logger.info(f"Prowlarr API Key: {'*' * (len(PROWLARR_API_KEY) - 4) + PROWLARR_API_KEY[-4:] if PROWLARR_API_KEY else 'Not Set'}")

# --- FastMCP Server Initialization ---
mcp = FastMCP(
    name="Prowlarr MCP Server",
    instructions="""Provides tools to interact with a Prowlarr instance.
Manages indexers, applications, performs searches, and checks system status.
Requires PROWLARR_URL and PROWLARR_API_KEY environment variables.
API interactions are based on Prowlarr API v1."""
)

# --- Helper Functions ---
async def _prowlarr_api_request(
    method: str,
    endpoint: str,
    params: Optional[Dict[str, Any]] = None,
    json_body: Optional[Dict[str, Any]] = None
) -> Union[Dict[str, Any], List[Any], None]:
    """Helper function to make requests to the Prowlarr API."""
    headers = {'X-Api-Key': PROWLARR_API_KEY}
    url = f"{PROWLARR_URL}{endpoint}"
    try:
        async with httpx.AsyncClient(timeout=30.0) as client:
            logger.debug(f"Request: {method} {url} - Params: {params} - Body: {json_body}")
            response = await client.request(method, url, params=params, json=json_body, headers=headers)
            response.raise_for_status() # Raises HTTPStatusError for 4xx/5xx responses
            if response.status_code == 204: # No content
                return None
            return response.json()
    except httpx.HTTPStatusError as e:
        logger.error(f"HTTP error calling Prowlarr API: {e.response.status_code} - {e.response.text}")
        error_content = {"error": f"Prowlarr API Error: {e.response.status_code}", "details": e.response.text}
        try: # Try to parse if Prowlarr gives a JSON error
            error_content["details"] = e.response.json()
        except Exception:
            pass
        return error_content
    except httpx.RequestError as e:
        logger.error(f"Request error calling Prowlarr API: {e}")
        return {"error": f"Request to Prowlarr API failed: {e}"}
    except Exception as e:
        logger.error(f"Unexpected error in Prowlarr API request: {e}", exc_info=True)
        return {"error": f"An unexpected error occurred: {str(e)}"}

def _human_readable_size(size_bytes: int) -> str:
    if size_bytes == 0:
        return "0B"
    size_name = ("B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB")
    i = int(math.floor(math.log(size_bytes, 1024)))
    p = math.pow(1024, i)
    s = round(size_bytes / p, 2)
    return f"{s} {size_name[i]}"

# --- Core Tools ---

@mcp.tool()
async def list_indexers() -> Dict[str, Any]:
    """
    Retrieves a list of all configured indexers in Prowlarr.
    Returns a summary and the full list of IndexerResource objects.
    """
    logger.info("Listing all indexers...")
    response = await _prowlarr_api_request("GET", f"/api/{API_VERSION}/indexer")

    if isinstance(response, dict) and "error" in response:
        return response # Return error as is

    if isinstance(response, list):
        summary = f"Found {len(response)} indexers. "
        if response:
            summary += "First few: " + ", ".join([f"{idx.get('name', 'N/A')} (ID: {idx.get('id', 'N/A')}, Enabled: {idx.get('enable', False)})" for idx in response[:3]])
        else:
            summary += "No indexers configured."
        
        return {"summary": summary, "indexers": response}
    
    logger.error(f"Unexpected response type from Prowlarr API for list_indexers: {type(response)}")
    return {"error": "Failed to list indexers due to unexpected API response."}

@mcp.tool()
async def get_indexer_details(id: int) -> Dict[str, Any]:
    """
    Retrieves detailed information about a specific indexer by its ID.
    Returns the full IndexerResource object.
    """
    logger.info(f"Getting details for indexer ID: {id}")
    response = await _prowlarr_api_request("GET", f"/api/{API_VERSION}/indexer/{id}")
    
    if isinstance(response, dict) and "error" in response:
        return response

    if isinstance(response, dict): # Expecting a single IndexerResource
        return {"summary": f"Details for Indexer '{response.get('name', 'N/A')}' (ID: {id}) retrieved.", "details": response}

    logger.error(f"Unexpected response type from Prowlarr API for get_indexer_details: {type(response)}")
    return {"error": f"Failed to get indexer details for ID {id} due to unexpected API response."}

@mcp.tool()
async def search_releases(
    query: str,
    indexerIds: Optional[List[int]] = None, 
    categories: Optional[List[int]] = None, 
    type: Optional[str] = None, # e.g., "tvsearch", "moviesearch" - exact values need confirmation
    limit: Optional[int] = 50, 
    offset: Optional[int] = 0
) -> Dict[str, Any]:
    """
    Performs a search query across specified or all enabled indexers for releases.
    Type parameter (e.g., "tvsearch", "moviesearch") exact values need Prowlarr documentation confirmation.
    Returns a summary of results and the full list of ReleaseResource objects.
    """
    logger.info(f"Searching releases for query: '{query}', Type: {type}, Categories: {categories}, IndexerIDs: {indexerIds}")
    params = {"query": query, "limit": limit, "offset": offset}
    if indexerIds:
        params["indexerIds"] = ",".join(map(str, indexerIds)) # API expects comma-separated string for array query params
    if categories:
        params["categories"] = ",".join(map(str, categories))
    if type:
        params["type"] = type
        
    response = await _prowlarr_api_request("GET", f"/api/{API_VERSION}/search", params=params)

    if isinstance(response, dict) and "error" in response:
        return response

    if isinstance(response, list):
        summary = f"Found {len(response)} releases for query '{query}'."
        if response:
            summary += " First few results:"
            for rel in response[:3]:
                title = rel.get('title', 'N/A')
                indexer_name = rel.get('indexer', 'N/A')
                size_bytes = rel.get('size', 0)
                protocol = rel.get('protocol', 'N/A').capitalize()
                seeders = rel.get('seeders', 'N/A')
                leechers = rel.get('leechers', 'N/A')
                
                details = f"  - '{title}' from {indexer_name} ({_human_readable_size(size_bytes)})"
                if protocol == 'Torrent':
                    details += f" (S: {seeders}, L: {leechers})"
                summary += f"\n{details}"
        
        return {"summary": summary, "releases": response}

    logger.error(f"Unexpected response type from Prowlarr API for search_releases: {type(response)}")
    return {"error": "Failed to search releases due to unexpected API response."}

@mcp.tool()
async def test_indexer(id: int) -> Dict[str, Any]:
    """
    Tests the connectivity and search capability of a specific indexer by its ID.
    The tool first fetches the indexer's configuration to use in the test request.
    Returns a success or failure message.
    """
    logger.info(f"Testing indexer ID: {id}")
    
    # Step 1: Get the IndexerResource for the given ID
    indexer_resource_response = await _prowlarr_api_request("GET", f"/api/{API_VERSION}/indexer/{id}")

    if not isinstance(indexer_resource_response, dict) or "id" not in indexer_resource_response:
        logger.error(f"Failed to fetch IndexerResource for ID {id} to perform test.")
        return {"error": f"Could not retrieve indexer details for ID {id} before testing.", "details": indexer_resource_response}

    if "error" in indexer_resource_response: # Error from fetching
        return indexer_resource_response

    # Step 2: Perform the test using the fetched resource
    # The API spec for POST /api/v1/indexer/test expects an IndexerResource in the body.
    # It seems it validates the provided resource.
    # The response for a successful test is typically 200 OK, possibly with validation results.
    # The prowlarr-api.json indicates 200 OK but doesn't specify response body for POST /api/v1/indexer/test
    
    test_response = await _prowlarr_api_request("POST", f"/api/{API_VERSION}/indexer/test", json_body=indexer_resource_response)

    if test_response is None: # Successful 204 No Content or 200 OK with empty body might mean success
         return {"summary": f"Test initiated for indexer '{indexer_resource_response.get('name', id)}'. Prowlarr API returned success (e.g. 200 OK or 204). Check Prowlarr UI or logs for detailed test results if not provided here.", "status": "success"}

    if isinstance(test_response, dict) and "error" in test_response:
        return {"summary": f"Test failed for indexer '{indexer_resource_response.get('name', id)}'.", "status": "error", "details": test_response}

    # If the response is a dict and not an error, it might contain validation results
    if isinstance(test_response, dict):
        # Prowlarr's test response for a single indexer usually returns a validation object.
        # Example: { "isValid": true, "message": "Test successful", "errors": [] } or something similar.
        # We adapt to a common pattern if the exact schema isn't in OpenAPI for this response.
        is_valid = test_response.get('isValid', True) # Assume valid if not specified but no error
        message = test_response.get('message', 'Test completed.')
        if not is_valid and not test_response.get('errors'): # If not valid but no errors, make a generic error message
            message = test_response.get('message', 'Test reported as not valid, but no specific errors provided.')


        return {
            "summary": f"Test for indexer '{indexer_resource_response.get('name', id)}': {message}", 
            "status": "success" if is_valid else "error",
            "details": test_response
        }

    logger.warning(f"Unexpected response type from Prowlarr API for test_indexer: {type(test_response)}. Response: {test_response}")    
    return {"summary": f"Test for indexer '{indexer_resource_response.get('name', id)}' completed. Response format was unexpected.", "status": "unknown", "raw_response": test_response}


@mcp.tool()
async def list_applications() -> Dict[str, Any]:
    """
    Retrieves a list of all applications (e.g., Sonarr, Radarr) synced with Prowlarr.
    Returns a summary and the full list of ApplicationResource objects.
    """
    logger.info("Listing all applications...")
    response = await _prowlarr_api_request("GET", f"/api/{API_VERSION}/applications")

    if isinstance(response, dict) and "error" in response:
        return response

    if isinstance(response, list):
        summary = f"Found {len(response)} applications. "
        if response:
            summary += "Applications: " + ", ".join([f"{app.get('name', 'N/A')} (ID: {app.get('id', 'N/A')}, Type: {app.get('implementationName', 'N/A')})" for app in response[:5]])
        else:
            summary += "No applications configured."
        
        return {"summary": summary, "applications": response}

    logger.error(f"Unexpected response type from Prowlarr API for list_applications: {type(response)}")
    return {"error": "Failed to list applications due to unexpected API response."}

@mcp.tool()
async def get_system_status() -> Dict[str, Any]:
    """
    Retrieves general system status and information about the Prowlarr instance.
    Returns a human-readable summary and the full SystemResource object.
    """
    logger.info("Getting Prowlarr system status...")
    response = await _prowlarr_api_request("GET", f"/api/{API_VERSION}/system/status")

    if isinstance(response, dict) and "error" in response:
        return response

    if isinstance(response, dict):
        summary = (
            f"Prowlarr Version: {response.get('version', 'N/A')} (Branch: {response.get('branch', 'N/A')})\n"
            f"OS: {response.get('osName', 'N/A')} {response.get('osVersion', 'N/A')}\n"
            f"Runtime: {response.get('runtimeName', 'N/A')} {response.get('runtimeVersion', 'N/A')}\n"
            f"AppData: {response.get('appData', 'N/A')}\n"
            f"Startup Time: {response.get('startTime', 'N/A')}\n"
            f"Docker: {'Yes' if response.get('isDocker') else 'No'}"
        )
        return {"summary": summary, "status": response}

    logger.error(f"Unexpected response type from Prowlarr API for get_system_status: {type(response)}")
    return {"error": "Failed to get system status due to unexpected API response."}

@mcp.tool()
async def get_indexer_categories() -> Dict[str, Any]:
    """
    Retrieves the list of default indexer categories available in Prowlarr.
    Returns a summary and the full list of IndexerCategory objects.
    """
    logger.info("Getting Prowlarr indexer categories...")
    response = await _prowlarr_api_request("GET", f"/api/{API_VERSION}/indexer/categories")

    if isinstance(response, dict) and "error" in response:
        return response

    if isinstance(response, list):
        summary = f"Found {len(response)} top-level indexer categories."
        # Could add more detail here if needed, e.g., names of first few.
        return {"summary": summary, "categories": response}

    logger.error(f"Unexpected response type from Prowlarr API for get_indexer_categories: {type(response)}")
    return {"error": "Failed to get indexer categories due to unexpected API response."}

@mcp.tool()
async def get_history(
    page: Optional[int] = 1, 
    pageSize: Optional[int] = 20, 
    sortKey: Optional[str] = "date", # Common default for history
    sortDirection: Optional[str] = "descending", # Common default
    eventType: Optional[List[int]] = None, # Array of HistoryEventType enum values
    successful: Optional[bool] = None,
    downloadId: Optional[str] = None,
    indexerIds: Optional[List[int]] = None
) -> Dict[str, Any]:
    """
    Retrieves Prowlarr's history records (grabs, queries, etc.) with pagination and filtering.
    Returns a summary and the HistoryResourcePagingResource object.
    """
    logger.info(f"Getting Prowlarr history: Page {page}, PageSize {pageSize}, Sort: {sortKey} {sortDirection}")
    params = {
        "page": page,
        "pageSize": pageSize,
        "sortKey": sortKey,
        "sortDirection": sortDirection,
    }
    if eventType is not None:
        params["eventType"] = ",".join(map(str, eventType))
    if successful is not None:
        params["successful"] = str(successful).lower() # API expects boolean as string
    if downloadId is not None:
        params["downloadId"] = downloadId
    if indexerIds is not None:
        params["indexerIds"] = ",".join(map(str, indexerIds))

    response = await _prowlarr_api_request("GET", f"/api/{API_VERSION}/history", params=params)

    if isinstance(response, dict) and "error" in response:
        return response

    if isinstance(response, dict) and "records" in response:
        records = response.get("records", [])
        total_records = response.get("totalRecords", 0)
        summary = f"Retrieved {len(records)} history events (Page {response.get('page')}/{ (total_records // response.get('pageSize', 1)) +1 if response.get('pageSize',1) > 0 else 1}, Total: {total_records})."
        if records:
            summary += " First few events:"
            for rec in records[:3]:
                event_type_str = rec.get('eventType', 'N/A') 
                # Ideally map HistoryEventType enum to string here if known
                date_str = rec.get('date', 'N/A')
                indexer_name = rec.get('data', {}).get('indexerName', rec.get('indexerId', 'N/A')) # Attempt to get name
                summary += f"\n  - Event: {event_type_str}, Indexer: {indexer_name}, Date: {date_str}, Successful: {rec.get('successful', 'N/A')}"
        
        return {"summary": summary, "history_page": response}

    logger.error(f"Unexpected response type from Prowlarr API for get_history: {type(response)}")
    return {"error": "Failed to get history due to unexpected API response."}


@mcp.tool()
async def test_all_indexers() -> Dict[str, Any]:
    """
    Triggers a test for all configured and enabled indexers in Prowlarr.
    Returns a message indicating the tests have been initiated and a summary of results.
    Detailed results from Prowlarr are included in the 'details' field.
    """
    logger.info("Initiating test for all indexers...")
    response = await _prowlarr_api_request("POST", f"/api/{API_VERSION}/indexer/testall")

    if response is None: # Should not happen based on recent test, but good to keep as a fallback
        return {"summary": "Test all indexers command sent, but Prowlarr returned no specific results (e.g., 204 No Content). Check Prowlarr UI.", "status": "unknown"}
    
    if isinstance(response, dict) and "error" in response:
        return response # Return error as is

    if isinstance(response, list):
        passed_count = 0
        failed_count = 0
        failed_details = []
        total_count = len(response)

        for item in response:
            if isinstance(item, dict) and item.get("isValid") is True:
                passed_count += 1
            else:
                failed_count += 1
                idx_id = item.get("id", "Unknown ID")
                # Prowlarr's actual validationFailures structure for testall isn't fully clear from current data,
                # assuming it might be a list of objects with an 'errorMessage' or similar.
                # For now, just capture the whole item if it's not valid.
                failures = item.get("validationFailures")
                error_message = "Test failed."
                if failures and isinstance(failures, list) and len(failures) > 0:
                    # Try to get a more specific message if available
                    first_failure = failures[0]
                    if isinstance(first_failure, dict) and first_failure.get("errorMessage"):
                        error_message = first_failure.get("errorMessage")
                    else:
                        error_message = str(failures) # fallback to string of failures list
                elif isinstance(item, dict) and item.get("message"):
                     error_message = item.get("message")

                failed_details.append(f"Indexer ID {idx_id}: {error_message}")

        summary = f"Tested {total_count} indexers. Passed: {passed_count}, Failed: {failed_count}."
        if failed_count > 0:
            summary += " Failures: " + "; ".join(failed_details)
        
        return {"summary": summary, "status": "success" if failed_count == 0 else "partial_success", "details": response}

    logger.warning(f"Unexpected response structure from Prowlarr API for test_all_indexers: {type(response)}")
    return {"summary": "Test all indexers command sent. Response structure was unexpected.", "status": "unknown", "raw_response": response}

@mcp.tool()
async def update_indexer(id: int, indexer_config: Dict[str, Any]) -> Dict[str, Any]:
    """
    Updates an existing indexer's configuration by its ID.
    Requires the full IndexerResource object (or a subset of fields to update) in the request body.
    Note: The exact structure of indexer_config might need to be precise.
    """
    logger.info(f"Updating indexer ID: {id} with config: {indexer_config}")
    response = await _prowlarr_api_request("PUT", f"/api/{API_VERSION}/indexer/{id}", json_body=indexer_config)
    
    if isinstance(response, dict) and "error" in response:
        return response

    if isinstance(response, dict): # Expecting the updated IndexerResource
        return {"summary": f"Indexer '{response.get('name', id)}' updated successfully.", "details": response}

    logger.error(f"Unexpected response type from Prowlarr API for update_indexer: {type(response)}")
    return {"error": "Failed to update indexer due to unexpected API response."}

# --- Main Execution ---
if __name__ == "__main__":
    logger.info(f"Starting Prowlarr MCP Server...")
    
    if PROWLARR_MCP_TRANSPORT == 'sse':
        mcp.run(
            transport='sse',
            host=PROWLARR_MCP_HOST,
            port=PROWLARR_MCP_PORT,
            path='/mcp'  # Standardized path for MCP over SSE
        )
    elif PROWLARR_MCP_TRANSPORT == 'stdio':
        mcp.run() # Defaults to stdio
    else:
        logger.error(f"Invalid PROWLARR_MCP_TRANSPORT: '{PROWLARR_MCP_TRANSPORT}'. Must be 'sse' or 'stdio'. Defaulting to STDIO.")
        mcp.run() 