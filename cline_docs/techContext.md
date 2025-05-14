# Technical Context

## Core Technologies Used

*   **Primary Language:** Python (3.10+)
*   **MCP Framework:** FastMCP (v2.x)
*   **HTTP Client:** `httpx` (for custom API clients for Overseerr, SABnzbd, Tautulli)
*   **Plex API Library:** `plexapi`
*   **qBittorrent API Library:** `qbittorrent-api`
*   **Environment Management:** `python-dotenv` (for `.env` file loading)
*   **Protocol:** Model Context Protocol (MCP)
*   **Package/Environment Manager:** `uv`

## Development Setup & Structure

1.  **Project Root (`mcplex/`):** Contains main configuration like `.env`, `pyproject.toml`, and the `src/` directory.
2.  **Source Directory (`src/`):** Houses subdirectories for each individual MCP server module (e.g., `plex-mcp/`, `overseerr-mcp/`, etc.).
3.  **Server Module Structure (e.g., `src/overseerr-mcp/`):
    *   `__init__.py`: Marks the directory as a Python package.
    *   `server.py`: Contains the FastMCP application instance, tool definitions, and lifespan management for that specific service.
    *   `client.py`: (Where applicable) Contains a custom API client class for interacting with the service's API (typically using `httpx`).
    *   `README.md`: Specific setup, run, and tool information for that server.
4.  **Virtual Environment:** Managed by `uv` (e.g., `.venv/` in project root), activated via `source .venv/bin/activate`.
5.  **Dependencies:** Defined in `pyproject.toml` and installed using `uv add ...` or `uv sync`.
6.  **Configuration:** Sensitive URLs, API keys, and credentials for all services are stored in a single `.env` file in the project root and loaded by each server script at startup.

## Key Technical Constraints & Learnings

*   **Python Import Resolution:**
    *   When running server scripts directly via `fastmcp run src/xxx-mcp/server.py:mcp`, and these scripts need to import sibling modules (e.g., `client.py`), a reliable method is to have `server.py` explicitly add its own directory to `sys.path` at the very beginning (`SCRIPT_DIR = Path(__file__).resolve().parent; if str(SCRIPT_DIR) not in sys.path: sys.path.insert(0, str(SCRIPT_DIR))`) and then use a direct import (`from client import ...`).
    *   The alternative `python -m src.xxx-mcp.server` also works and handles relative imports (`from .client import ...`) correctly, provided the MCP client executes this command from the project root or ensures the project root is on `PYTHONPATH`.
    *   Static analysis tools like Pylance may require additional configuration (e.g., `python.analysis.extraPaths` in VSCode settings or `pyrightconfig.json`) to correctly resolve imports that rely on runtime `sys.path` modifications or project structure not immediately obvious to the analyzer.
*   **Asynchronous vs. Synchronous Libraries:**
    *   For asynchronous operations (network I/O), `httpx.AsyncClient` is used in custom clients.
    *   When using synchronous libraries like `qbittorrent-api` within an async FastMCP server, their blocking calls must be run in a thread executor: `await asyncio.get_event_loop().run_in_executor(executor, blocking_func, ...)`. A shared `ThreadPoolExecutor` is instantiated in such server modules.
*   **API Client Management:** API clients are initialized and (if necessary) closed using FastMCP's lifespan context manager (`@asynccontextmanager` function passed to `FastMCP(lifespan=...)`). The client instance is stored on the `FastMCP` app instance (e.g., `app.service_client`) and accessed by tools via `ctx.fastmcp.service_client`.
*   **Server Execution:** Servers are primarily run via `fastmcp run path/to/server.py:mcp` as configured in the MCP client (e.g., `cline_mcp_settings.json`), using STDIO transport.

## Technical Constraints

*   Requires network access to the user's Plex Media Server.
*   Requires Plex authentication credentials (e.g., token, username/password) which must be handled securely.
*   Functionality is limited by the capabilities of the `python-plexapi` library and the Plex API itself.
*   Server needs to run persistently for an MCP host to connect.

# Tech Context: Unifi Site Manager MCP Server

## Technologies Used
- **Programming Language**: Python (version 3.9+ for async features, though specific version not strictly enforced by current code, 3.13 used in dev environment)
- **MCP Framework**: FastMCP
- **HTTP Client**: `httpx` (for asynchronous API requests to the Unifi API)
- **Environment Management**: `python-dotenv` (for loading API keys and URLs from a `.env` file)
- **Standard Libraries**: `os`, `sys`, `asyncio`, `time`, `logging`, `typing`.

## Development Setup
- **Virtual Environment**: Recommended (e.g., using `venv` or `uv`).
- **Dependencies**: Listed in `src/unifi-mcp/requirements.txt` (`fastmcp`, `httpx`, `python-dotenv`). Install with `pip install -r requirements.txt`.
- **Configuration**: A `.env` file in `src/unifi-mcp/` is required, based on `.env.example`. It must contain:
    - `UNIFI_API_KEY`: Your Unifi Site Manager API key.
    - `UNIFI_BASE_URL`: The base URL for the Unifi API (defaults to `https://api.ui.com`).
    - `MCP_PORT` (optional): Port for the SSE server (defaults to 3000).
    - `MCP_HOST` (optional): Host for the SSE server (defaults to `0.0.0.0`).
