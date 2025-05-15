# System Patterns

## Overall Architecture

The `mcplex` project comprises a suite of independent **Model Context Protocol (MCP) Servers**, each built using the **FastMCP framework** in Python. Each server is dedicated to a specific media-related application (Plex, Overseerr, SABnzbd, qBittorrent, Tautulli), providing a standardized MCP interface to its functionalities.

## Per-Server Architecture Pattern

Most individual MCP servers within this project follow a common structure:

1.  **Main Server Script (`server.py`):**
    *   Initializes a `FastMCP` instance.
    *   Manages configuration loading (URLs, API keys/credentials) from environment variables (via a root `.env` file and `python-dotenv`).
    *   Implements **FastMCP Lifespan Management** (`@asynccontextmanager`) to:
        *   Initialize an API client for the target service on server startup.
        *   Store the initialized API client instance on the FastMCP application instance (e.g., `app.service_client`).
        *   Handle cleanup (e.g., closing HTTP client sessions) on server shutdown.
    *   Defines **MCP Tools** (`@mcp.tool()`) that:
        *   Receive the FastMCP context (`ctx: Context`) to access the API client via `ctx.fastmcp.service_client`.
        *   Perform actions by calling methods on the API client.
        *   Handle responses and errors from the API client, returning JSON-serializable data or error strings.
    *   Includes basic logging for startup, tool execution, and errors.

2.  **API Client Module (`client.py`):** (For services not using a pre-existing comprehensive Python library like `plexapi` or `qbittorrent-api`)
    *   Encapsulates interactions with the target service's REST API.
    *   Typically uses `httpx.AsyncClient` for asynchronous HTTP requests.
    *   Handles API-specific details like base URL construction, authentication headers (e.g., `X-Api-Key`), query parameter formatting (including URL encoding where necessary), and parsing JSON responses.
    *   Provides methods that map to logical operations on the service (e.g., `get_queue()`, `search_media()`, `add_nzb_url()`).
    *   Includes error handling for HTTP status codes and request exceptions.

## Key Technical Decisions & Patterns

*   **Framework:** FastMCP for all MCP server implementations.
*   **Language:** Python 3.10+.
*   **Modularity:** Separate `src/xxx-mcp/` directories for each service, promoting separation of concerns.
*   **Configuration:** Centralized `.env` file at the project root, loaded by each server using `python-dotenv` with explicit path resolution.
*   **API Interaction:**
    *   Direct use of specialized libraries (`plexapi`, `qbittorrent-api`) where available and suitable.
    *   Custom `httpx.AsyncClient` wrappers for other REST APIs (Overseerr, SABnzbd, Tautulli).
*   **Asynchronous Operations:** Tools involving I/O (API calls) are `async` and leverage `httpx.AsyncClient`. Synchronous libraries (like `qbittorrent-api`) are called within a `ThreadPoolExecutor` using `asyncio.get_event_loop().run_in_executor`.
*   **State Management:** API client instances are managed as state on the `FastMCP` app object, initialized via the lifespan context.
*   **Python Import Handling (for `fastmcp run path/to/script.py` execution):**
    *   For servers (`server.py`) that import a local sibling module (`client.py`), the `server.py` script explicitly adds its own directory to `sys.path` at startup: `SCRIPT_DIR = Path(__file__).resolve().parent; sys.path.insert(0, str(SCRIPT_DIR))`, followed by a direct import: `from client import ...`.
    *   `__init__.py` files are present in each server module directory.
*   **Transport Protocol:** Primarily STDIO for MCP client interaction, as configured via `cline_mcp_settings.json` using `fastmcp run ...` commands.
*   **Error Handling:** Tools generally return descriptive error strings. API clients attempt to parse service-specific error messages. Logging is used for server-side diagnostics.

## Patterns

*   **API Abstraction:** The MCP server acts as an abstraction layer over the `python-plexapi`

# System Patterns: Unifi Site Manager MCP Server

