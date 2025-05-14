# SABnzbd MCP Server

A FastMCP server that allows interaction with a SABnzbd instance via the Model Context Protocol (MCP).
This enables MCP-compatible clients (like LLM applications) to view and manage SABnzbd downloads.

## Features

*   Connects to a specified SABnzbd instance using an API Key.
*   Exposes MCP tools:
    *   `get_server_stats()`: Retrieves various statistics from the SABnzbd server.
    *   `get_sab_queue(start, limit, category)`: View current download queue.
    *   `get_sab_history(start, limit)`: View download history.
    *   `pause_sab_queue()`: Pause the entire download queue.
    *   `resume_sab_queue()`: Resume the entire download queue.
    *   `toggle_pause_sabnzbd()`: Toggles the pause state of the SABnzbd download queue.
    *   `add_nzb_url(nzb_url, category)`: Add an NZB to the queue by URL.
    *   `set_sab_speedlimit(percentage)`: Set global speed limit (0-100%).

## Setup

1.  **Prerequisites:**
    *   A running SABnzbd instance.
    *   The API Key from your SABnzbd instance (Config -> General -> API Key).
    *   Python 3.10+.
    *   `uv` (recommended for package management).

2.  **Installation (as part of the `yarr-mcp` project):**
    Dependencies (`fastmcp`, `python-dotenv`, `httpx`) are managed by the main `yarr-mcp` project's `pyproject.toml`.
    
    Clone the repository (if you haven't already):
    ```bash
    git clone https://github.com/jmagar/yarr-mcp.git
    cd yarr-mcp
    ```

    Ensure you have activated the main project's virtual environment (assuming it's set up, e.g., using `uv venv`):
    ```bash
    # From the yarr-mcp project root
    source .venv/bin/activate 
    # or on Windows (Git Bash/WSL)
    # source .venv/Scripts/activate
    ```

3.  **Configuration:**
    Create or update a `.env` file in the `yarr-mcp` project root, or specifically in the `src/sabnzbd-mcp/` directory (server-specific will override project root).
    Refer to `src/sabnzbd-mcp/.env.example` for all available options. Key variables:

    *   `SABNZBD_URL`: The full base URL of your SABnzbd instance (e.g., `http://localhost:8080`).
    *   `SABNZBD_API_KEY`: Your SABnzbd API Key.
    *   `SABNZBD_MCP_TRANSPORT`: Transport method, `sse` (default) or `stdio`.
    *   `SABNZBD_MCP_HOST`: Host for SSE server (default: `0.0.0.0`).
    *   `SABNZBD_MCP_PORT`: Port for SSE server (default: `8004`).
    *   `SABNZBD_MCP_LOG_LEVEL`: Logging level (e.g., `INFO`, `DEBUG`, `WARNING`, `ERROR`, `CRITICAL`). Default: `INFO`.
    *   `SABNZBD_MCP_LOG_FILE`: Path to the log file (e.g., `sabnzbd-mcp.log` or `logs/sabnzbd-mcp.log`). Default: `sabnzbd-mcp.log` in the server's directory.

## Running the Server

Ensure your project's virtual environment is activated.

From the `yarr-mcp` project root, run:

```bash
python src/sabnzbd-mcp/sabnzbd-mcp-server.py
```

The server will start, using SSE transport by default on `0.0.0.0:8004`.
It will connect to your SABnzbd instance using the configured environment variables.

## Usage (with MCP Client)

Once the server is running, an MCP client can connect to it. 

**For Claude Desktop (or similar clients supporting `MCP_CLIENT_CONFIG`):**

If the server is running with SSE transport (the default), you can create an `MCP_CLIENT_CONFIG` JSON file to instruct your MCP client how to connect. For example:

```json
{
  "clients": [
    {
      "name": "SabnzbdMCP",
      "command": ["curl", "-N", "http://localhost:8004/mcp"],
      "type": "sse",
      "is_available": true
    }
    // Add other MCP server configurations here
  ]
}
```

Place this file (e.g., `mcp_config.json`) in a location your MCP client can access and configure the client to use it (often via an `MCP_CLIENT_CONFIG` environment variable pointing to this file's path).

If you configure `SABNZBD_MCP_TRANSPORT=stdio`, the server will use standard input/output, and the `MCP_CLIENT_CONFIG` command would be `["python", "src/sabnzbd-mcp/sabnzbd-mcp-server.py"]`.

## Implemented Tools

*   `get_server_stats()`: Retrieves various statistics from the SABnzbd server.
*   `get_sab_queue(start: Optional[int] = 0, limit: Optional[int] = 20, category: Optional[str] = None)`: View current download queue.
*   `get_sab_history(start: Optional[int] = 0, limit: Optional[int] = 20)`: View download history.
*   `pause_sab_queue()`: Pause the entire download queue.
*   `resume_sab_queue()`: Resume the entire download queue.
*   `toggle_pause_sabnzbd()`: Toggles the pause state of the SABnzbd download queue.
*   `add_nzb_url(nzb_url: str, category: Optional[str] = None)`: Add an NZB to the queue by URL.
*   `set_sab_speedlimit(percentage: int)`: Set global speed limit (0-100%).

## Error Handling & Troubleshooting

*   **Configuration Issues:** Ensure `SABNZBD_URL` and `SABNZBD_API_KEY` are correctly set in your `.env` file. The server will exit on startup if these are missing.
*   **Connection Problems:** Verify that your SABnzbd instance is running and accessible at the configured URL.
*   **Logging:** Check the server's console output and the log file (specified by `SABNZBD_MCP_LOG_FILE`, defaulting to `sabnzbd-mcp.log` in the server directory) for detailed error messages and operational information. Increase `SABNZBD_MCP_LOG_LEVEL` to `DEBUG` for more verbose output if needed.
*   **Port Conflicts:** If using SSE, ensure the `SABNZBD_MCP_PORT` (default `8004`) is not in use by another application. Change it in your `.env` file if necessary.

## FastMCP Notes

*   This server is built using the FastMCP framework.
*   It defaults to Server-Sent Events (SSE) for transport on the path `/mcp`.
*   The transport method can be changed to `stdio` by setting the `SABNZBD_MCP_TRANSPORT` environment variable to `stdio`.
*   The server name registered with FastMCP is `SabnzbdMCP`.
*   Refer to the `sabnzbd-api.yaml` (if present) for the OpenAPI specification of the tools provided by this server. 