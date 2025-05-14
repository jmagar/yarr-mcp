# yarr-mcp - ARRs & Friends MCP Servers 🏴‍☠️

A swashbuckling collection of FastMCP servers that allow interaction with Plex and a whole treasure trove of related media ecosystem applications via the Model Context Protocol (MCP).
This enables MCP-compatible clients (like LLM applications, AI assistants, or your own custom scripts) to query and control these services with ease.

## Available MCP Servers 🚢

This project currently includes the following MCP servers:

1.  **Plex MCP Server (`plex-mcp`)** 🎬
    *   Allows direct interaction with a Plex Media Server.
    *   Features: List libraries, search media, play media on clients, get server info, list all library titles, comprehensive media stats, and more.
    *   See `src/plex-mcp/README.md` for detailed setup and usage.

2.  **Overseerr MCP Server (`overseerr-mcp`)**  درخواست
    *   Interacts with an Overseerr instance for media requests and discovery.
    *   Features: Search media, get movie/TV details, request media, list pending/failed requests.
    *   See `src/overseerr-mcp/README.md` for detailed setup and usage.

3.  **SABnzbd MCP Server (`sabnzbd-mcp`)** 📨
    *   Manages downloads in a SABnzbd instance.
    *   Features: View queue/history, pause/resume queue, add NZB by URL, set speed limit.
    *   See `src/sabnzbd-mcp/README.md` for detailed setup and usage.

4.  **qBittorrent MCP Server (`qbittorrent-mcp`)** 🧲
    *   Manages torrents in a qBittorrent instance.
    *   Features: List torrents, add torrents by URL, pause/resume torrents, get transfer info and app preferences.
    *   See `src/qbittorrent-mcp/README.md` for detailed setup and usage.

5.  **Tautulli MCP Server (`tautulli-mcp`)** 📊
    *   Retrieves Plex statistics and activity from a Tautulli instance.
    *   Features: Get current activity, home stats, watch history, list libraries and users.
    *   See `src/tautulli-mcp/README.md` for detailed setup and usage.

6.  **Portainer MCP Server (`portainer-mcp`)** 🐳
    *   Manages Docker environments through a Portainer instance.
    *   Features: List endpoints, inspect containers & stacks, manage container lifecycle (start/stop/restart), get container logs, and retrieve stack files.
    *   See `src/portainer-mcp/README.md` for detailed setup and usage.

7.  **Prowlarr MCP Server (`prowlarr-mcp`)** 📡
    *   Interacts with a Prowlarr instance to manage indexers and search for releases.
    *   Features: List indexers, get indexer details, search releases, test indexers, list applications, and check system status.
    *   See `src/prowlarr-mcp/README.md` for detailed setup and usage.

8.  **Unifi MCP Server (`unifi-mcp`)** 🌐
    *   Connects to the Unifi Site Manager API to provide insights into your network.
    *   Features: List hosts, sites, devices, get ISP metrics (EA), and manage SD-WAN configurations (EA).
    *   See `src/unifi-mcp/README.md` for detailed setup and usage.

9.  **Unraid MCP Server (`unraid-mcp`)** 💾
    *   Interfaces with an Unraid server's GraphQL API for system information and management.
    *   Features: Get system info, array status, network config, Docker container/VM management, list shares, notifications, and logs.
    *   See `src/unraid-mcp/README.md` for detailed setup and usage.

10. **Gotify MCP Server (`gotify-mcp`)** 🔔
    *   Sends messages and manages a Gotify push notification server.
    *   Features: Create messages (requires app_token per call), manage applications and clients, get server health/version.
    *   See `src/gotify-mcp/README.md` for detailed setup and usage.

## General Setup (for all servers) ⚙️

1.  **Clone Repository:**
    ```bash
    git clone https://github.com/jmagar/yarr-mcp # Or your fork's URL
    cd yarr-mcp
    ```
2.  **Install Dependencies:**
    *   Requires Python 3.10+ (preferably 3.11+).
    *   Install `uv` (a very fast Python package installer & resolver):
        ```bash
        curl -LsSf https://astral.sh/uv/install.sh | sh
        # Or pip install uv
        ```
    *   Create and activate a virtual environment:
        ```bash
        uv venv
        source .venv/bin/activate  # For Linux/macOS
        # .venv\Scripts\activate    # For Windows
        ```
    *   Install all project dependencies from `pyproject.toml`:
        ```bash
        uv pip install -e .
        # This command installs the yarr-mcp package itself in editable mode 
        # and all its dependencies defined in pyproject.toml.
        ```