## How The System Is Built
- **Core Framework**: The server is a Python application built using the `FastMCP` library, which simplifies the creation of Model Context Protocol servers.
- **Communication Transport**: Server-Sent Events (SSE) is used as the transport mechanism, allowing for real-time, unidirectional communication from the server to MCP clients. This was a user-specified requirement.
- **API Interaction**: The server interacts with the Unifi Site Manager API (hosted at `https://api.ui.com`) using the `httpx` library for asynchronous HTTP requests.
- **Configuration Management**: Critical configuration, such as the Unifi API key and base URL, is managed through environment variables, loaded using the `python-dotenv` library from a `.env` file.
- **Modularity**: API interaction logic is largely encapsulated within a helper async function `_make_api_request`. This function centralizes:
    - Authentication (adding `X-API-Key` header).
    - HTTP method handling (GET, POST).
    - Parameter and JSON body encoding.
    - Response parsing.
    - Error handling (HTTP status codes and API-specific error structures).
    - Rate limit handling (respecting `Retry-After` headers).
    - Automatic pagination (iteratively calling API endpoints using `nextToken` until all data is fetched).
- **Tool Definition**: Each exposed functionality is defined as an asynchronous Python function decorated with `@mcp.tool()`. These tools directly call `_make_api_request` to perform their operations.
- **Logging**: Standard Python `logging` is used to provide information about server operations, API requests, and potential errors.

## Key Technical Decisions
- **SSE Transport**: Chosen based on explicit user request for remote accessibility and multi-client support.
- **Automatic Full Pagination**: For API endpoints returning lists, the server will automatically fetch all available pages and return a complete dataset. This simplifies client-side logic but can be resource-intensive for very large datasets.
- **Inclusion of Early Access (EA) Endpoints**: Tools for ISP Metrics and SD-WAN features utilize Unifi API endpoints currently marked as `/ea/`. This was approved by the user with the understanding of potential instability and stricter rate limits (100 requests/minute vs. 10,000/minute for v1 endpoints).
- **Exclusion of `get_device_details` Tool**: A proposed tool to get details for a single device was removed from the final plan because the Unifi Site Manager API does not offer a direct endpoint for this. Implementing it would have required fetching all devices (potentially across multiple hosts) and filtering, which was deemed inefficient.
- **Removal of `ctx` (ToolContext/McpContext) Parameter**: Initial attempts to include a context parameter in tool definitions led to import errors. Since the context parameter was not being utilized within the tool implementations (e.g., for logging or accessing resources via context), it was removed for simplicity, aligning with basic FastMCP usage patterns.

## Architecture Patterns
- **MCP Server**: Follows the Model Context Protocol server architecture, exposing tools for LLM/client consumption.
- **Service Layer Abstraction**: The `_make_api_request` function acts as a service layer, abstracting the direct complexities of `httpx` calls and Unifi API-specific behaviors (like pagination and error formats) from the tool definitions.
- **Asynchronous Operations**: Leverages `async/await` for all I/O-bound operations (API calls), ensuring the server remains responsive.

# System Patterns: yarr-mcp

## How the System is Built

`yarr-mcp` is designed as a collection of independent yet consistently structured Model Context Protocol (MCP) servers.

*   **Project Structure**:
    *   The root directory `yarr-mcp/` contains project-level files like `pyproject.toml`, the main `README.md`, and the central `.env` file.
    *   Individual MCP servers reside in subdirectories under `src/`, following the pattern `src/<service-name>-mcp/` (e.g., `src/plex-mcp/`).
    *   Each server directory contains:
        *   The main server script (e.g., `plex-mcp-server.py`).
        *   A specific `README.md` for that server.
        *   A `.env.example` file.
        *   A tool testing results file (e.g., `plex-mcp-tools-test-results.md`).
*   **Server Implementation**:
    *   Built using Python and the FastMCP library.
    *   Adherence to a common server template (`create-mcp-server_v2.md`) is enforced to ensure consistency across servers.
