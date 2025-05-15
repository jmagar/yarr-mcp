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

# Technical Context: yarr-mcp Project & Dockerization

This document details the technologies used, development setup, and technical constraints for the `yarr-mcp` project, with a focus on its Dockerized deployment.

## Technologies Used

*   **Core Language:**
    *   **Python 3.13:** The MCP server applications are written in Python. The Docker image is specifically based on `python:3.13-slim`.
*   **Containerization & Orchestration:**
    *   **Docker:** Used to create a containerized environment for the `yarr-mcp` services.
    *   **Docker Compose (V2 CLI):** Used to define and manage the multi-service (conceptually, within a single container) application, including building the image, managing environment variables, and port mappings.
*   **Package Management (Python):**
    *   **`uv`:** A fast Python package installer and resolver, used for managing project dependencies as defined in `pyproject.toml`.
*   **Scripting:**
    *   **Bash:** Used for the `entrypoint.sh` script within the Docker container, which manages the startup of individual MCP services.
*   **Version Control:**
    *   **Git:** Used for source code management, with the project hosted on GitHub.
*   **MCP Framework (Inferred):**
    *   **FastMCP:** While not explicitly detailed in every part of the Dockerization discussion, the MCP services are assumed to be built using or compatible with the FastMCP framework principles.
*   **Operating System (Docker Base):**
    *   The `python:3.13-slim` image is Debian-based, so the services run in a Linux environment within the container.
*   **External Applications Interfaced With (Not part of `yarr-mcp` tech stack but crucial context):**
    *   Plex Media Server
    *   Overseerr
    *   SABnzbd
    *   qBittorrent
    *   Tautulli
    *   Portainer
    *   Prowlarr
    *   Unifi Controller (local or cloud)
    *   Unraid OS (via GraphQL API)
    *   Gotify

## Development and Deployment Setup

### Local Native Development (Pre-Docker or for individual service development)

*   **Python Environment:** Requires a Python 3.13 environment.
*   **Dependency Installation:** `uv venv` to create a virtual environment, `source .venv/bin/activate`, then `uv pip install -e .` to install project dependencies from `pyproject.toml`.
*   **Configuration:** Manual creation of an `.env` file in the project root, containing API keys, URLs, and MCP-specific settings (host, port, transport, log level) for each service being run.
*   **Running Services:** Individually running each `python src/SERVICENAME-mcp/SERVICENAME-mcp-server.py` script.

### Dockerized Deployment (Current Recommended Setup)

*   **Required Files:**
    *   `Dockerfile`: Defines the image build process.
    *   `entrypoint.sh`: Manages service startup within the container.
    *   `docker-compose.yml`: Defines the application service, build context, port mappings, and environment file.
    *   `.env.example`: Template for the `.env` file.
    *   `.env`: User-created file (from `.env.example`) containing all necessary runtime configurations and secrets.
    *   `pyproject.toml` & `uv.lock`: For Python dependency management during the Docker build.
*   **Build Process:** Initiated by `docker compose up --build -d`. `uv` is used within the `Dockerfile` to install Python packages.
*   **Running the Application:** `docker compose up -d` (after initial build).
*   **Stopping:** `docker compose down`.
*   **Logs:** `docker compose logs -f yarr-mcp-app`.
*   **Project Structure:** MCP service code resides in `src/SERVICENAME-mcp/` subdirectories.

## Technical Constraints & Considerations

*   **Single Docker Image:** The current architecture uses a single Docker image for all services to simplify initial deployment, rather than a microservice architecture with separate images per MCP service.
*   **Centralized Dependencies:** All Python dependencies must be compatible and managed through the single root `pyproject.toml`.
*   **Environment Variable Configuration:** The system relies entirely on environment variables for configuration at runtime. No hardcoded secrets or configurations within the image (beyond defaults in scripts that can be overridden).
*   **Service Discovery (Implicit):** The `entrypoint.sh` script contains a hardcoded list of service names to iterate through. Adding a new MCP service requires updating this script, the `Dockerfile` (to copy the new service code), and `docker-compose.yml` (for port mapping if needed) and `.env.example`.
*   **Port Management:** Users must ensure that the ports specified in their `.env` file (e.g., `PLEX_MCP_PORT`) are available on the host machine for mapping.
*   **Network Access:** The Docker container must have network access to reach the external applications (Plex, Unraid, etc.) that the MCP services interface with.
*   **MCP Path Convention:** Services are expected to expose their primary MCP endpoint at the `/mcp` path (e.g. `http://host:port/mcp`). 