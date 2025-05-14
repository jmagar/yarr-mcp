# Prowlarr MCP Server

This server provides a set of tools to interact with a Prowlarr instance using the Model Context Protocol (MCP). It is built with FastMCP and allows for managing indexers, applications, performing searches, checking system status, and more, all based on the Prowlarr API v1.

## Design Rationale

The tools were chosen to expose core Prowlarr functionalities that are useful for automation, status checking, and manual interaction via an MCP-compatible client. The selection is based on the Prowlarr v1 OpenAPI specification (`prowlarr-api.json`) and common use cases for managing indexers and their associated applications.

Key considerations:
- **Comprehensive Coverage**: Aim to cover major Prowlarr entities like Indexers, Applications, System Status, History, and Search.
- **User-Friendliness**: Provide human-readable summaries for complex outputs while retaining full JSON data for programmatic use.
- **Configuration**: Rely on environment variables for Prowlarr API URL and Key, as well as MCP server settings (transport, host, port, logging) for security and flexibility.
- **Transport**: Defaults to SSE (Server-Sent Events) for remote accessibility and concurrent connections.

## Implemented Tools

1.  `list_indexers()`: Retrieves a list of all configured indexers.
2.  `get_indexer_details(id: int)`: Retrieves detailed information for a specific indexer.
3.  `search_releases(query: str, indexerIds: Optional[List[int]], categories: Optional[List[int]], type: Optional[str], limit: Optional[int], offset: Optional[int])`: Searches for releases across indexers.
4.  `test_indexer(id: int)`: Tests a specific indexer's connectivity and search capability.
5.  `update_indexer(id: int, indexer_config: Dict[str, Any])`: Updates an existing indexer's configuration.
6.  `list_applications()`: Retrieves a list of applications synced with Prowlarr.
7.  `get_system_status()`: Retrieves Prowlarr system status and information.
8.  `get_indexer_categories()`: Retrieves the list of default indexer categories.
9.  `get_history(page: Optional[int], pageSize: Optional[int], sortKey: Optional[str], sortDirection: Optional[str], eventType: Optional[List[int]], successful: Optional[bool], downloadId: Optional[str], indexerIds: Optional[List[int]])`: Retrieves Prowlarr's history records.
10. `test_all_indexers()`: Triggers a test for all configured indexers and summarizes the results.

## Quick Start

### Prerequisites
- Python 3.8+ (Python 3.10+ recommended for `yarr-mcp` project)
- An operational Prowlarr instance (v1 API compatible).
- `uv` (recommended for package management within the `yarr-mcp` project).

### Installation