*   **Configuration Management**:
    *   Environment variables are the primary method of configuration.
    *   A single `.env` file at the project root (`yarr-mcp/.env`) is loaded by all servers. Path to this file is `../../.env` relative to the server script.
    *   Standardized naming for environment variables:
        *   Service connection: `<SERVICE_NAME_UPPER>_API_URL`, `<SERVICE_NAME_UPPER>_API_KEY` (or `_USER`/`_PASS`).
        *   MCP server settings: `<SERVICE_NAME_UPPER>_MCP_TRANSPORT`, `_MCP_HOST`, `_MCP_PORT`.
        *   Logging: `<SERVICE_NAME_UPPER>_MCP_LOG_LEVEL`, `_MCP_LOG_FILE`.
*   **Transport**:
    *   SSE (Server-Sent Events) is the default transport mechanism, facilitating remote and concurrent access. Endpoint typically at `/mcp`.
    *   STDIO (Standard Input/Output) is a configurable alternative for local client usage.
*   **Logging**:
    *   Standardized logging setup in each server:
        *   Outputs to both console (stdout) and a rotating file.
        *   Log file typically named `<service-name>-mcp.log` within the server's directory or as configured by `_MCP_LOG_FILE`.
        *   Log level is configurable via `_MCP_LOG_LEVEL` (defaults to `INFO`).
*   **Execution**:
    *   Each server is an independent Python script executed using `python src/<service-name>-mcp/<service-name>-mcp-server.py`.
    *   A standard `if __name__ == "__main__":` block is used to initiate the server.
    *   Servers perform critical checks for essential environment variables (API URL, credentials) at startup and exit if missing.
*   **Dependency Management**:
    *   Project dependencies are managed centrally in `pyproject.toml` at the project root.
    *   `uv` is the recommended tool for installing dependencies (`uv pip install -e .`).
    *   Individual `requirements.txt` files within server subdirectories are deprecated and have been removed.

## Key Technical Decisions

*   **FastMCP as Core Framework**: Leveraging FastMCP for its capabilities in rapidly developing MCP servers.
*   **Standardization via Template**: Using `create-mcp-server_v2.md` to enforce a consistent structure, configuration pattern, and feature set (logging, env vars) across all servers.
*   **SSE as Default Transport**: Prioritizing SSE for broader client compatibility and remote access, while retaining STDIO as an option.
*   **Centralized Configuration (`.env` at root)**: Simplifying management of secrets and settings for multiple servers.
*   **Centralized Dependency Management**: Using `pyproject.toml` for a single source of truth for dependencies.
*   **Robust Logging**: Implementing a consistent and configurable logging pattern for easier debugging and monitoring.

## Architecture Patterns

*   **Microservice-like Architecture**: Each media application (Plex, Prowlarr, etc.) is fronted by its own dedicated MCP server. This promotes modularity and isolates concerns.
*   **Configuration-Driven Behavior**: Server behavior (transport, ports, logging) is largely controlled by environment variables.
*   **Template-Based Development**: New servers are created or existing ones are refactored based on a defined template (`create-mcp-server_v2.md`).

# System Patterns: yarr-mcp Dockerized Application

This document outlines the key system patterns and architectural decisions employed in the Dockerized `yarr-mcp` application.

## Core Architecture

1.  **Containerized Application:**
    *   The entire `yarr-mcp` suite of MCP services is packaged into a single Docker image.
    *   This promotes consistency and simplifies deployment across different environments.

2.  **Service-Oriented (within the container):**
    *   While running in one container, the system logically operates as multiple distinct MCP services.
    *   Each service (`SERVICENAME-mcp`) is a separate Python application responsible for interfacing with a specific external tool (Plex, Gotify, etc.).

3.  **Centralized Entrypoint (`entrypoint.sh`):**
    *   A bash script (`entrypoint.sh`) acts as the main process manager within the container.
    *   It dynamically discovers and launches enabled MCP services based on environment variables.
    *   This pattern avoids needing separate Docker services for each MCP application in `docker-compose.yml` at this stage, simplifying the Docker Compose setup to a single application service.