3.  **Configuration (`.env` file):**
    Create a `.env` file in the `yarr-mcp` project root. Add the necessary URL and API Key/credentials for each service you intend to use.
    Refer to the individual README files in each `src/*-mcp` directory for specific variable names and details if needed, but the general structure is below.

    Example `.env` structure (include only the services you use):
    ```env
    # --- Plex ---
    PLEX_URL=http://plex_host:32400
    PLEX_TOKEN=your_plex_token
    # PLEX_MCP_TRANSPORT=sse
    # PLEX_MCP_HOST=0.0.0.0
    # PLEX_MCP_PORT=8000
    # PLEX_LOG_LEVEL=INFO

    # --- Overseerr ---
    OVERSEERR_URL=http://overseerr_host:5055
    OVERSEERR_API_KEY=your_overseerr_api_key
    # OVERSEERR_MCP_PORT=8001
    # OVERSEERR_LOG_LEVEL=INFO

    # --- SABnzbd ---
    SABNZBD_URL=http://sabnzbd_host:8080
    SABNZBD_API_KEY=your_sabnzbd_api_key
    # SABNZBD_MCP_PORT=8004 # Example, check actual default in script
    # SABNZBD_LOG_LEVEL=INFO

    # --- qBittorrent ---
    QBITTORRENT_URL=http://qbittorrent_host:8080
    QBITTORRENT_USER=your_qb_username
    QBITTORRENT_PASS=your_qb_password
    # QBITTORRENT_MCP_PORT=8003 # Example, check actual default
    # QBITTORRENT_LOG_LEVEL=INFO

    # --- Tautulli ---
    TAUTULLI_URL=http://tautulli_host:8181
    TAUTULLI_API_KEY=your_tautulli_api_key
    # TAUTULLI_MCP_PORT=8002
    # TAUTULLI_LOG_LEVEL=INFO

    # --- Portainer ---
    PORTAINER_URL=http://portainer_host:9000 # Or :9443 for HTTPS
    PORTAINER_API_KEY=your_portainer_api_key
    # PORTAINER_MCP_PORT=6971
    # PORTAINER_MCP_LOG_LEVEL=INFO
    # PORTAINER_MCP_LOG_FILE=portainer_mcp.log

    # --- Prowlarr ---
    PROWLARR_URL=http://prowlarr_host:9696
    PROWLARR_API_KEY=your_prowlarr_api_key
    # PROWLARR_MCP_PORT=6973
    # PROWLARR_MCP_LOG_LEVEL=INFO
    # PROWLARR_MCP_LOG_FILE=prowlarr_mcp.log

    # --- Unifi ---
    UNIFI_BASE_URL=https://api.ui.com # Default, or your self-hosted controller URL if applicable
    UNIFI_API_KEY=your_unifi_api_key
    # UNIFI_MCP_PORT=6969
    # UNIFI_MCP_LOG_LEVEL=INFO
    # UNIFI_MCP_LOG_FILE=unifi_mcp.log

    # --- Unraid ---
    UNRAID_API_URL=http://your-unraid-server-ip/graphql # Or https if configured
    UNRAID_API_KEY=your_unraid_api_key
    # UNRAID_VERIFY_SSL=true # Set to false for self-signed certs, or path to CA bundle
    # UNRAID_MCP_PORT=6970
    # UNRAID_MCP_LOG_LEVEL=INFO
    # UNRAID_MCP_LOG_FILE=unraid_mcp.log

    # --- Gotify ---
    GOTIFY_URL=http://gotify_host
    GOTIFY_CLIENT_TOKEN=your_gotify_client_token # For management operations
    # GOTIFY_APP_TOKEN=your_default_app_token # For sending messages if a default is desired, but server expects app_token per call
    # GOTIFY_MCP_PORT=8000 # Check default in gotify-mcp-server.py if different
    # GOTIFY_LOG_LEVEL=INFO
    # GOTIFY_MCP_LOG_FILE=gotify_mcp.log

    # General MCP settings (can be overridden per service as above)
    # LOG_LEVEL=DEBUG # For verbose logging across all MCP servers if not overridden
    ```

## Running with Docker and Docker Compose (Recommended)

This document explains how to build and run the `yarr-mcp` application using `docker compose`. This setup allows you to run multiple MCP (Model Context Protocol) services, each configurable via environment variables loaded from an `.env` file.

### Prerequisites

- Docker installed on your system.
- Docker Compose (V2 CLI plugin) installed on your system.

### Setup

1.  **Clone the Repository:** If you haven't already, clone the `yarr-mcp` project.
    ```bash
    # git clone https://github.com/jmagar/yarr-mcp
    cd yarr-mcp
    ```