1.  **Clone the `yarr-mcp` repository (if you haven't already):**
    ```bash
    git clone https://github.com/jmagar/yarr-mcp.git
    cd yarr-mcp
    ```

2.  **Install dependencies:**
    Dependencies (`fastmcp`, `httpx`, `python-dotenv`, etc.) are managed by the main `yarr-mcp` project's `pyproject.toml`.
    Ensure you have activated the main project's virtual environment (e.g., using `uv venv`):
    ```bash
    # From the yarr-mcp project root
    source .venv/bin/activate 
    # or on Windows (Git Bash/WSL)
    # source .venv/Scripts/activate
    ```

3.  **Set up environment variables:**
    Create or update a `.env` file in the `yarr-mcp` project root, or specifically in the `src/prowlarr-mcp/` directory (server-specific will override project root).
    Refer to `src/prowlarr-mcp/.env.example` for all available options. Key variables:

    ```env
    PROWLARR_URL=http://your-prowlarr-host:9696
    PROWLARR_API_KEY=your_prowlarr_api_key_here
    
    PROWLARR_MCP_TRANSPORT=sse
    PROWLARR_MCP_HOST=0.0.0.0
    PROWLARR_MCP_PORT=6973
    PROWLARR_MCP_LOG_LEVEL=INFO
    PROWLARR_MCP_LOG_FILE=prowlarr_mcp.log 
    ```
    Replace with your actual Prowlarr instance URL and API key. The API key can be found in Prowlarr under Settings > General.

### Running the Server

Ensure your project's virtual environment is activated.

From the `yarr-mcp` project root, run:
```bash
python src/prowlarr-mcp/prowlarr-mcp-server.py
```
The server will start, by default using SSE transport on the host and port specified by `PROWLARR_MCP_HOST` and `PROWLARR_MCP_PORT` (defaults to `0.0.0.0:6973`). You should see log messages indicating the server has started and the SSE endpoint, e.g.:
`INFO - ProwlarrMCPServer - Prowlarr MCP SSE endpoint will be available at http://0.0.0.0:6973/mcp`

## Client Configuration

### Claude Desktop Configuration (for SSE)

If the server is running with SSE transport (the default), you can create an `MCP_CLIENT_CONFIG` JSON file to instruct Claude Desktop (or similar clients) how to connect. For example:

```json
{
  "clients": [
    {
      "name": "ProwlarrMCP", // Or any name you prefer
      "command": ["curl", "-N", "http://localhost:6973/mcp"],
      "type": "sse",
      "is_available": true
    }
    // Add other MCP server configurations here
  ]
}
```
Place this file (e.g., `mcp_config.json`) in a location your MCP client can access and configure the client to use it (often via an `MCP_CLIENT_CONFIG` environment variable pointing to this file's path).

### STDIO Transport Alternative

If you need to use STDIO transport, set `PROWLARR_MCP_TRANSPORT=stdio` in your `.env` file. The `MCP_CLIENT_CONFIG` command would then be:
`["python", "/path/to/yarr-mcp/src/prowlarr-mcp/prowlarr-mcp-server.py"]` (using the absolute path to the script).

## Usage Examples

*(Assuming the server is running and connected to your MCP client)*

1.  **List all configured indexers:**
    ```
    @ProwlarrMCP list_indexers
    ```
    *Expected Summary: "Found X indexers. First few: Name1 (ID: Y, Enabled: True), ..."*

2.  **Get details for a specific indexer (e.g., ID 10):**
    ```
    @ProwlarrMCP get_indexer_details id=10
    ```
    *Expected Summary: "Details for Indexer 'IndexerName' (ID: 10) retrieved."*

3.  **Search for releases related to "ubuntu" in PC/ISO categories:**
    *(Category IDs can be found using `get_indexer_categories` and examining Prowlarr UI/API)*
    ```
    @ProwlarrMCP search_releases query="ubuntu" categories=\[4020]
    ```
    *Expected Summary: "Found X releases for query 'ubuntu'. First few results: ..."*

## Troubleshooting

1.  **401 Unauthorized Errors**: 
    *   Verify `PROWLARR_URL` and `PROWLARR_API_KEY` environment variables are correctly set.
    *   Ensure your Prowlarr instance is network-accessible and API access is enabled.

2.  **Connection issues (SSE)**:
    *   Ensure the `prowlarr-mcp-server.py` script is running and listening on the correct host/port (default `0.0.0.0:6973/mcp`).
    *   Check firewalls or network configurations.
    *   Verify the URL in the client configuration matches.

3.  **Tool execution failures / Type errors with parameters**:
    *   Check server logs (`src/prowlarr-mcp/prowlarr_mcp.log` as configured by `PROWLARR_MCP_LOG_FILE`, and console output) for detailed error messages.

4.  **`test_indexer` reports 400 Bad Request**:
    *   This often means Prowlarr itself found a validation issue with that indexer's configuration. Check Prowlarr UI.

## FastMCP Implementation Notes

-   The server uses a global `logger` instance for application logging, configured for both console and rotating file output (path and level configurable via `PROWLARR_MCP_LOG_FILE` and `PROWLARR_MCP_LOG_LEVEL`).
-   An asynchronous HTTP client (`httpx`) is used for all API calls to Prowlarr.
-   Error handling in `_prowlarr_api_request` attempts to catch HTTP status errors and other request exceptions.
-   Tools are defined using the `@mcp.tool()` decorator from FastMCP.
-   The server defaults to SSE transport on the path `/mcp`. This can be changed to `stdio` via the `PROWLARR_MCP_TRANSPORT` environment variable.
-   The server name registered with FastMCP is `Prowlarr MCP Server`.
-   Refer to `prowlarr-api.json` for the OpenAPI specification of the Prowlarr API v1 this server interacts with. 