- **Running the Server**: Execute `python src/unifi-mcp/unifi_mcp_server.py` from the project root or `python unifi_mcp_server.py` from within `src/unifi-mcp/`.

## Technical Constraints & Considerations
- **API Rate Limits**: The Unifi Site Manager API has rate limits (10,000 requests/minute for v1, 100 requests/minute for Early Access endpoints). The server includes basic `Retry-After` handling.
- **Early Access (EA) Endpoints**: Several tools rely on `/ea/` API endpoints which are subject to change by Ubiquiti and have stricter rate limits.
- **Automatic Pagination**: Fetching all pages for list operations can be time-consuming and memory-intensive for Unifi setups with a very large number of hosts, sites, or devices.
- **SSE Transport**: While enabling remote access, SSE requires appropriate network configuration if accessed outside localhost (firewalls, port forwarding). Security of the exposed SSE endpoint should be considered.
- **Error Propagation**: API errors are logged and raised as exceptions, which FastMCP should then report to the client. The detail of these error messages as seen by the end-user might need refinement based on usage.
- **No Local API Client State**: Unlike some other MCP servers in the `mcplex` project that might initialize and hold a client object (e.g., `plexapi` instance), this server makes direct `httpx` calls in each tool via the `_make_api_request` helper, managing authentication per call. There's no persistent client session object stored on the `FastMCP` instance.

# Technical Context: yarr-mcp

## Technologies Used

*   **Primary Language**: Python (Version 3.10+ recommended, 3.11+ preferred).
*   **MCP Framework**: FastMCP library for building Model Context Protocol servers.
*   **Environment Management**: `python-dotenv` library for loading configuration from `.env` files.
*   **HTTP Client**: `httpx` is commonly used by underlying service-specific libraries or directly for API interactions within servers (often abstracted by FastMCP).
*   **Package Management**: `uv` (from Astral) for Python package installation, resolution, and virtual environment management. Project dependencies are defined in `pyproject.toml`.
*   **Version Control**: Git, with the project hosted on GitHub (github.com/jmagar/yarr-mcp).
*   **Documentation**: Markdown for `README.md` files, tool test reports, and internal documentation (like these memory bank files).
*   **Operating System (Development)**: Primarily developed and tested in a WSL (Windows Subsystem for Linux) environment (Ubuntu).

## Development Setup

1.  **Prerequisites**:
    *   Python 3.10+ installed.
    *   `uv` installed (`curl -LsSf https://astral.sh/uv/install.sh | sh` or `pip install uv`).
    *   Git installed.
2.  **Clone Repository**:
    ```bash
    git clone https://github.com/jmagar/yarr-mcp.git # Or your fork
    cd yarr-mcp
    ```
3.  **Virtual Environment**:
    *   Create: `uv venv`
    *   Activate (Linux/macOS): `source .venv/bin/activate`
    *   Activate (Windows): `.venv\Scripts\activate`
4.  **Install Dependencies**:
    ```bash
    uv pip install -e .
    ```
    This installs the `yarr-mcp` package itself (if defined as such in `pyproject.toml`) and all its dependencies.
5.  **Configuration**:
    *   Create a `.env` file in the project root (`yarr-mcp/.env`) by copying from an example or creating anew.
    *   Populate this file with API URLs, keys/credentials for the target media services, and any desired MCP server-specific settings (transport, host, port, log level, log file).
6.  **Running a Server**:
    *   Navigate to the project root if not already there.
    *   Execute the desired server script, for example:
        ```bash
        python src/plex-mcp/plex-mcp-server.py
        ```
7.  **Testing**:
    *   Individual server tools are tested by running the server and using an MCP client (or programmatic calls) to invoke the tools.
    *   Results are documented in `<service>-mcp-tools-test-results.md` files.

## Technical Constraints & Considerations

*   **Template Adherence**: All servers must align with the structure and conventions defined in `src/create-mcp-server_v2.md`.
*   **API Response Handling**: Servers must be robust in handling varied API responses from target services (e.g., JSON, sometimes direct strings, error structures).
*   **Environment Path Issues**: Care must be taken in WSL or similar environments to ensure the correct Python interpreter and paths are used, avoiding conflicts with system or other Python installations (e.g., Windows `pyenv-win` shims interfering with WSL Python). The virtual environment should correctly prioritize its Python executable.
*   **FastMCP API Versioning**: The version of FastMCP used may have specific API call patterns (e.g., `mcp.run()` vs. the older `mcp.run_server()`). Code must be compatible with the installed version.
*   **Error Handling**: Tools should implement try-except blocks to catch errors during API calls or processing and return informative error messages to the MCP client, while also logging detailed errors server-side. 