from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
import uvicorn
from dotenv import dotenv_values
from pathlib import Path
from typing import List, Dict, Optional
import re # For parsing URLs
import docker # Added for Docker logs
from docker.errors import NotFound as DockerNotFound, APIError as DockerAPIError # Added

app = FastAPI(
    title="Yarr-MCP WebUI Backend",
    description="Provides API endpoints for the Yarr-MCP dashboard.",
    version="0.1.0"
)

# --- CORS Configuration ---
origins = [
    "http://localhost:5173", # Vite dev server
    "http://127.0.0.1:5173", # Vite dev server
    # Add any other origins if needed (e.g., your production frontend URL)
]

app.add_middleware(
    CORSMiddleware,
    allow_origins=origins,
    allow_credentials=True,
    allow_methods=["*"], # Allows all methods
    allow_headers=["*"], # Allows all headers
)
# --- End CORS Configuration ---

@app.get("/")
async def read_root():
    return {"message": "Welcome to the Yarr-MCP WebUI Backend!"}

# --- Helper to parse .env and identify MCP services ---
def get_mcp_service_config() -> List[Dict[str, Optional[str]]]:
    # Load .env from the project root relative to this backend script
    # backend/main.py -> web-ui/backend/main.py -> web-ui/ -> project_root/
    project_root_env_path = Path(__file__).resolve().parent.parent.parent / ".env"
    config = dotenv_values(project_root_env_path)
    
    services = []
    service_names = set()

    # First pass to identify service prefixes
    for key in config:
        if key.endswith("_MCP_URL") or key.endswith("_URL") or key.endswith("_BASE_URL"):
            # Attempt to extract service name prefix
            # PLEX_MCP_URL -> PLEX
            # UNIFI_BASE_URL -> UNIFI
            # SABNZBD_URL -> SABNZBD
            match = re.match(r"([A-Z0-9_]+?)_(?:MCP_)?(?:BASE_)?URL", key)
            if match:
                service_names.add(match.group(1))
            elif key.endswith("_URL"): # Simpler cases like PLEX_URL
                 name_candidate = key.replace("_URL", "")
                 # Ensure this isn't part of a longer name like FOO_BAR_URL if FOO is also a service
                 if all(not name_candidate.startswith(sn + "_") for sn in service_names if sn != name_candidate):
                    service_names.add(name_candidate)


    for name in sorted(list(service_names)):
        service_details = {"name": name.capitalize()}
        
        # Determine URL key pattern
        url_key = f"{name}_MCP_URL"
        if url_key not in config:
            url_key = f"{name}_URL"
            if url_key not in config:
                 url_key = f"{name}_BASE_URL" # For Unifi

        raw_url = config.get(url_key)
        service_details["mcp_url"] = raw_url
        
        # Infer host from URL
        if raw_url:
            try:
                # Basic host extraction (can be improved with urllib.parse if complex URLs are expected)
                parsed_host = re.match(r"https?://([^:/]+)", raw_url)
                service_details["mcp_host_inferred"] = parsed_host.group(1) if parsed_host else None
            except Exception:
                service_details["mcp_host_inferred"] = None
        else:
            service_details["mcp_host_inferred"] = None

        service_details["mcp_port"] = config.get(f"{name}_MCP_PORT")
        
        disabled_str = config.get(f"{name}_MCP_DISABLE", "true").lower() # Default to disabled if not found
        service_details["enabled"] = disabled_str == "false"
        
        services.append(service_details)
        
    return services

@app.get("/api/mcp-services")
async def list_mcp_services():
    """
    Lists all configured MCP services, their inferred MCP host/port, and enabled status from the root .env file.
    """
    services = get_mcp_service_config()
    return services

# Placeholder for future endpoints
# @app.get("/api/services")
# async def get_services_status():
#     # Logic to read .env and check health endpoints
#     return {"message": "Service status endpoint placeholder"}

@app.get("/api/logs/yarr-mcp")
async def get_yarr_mcp_logs(tail: Optional[int] = 100, since: Optional[str] = None):
    """
    Retrieves logs from the yarr-mcp Docker container.
    `tail`: Number of lines from the end of the logs to retrieve.
    `since`: Show logs since a UNIX timestamp or relative time (e.g., '10m', '1h').
    """
    container_name = "yarr-mcp" # Corrected from "yarr-mcp-app"
    try:
        client = docker.from_env()
        container = client.containers.get(container_name)
        
        log_params = {
            "stdout": True,
            "stderr": True,
            "timestamps": True, # Include timestamps
            "tail": tail if tail is not None else "all" # Default to all if tail is not specified, or use provided value
        }
        if since:
            log_params["since"] = since
            
        logs = container.logs(**log_params)
        return {"container_name": container_name, "logs": logs.decode('utf-8')}
    except DockerNotFound:
        return {"error": f"Container '{container_name}' not found."}
    except DockerAPIError as e:
        return {"error": f"Docker API error: {str(e)}"}
    except Exception as e:
        return {"error": f"An unexpected error occurred: {str(e)}"}

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8081, log_level="info") # Using port 8081 for the backend 