4.  **Orchestration with Docker Compose:**
    *   `docker-compose.yml` is used to define and manage the `yarr-mcp` application service.
    *   It handles image building, container lifecycle (start, stop, restart), environment variable injection from `.env` files, and port mapping.

## Key Technical Decisions & Patterns

1.  **Unified Docker Image:**
    *   **Decision:** Package all MCP services into one Docker image rather than creating separate images for each.
    *   **Rationale:** Simplifies the build process and initial deployment complexity for the user. Reduces the number of images to manage.

2.  **Centralized Python Dependency Management:**
    *   **Decision:** Use a single, root-level `pyproject.toml` file with `uv` to manage all Python dependencies for the `yarr-mcp` project and all its sub-services.
    *   **Rationale:** Ensures dependency consistency across all services, avoids potential conflicts that could arise from per-service `requirements.txt` files, and simplifies the dependency installation step in the `Dockerfile`.

3.  **Environment Variable Driven Configuration:**
    *   **Pattern:** All runtime configurations (service URLs, API keys, ports, enable/disable flags, log levels) are managed via environment variables.
    *   **Implementation:** Docker Compose injects these variables from an `.env` file into the container environment.
    *   **Rationale:** Standard, flexible, and secure way to configure applications in Dockerized environments, allowing easy customization without modifying code or the Docker image.

4.  **Dynamic Service Activation via Entrypoint Script:**
    *   **Decision:** Use an `entrypoint.sh` script to determine which MCP services to start at runtime.
    *   **Pattern:** Services are enabled by default. They are disabled if their corresponding `SERVICENAME_MCP_DISABLE` environment variable is set to `true`.
    *   **Rationale:** Provides fine-grained control over which services run without altering the `Dockerfile` or `docker-compose.yml` for typical use cases. Allows users to run only the services they need, saving resources.

5.  **`uv` for Python Packaging and Resolution:**
    *   **Decision:** Utilize `uv` for installing Python dependencies.
    *   **Rationale:** `uv` is a fast and modern Python package installer and resolver. Its speed can significantly reduce Docker image build times compared to traditional `pip` with complex dependency trees.
    *   **Implementation Detail:** The `uv` binary is copied from the official `ghcr.io/astral-sh/uv:latest` image into `/usr/local/bin/uv` in the `Dockerfile` for reliable access.

6.  **Standardized MCP Service Script Naming:**
    *   **Pattern:** Each MCP service has a main server script named `SERVICENAME-mcp-server.py` (e.g., `plex-mcp-server.py`).
    *   **Rationale:** Allows the `entrypoint.sh` script to programmatically identify and launch these server scripts.

7.  **Standardized MCP Endpoint Path:**
    *   **Pattern:** MCP services are expected to serve their main functionality (especially for SSE) under the `/mcp` path (e.g., `http://localhost:PORT/mcp`).
    *   **Rationale:** Provides a consistent access pattern for clients connecting to any of the MCP services.

## Communication and Data Flow

*   **User to Docker Compose:** User interacts with `docker compose` CLI to manage the application.
*   **Docker Compose to Container:** Docker Compose starts the container, passing environment variables from `.env`.
*   **Container Entrypoint:** `entrypoint.sh` reads environment variables and starts individual `SERVICENAME-mcp-server.py` processes.
*   **MCP Service (Internal):** Each Python MCP server script listens on its configured `SERVICENAME_MCP_HOST` and `SERVICENAME_MCP_PORT` inside the container.
*   **MCP Service to External Application:** Each MCP service uses its specific SDK/API client (configured with URLs/tokens from env vars) to communicate with the target application (Plex, Gotify, etc.).
*   **Client to MCP Service:** External MCP clients connect to the host machine on the ports mapped by Docker Compose, which forward to the respective `SERVICENAME_MCP_PORT` inside the container, typically at the `/mcp` path.