2.  **Create `.env` File:**
    Copy the example environment file `.env.example` to `.env`:
    ```bash
    cp .env.example .env
    ```
    Edit the `.env` file and fill in your specific configurations for each service you intend to use. This includes API keys, URLs, hostnames, and ports. Pay close attention to the `SERVICENAME_MCP_PORT` variables, as these are used by `docker-compose.yml` to map ports to your host.

### Building and Running with Docker Compose

Navigate to the root directory of the `yarr-mcp` project (where `docker-compose.yml` is located).

1.  **Build and Start Services:**
    To build the Docker image (if it doesn't exist or if `Dockerfile` has changed) and start all configured services in detached mode, run:
    ```bash
    docker compose up --build -d
    ```
    If you only want to start the services without rebuilding (assuming the image is already built and up-to-date):
    ```bash
    docker compose up -d
    ```

2.  **Stopping Services:**
    To stop the running services:
    ```bash
    docker compose down
    ```

3.  **Viewing Logs:**
    To view the combined logs from all services managed by `docker compose`:
    ```bash
    docker compose logs -f
    ```
    To view logs for a specific service (the default service name is `yarr-mcp-app`):
    ```bash
    docker compose logs -f yarr-mcp-app
    ```

### Configuration via `.env` File

All configuration is managed through the `.env` file in the project root.

#### Disabling Services

By default, all MCP services are **enabled**. To disable a specific service, set its corresponding `SERVICENAME_MCP_DISABLE` variable to `true` in your `.env` file, directly under that service's configuration block.

Example from `.env`:
```env
# ... other variables for a service ...
# PLEX_URL=https://your-plex-url
# PLEX_TOKEN=your_plex_token
PLEX_MCP_DISABLE=false      # Plex service will be enabled
# ...

# ... other variables for another service ...
# SABNZBD_URL=https://your-sabnzbd-url
# SABNZBD_API_KEY=your_sabnzbd_api_key
SABNZBD_MCP_DISABLE=true      # Sabnzbd service will be disabled
# ...
```
If a `SERVICENAME_MCP_DISABLE` variable is not set or is set to `false` (or any value other than `true`, case-insensitive), the corresponding service will attempt to start.

#### Service-Specific Configuration

Each service requires its own set of environment variables for its specific operation (e.g., API URLs, tokens). These **must** be defined in your `.env` file.

Additionally, each service has:
- `SERVICENAME_MCP_HOST`: The host the MCP server will listen on *inside the container* (usually `0.0.0.0`).
- `SERVICENAME_MCP_PORT`: The port the MCP server will listen on *inside the container*. This value is crucial as `docker-compose.yml` uses it to map the service's port to your host machine.
- `SERVICENAME_MCP_DISABLE`: Set to `true` to disable the service, `false` or unset to enable.

**Example snippet from `.env`:**
```env
# Gotify MCP Service Configuration
GOTIFY_URL=http://your_gotify_instance:80
GOTIFY_APP_TOKEN=your_gotify_app_token_here
GOTIFY_CLIENT_TOKEN=your_gotify_client_token_here # Optional
GOTIFY_MCP_HOST=0.0.0.0
GOTIFY_MCP_PORT=6972
GOTIFY_LOG_LEVEL=INFO
GOTIFY_MCP_DISABLE=false

# Portainer MCP Service Configuration
PORTAINER_URL=http://your_portainer_instance:9000
PORTAINER_API_KEY=your_portainer_api_key_here
PORTAINER_MCP_HOST=0.0.0.0
PORTAINER_MCP_PORT=6971
PORTAINER_LOG_LEVEL=INFO
PORTAINER_MCP_DISABLE=false
```
Refer to `.env.example` for the full list of variables for all services.

#### Port Mapping

The `docker-compose.yml` file is configured to map the `SERVICENAME_MCP_PORT` for each service (as defined in your `.env` file) to the same port number on your host machine. The MCP services themselves listen on the `/mcp` path.

For example, if in your `.env` file you have:
```env
GOTIFY_MCP_PORT=6972
PORTAINER_MCP_PORT=6971
```
Then:
- The Gotify MCP service will be accessible at `http://localhost:6972/mcp` on your host.
- The Portainer MCP service will be accessible at `http://localhost:6971/mcp` on your host.

If a service is disabled via `SERVICENAME_MCP_DISABLE=true`, or if its `SERVICENAME_MCP_PORT` is not set in the `.env` file, the port mapping for that service in `docker-compose.yml` will effectively be ignored (as Docker won't be able to map to a non-existent or unassigned variable, or the internal service won't start).

Ensure the ports you define in `.env` are free on your host machine or adjust them as needed (though you would then need to adjust the left-hand side of the port mapping in `docker-compose.yml` if you wanted the host port to differ from the container port, which is not the default setup).

### Service List

The Docker setup can manage the following MCP services (found in `src/`):

- `gotify-mcp`
- `overseerr-mcp`
- `plex-mcp`
- `portainer-mcp`
- `prowlarr-mcp`
- `qbittorrent-mcp`
- `sabnzbd-mcp`
- `tautulli-mcp`
- `unifi-mcp`
- `unraid-mcp`

For each service, ensure its `_URL` (or equivalent), API keys/tokens, `_MCP_HOST`, `_MCP_PORT`, `_LOG_LEVEL`, and `_MCP_DISABLE` variables are correctly set in your `.env` file.

## Running the Servers 🚀

Each MCP server runs as a separate Python process. Ensure your virtual environment is activated (`source .venv/bin/activate`).

From the `yarr-mcp` project root, run the desired server(s):

*   **Plex Server:**
    ```bash
    python src/plex-mcp/plex-mcp-server.py
    ```
*   **Overseerr Server:**
    ```bash
    python src/overseerr-mcp/overseerr-mcp-server.py
    ```
*   **SABnzbd Server:**
    ```bash
    python src/sabnzbd-mcp/sabnzbd-mcp-server.py
    ```
*   **qBittorrent Server:**
    ```bash
    python src/qbittorrent-mcp/qbittorrent-mcp-server.py
    ```
*   **Tautulli Server:**
    ```bash
    python src/tautulli-mcp/tautulli-mcp-server.py
    ```
*   **Portainer Server:**
    ```bash
    python src/portainer-mcp/portainer-mcp-server.py
    ```
*   **Prowlarr Server:**
    ```bash
    python src/prowlarr-mcp/prowlarr-mcp-server.py
    ```
*   **Unifi Server:**
    ```bash
    python src/unifi-mcp/unifi-mcp-server.py
    ```
*   **Unraid Server:**
    ```bash
    python src/unraid-mcp/unraid-mcp-server.py
    ```
*   **Gotify Server:**
    ```bash
    python src/gotify-mcp/gotify-mcp-server.py
    ```

Most servers default to **SSE (Server-Sent Events)** transport. You can configure the transport (`_MCP_TRANSPORT`), host (`_MCP_HOST`), and port (`_MCP_PORT`) for each server via environment variables (see `.env` example).

Refer to the individual README files in each `src/*-mcp` directory for more specific details on their tools, unique configurations, and default ports.

## MCP Client Usage 🤖

Connect your MCP host/client application (e.g., Claude Desktop, a custom script) to the desired running server(s).

*   **For SSE (default for most):**
    Your client will need the SSE endpoint URL, typically `http://<server_host>:<server_port>/mcp`.
    For example, if Prowlarr MCP is running on `0.0.0.0:6973`, the SSE endpoint is `http://localhost:6973/mcp`.

    **Example Client Configuration (e.g., for a client like Cline in its `cline_mcp_settings.json`):**
    ```json
    {
      "mcpServers": {
        "my-service-mcp-sse": { // A descriptive name for your client's UI
          "url": "http://localhost:6973/mcp", // Replace with actual host, port, and /mcp path
          "timeout": 30 // Optional: timeout in seconds
        }
        // ... other SSE server configurations ...
      }
    }
    ```
    Ensure your MCP server (e.g., `prowlarr-mcp-server.py`) is running and accessible at the specified URL.

*   **For STDIO (if configured):**
    If a server is configured to use `STDIO` transport (e.g., `PLEX_MCP_TRANSPORT=stdio`), the client will typically need the command used to run the server.
    Example for Claude Desktop (if `plex-mcp` is using STDIO):
    ```json
    {
      "name": "Plex (yarr-mcp)",
      "command": ["python", "src/plex-mcp/plex-mcp-server.py"],
      "working_directory": "/path/to/your/yarr-mcp", // Absolute path to project root
      "environment": {
        "PLEX_MCP_TRANSPORT": "stdio" 
        // ... other necessary env vars from your .env file ...
      }
    }
    ```
    Ensure the `working_directory` is correct and all necessary environment variables from your `.env` file are passed to the client's execution environment if it doesn't inherit them or load the `.env` itself.

Consult your specific MCP client's documentation for connection instructions.
The servers log their transport mode, host, and port on startup.
