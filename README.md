# mcplex - Media Control Plex & Friends MCP Servers

A collection of FastMCP servers that allow interaction with Plex and related media ecosystem applications via the Model Context Protocol (MCP).
This enables MCP-compatible clients (like LLM applications) to query and control these services.

## Available MCP Servers

This project currently includes the following MCP servers:

1.  **Plex MCP Server (`plex-mcp`)**
    *   Allows direct interaction with a Plex Media Server.
    *   Features: List libraries, search media, play media on clients, get server info, list all library titles, etc.
    *   See `src/plex-mcp/README.md` for detailed setup and usage.

2.  **Overseerr MCP Server (`overseerr-mcp`)**
    *   Interacts with an Overseerr instance for media requests and discovery.
    *   Features: Search media, get movie/TV details, request media, list pending/failed requests.
    *   See `src/overseerr-mcp/README.md` for detailed setup and usage.

3.  **SABnzbd MCP Server (`sabnzbd-mcp`)**
    *   Manages downloads in a SABnzbd instance.
    *   Features: View queue/history, pause/resume queue, add NZB by URL, set speed limit.
    *   See `src/sabnzbd-mcp/README.md` for detailed setup and usage.

4.  **qBittorrent MCP Server (`qbittorrent-mcp`)**
    *   Manages torrents in a qBittorrent instance.
    *   Features: List torrents, add torrents by URL, pause/resume torrents, get transfer info and app preferences.
    *   See `src/qbittorrent-mcp/README.md` for detailed setup and usage.

5.  **Tautulli MCP Server (`tautulli-mcp`)**
    *   Retrieves Plex statistics and activity from a Tautulli instance.
    *   Features: Get current activity, home stats, watch history, list libraries and users.
    *   See `src/tautulli-mcp/README.md` for detailed setup and usage.

## General Setup (for all servers)

1.  **Clone Repository:**
    ```bash
    git clone <your_repository_url> # Or ensure you are in the project root
    cd mcplex
    ```
2.  **Install Dependencies:**
    *   Requires Python 3.10+.
    *   Install `uv` (Python package manager, recommended): `curl -LsSf https://astral.sh/uv/install.sh | sh`
    *   Create/activate virtual environment:
        ```bash
        uv venv
        source .venv/bin/activate
        ```
    *   Install all project dependencies:
        ```bash
        uv pip install -r requirements.txt # If you generate one
        # OR sync with pyproject.toml if it includes all dependencies
        uv sync 
        # OR install individually if needed (already done during development of each module)
        # uv add fastmcp httpx python-dotenv qbittorrent-api plexapi
        ```
3.  **Configuration (`.env` file):**
    Create a `.env` file in the `mcplex` project root. Add the necessary URL and API Key/credentials for each service you intend to use. Refer to the individual README files in each `src/*-mcp` directory for the specific environment variable names required (e.g., `PLEX_URL`, `OVERSEERR_URL`, `SABNZBD_API_KEY`, etc.).

    Example `.env` structure:
    ```env
    PLEX_URL=http://plex_host:32400
    PLEX_TOKEN=your_plex_token
    
    OVERSEERR_URL=http://overseerr_host:5055
    OVERSEERR_API_KEY=your_overseerr_api_key
    
    SABNZBD_URL=http://sabnzbd_host:8080
    SABNZBD_API_KEY=your_sabnzbd_api_key
    
    QBITTORRENT_URL=http://qbittorrent_host:8080
    QBITTORRENT_USER=your_qb_username
    QBITTORRENT_PASS=your_qb_password
    
    TAUTULLI_URL=http://tautulli_host:8181
    TAUTULLI_API_KEY=your_tautulli_api_key
    ```

## Running the Servers

Each MCP server is run as a separate process. Make sure your virtual environment is activated.

*   **Plex Server:**
    ```bash
    fastmcp run src/plex-mcp/server.py:mcp 
    # or python -m src.plex-mcp.server
    ```
*   **Overseerr Server:**
    ```bash
    fastmcp run src/overseerr-mcp/server.py:mcp
    # or python -m src.overseerr-mcp.server
    ```
*   **SABnzbd Server:**
    ```bash
    fastmcp run src/sabnzbd-mcp/server.py:mcp
    # or python -m src.sabnzbd-mcp.server
    ```
*   **qBittorrent Server:**
    ```bash
    fastmcp run src/qbittorrent-mcp/server.py:mcp
    # or python -m src.qbittorrent-mcp.server
    ```
*   **Tautulli Server:**
    ```bash
    fastmcp run src/tautulli-mcp/server.py:mcp
    # or python -m src.tautulli-mcp.server
    ```

Refer to the individual README files in each `src/*-mcp` directory for more details on tools and specific configurations.

## MCP Client Usage

Connect your MCP host/client application to the desired running server(s) using the STDIO transport method. The client will typically need the command used to run the server (e.g., `fastmcp run src/plex-mcp/server.py:mcp`).
