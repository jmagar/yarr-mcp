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