# qBittorrent MCP Server (mc_qbittorrent)

A FastMCP server that allows interaction with a qBittorrent instance via the Model Context Protocol (MCP).
This enables MCP-compatible clients (like LLM applications) to manage torrents.

## Features

*   Connects to a specified qBittorrent WebUI using username and password.
*   Uses the `qbittorrent-api` Python library for communication.
*   Exposes MCP tools:
    *   `list_torrents(filter, category, tag)`: Lists torrents with various filters.
    *   `add_torrent_url(torrent_url, save_path, category, tags, is_paused, upload_limit_kib, download_limit_kib)`: Adds a new torrent via URL or magnet link.
    *   `pause_torrent(torrent_hash)`: Pauses a specific torrent.
    *   `resume_torrent(torrent_hash)`: Resumes a specific torrent.
    *   `get_qb_transfer_info()`: Retrieves global transfer (upload/download speed) information.
    *   `get_qb_app_preferences()`: Retrieves qBittorrent application preferences.

## Setup

1.  **Prerequisites:**
    *   A running qBittorrent instance with WebUI enabled.
    *   Username and password for the qBittorrent WebUI.
    *   Python 3.10+.
    *   `uv` (recommended for package management).

2.  **Installation (as part of the `yarr-mcp` project):**
    Dependencies (`fastmcp`, `python-dotenv`, `qbittorrent-api`) are managed by the main `yarr-mcp` project's `pyproject.toml`.
    
    Clone the repository:
    ```bash
    git clone https://github.com/jmagar/yarr-mcp.git
    cd yarr-mcp
    ```

    Ensure you have activated the main project's virtual environment (assuming it's set up, e.g., using `uv venv`):
    ```bash
    # From the yarr-mcp project root
    source .venv/bin/activate
    # If you haven't installed dependencies yet:
    # uv pip install -r requirements.txt # Or based on pyproject.toml
    ```

3.  **Configuration:**
    Set the following environment variables. You can place them in the main `.env` file in the `yarr-mcp` project root, or ensure they are set in the environment where the MCP server will run:
    *   `QBITTORRENT_URL`: The full URL of your qBittorrent WebUI (e.g., `http://localhost:8080`).
    *   `QBITTORRENT_USER`: Your qBittorrent WebUI username.
    *   `QBITTORRENT_PASS`: Your qBittorrent WebUI password.
    *   `QBITTORRENT_MCP_TRANSPORT`: (Optional) `sse` (default) or `stdio`.
    *   `QBITTORRENT_MCP_HOST`: (Optional) Host for SSE transport, defaults to `0.0.0.0`.
    *   `QBITTORRENT_MCP_PORT`: (Optional) Port for SSE transport, defaults to `8000`.
    *   `QBITTORRENT_LOG_LEVEL`: (Optional) Logging level, e.g., `INFO`, `DEBUG`. Defaults to `INFO`.

    An example environment file (`.env.example`) is provided in the `src/qbittorrent-mcp/` directory. You can copy it to `.env` in the main project root and customize it.

## Running the Server

Make sure your virtual environment is activated (`source .venv/bin/activate` from the `yarr-mcp` root).

The server script is `src/qbittorrent-mcp/qbittorrent-mcp-server.py`.

You can run the server directly using Python:
    ```bash
python src/qbittorrent-mcp/qbittorrent-mcp-server.py
    ```
This will typically start the server using SSE by default, as configured by `QBITTORRENT_MCP_TRANSPORT` or its internal default.

The server will attempt to connect to your qBittorrent instance using the configured environment variables and listen for MCP connections.

## Claude Desktop Configuration

To use this server with Claude Desktop (primarily for STDIO transport, or if you manage an SSE-to-STDIO proxy), add the following to your Claude Desktop configuration file:

**MacOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`  
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`  
**Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "qbittorrent-mcp": {
      "command": "python",
      "args": [
        "/absolute/path/to/yarr-mcp/src/qbittorrent-mcp/qbittorrent-mcp-server.py"
      ],
      "env": {
        "QBITTORRENT_URL": "http://your-qbittorrent-url:8080",
        "QBITTORRENT_USER": "your_username",
        "QBITTORRENT_PASS": "your_password",
        "QBITTORRENT_MCP_TRANSPORT": "stdio" // Recommended for Claude Desktop direct launch
        // Add other necessary env vars like QBITTORRENT_MCP_PORT if using a specific port for stdio (though usually not needed)
        // or QBITTORRENT_LOG_LEVEL
      }
    }
  }
}
```
**Important:** Replace `/absolute/path/to/yarr-mcp/` with the actual absolute path to the cloned `yarr-mcp` directory on your system.

**Note on SSE Transport with Claude Desktop:**

Currently, Claude Desktop does not natively initiate connections to SSE servers directly through its `mcpServers` configuration. The configuration above is tailored for STDIO.

*   **For Claude Desktop**: If you want Claude Desktop to manage the server process, set `QBITTORRENT_MCP_TRANSPORT="stdio"` in the `env` block of the `claude_desktop_config.json`.
*   **For SSE-compatible clients** (like Cline, other custom scripts):
    1.  Ensure the server is configured for SSE (e.g., `QBITTORRENT_MCP_TRANSPORT=sse` in your main `.env` or as the default in the script).
    2.  Run the `qbittorrent-mcp-server.py` script manually (e.g., `python src/qbittorrent-mcp/qbittorrent-mcp-server.py`).
    3.  Connect your SSE client to `http://<QBITTORRENT_MCP_HOST>:<QBITTORRENT_MCP_PORT>/mcp` (e.g., `http://localhost:8000/mcp` or `http://0.0.0.0:8000/mcp`).

After configuring Claude Desktop (for STDIO) or starting the server manually (for SSE):
1.  (Claude Desktop) Restart Claude Desktop.
2.  Look for the MCP icon (🔌) in the text input area of your client.
3.  Click to see available tools from the qBittorrent server.

## Usage Examples
Below are conceptual examples of how tools might be invoked. Actual invocation depends on the MCP client.

*   **List all torrents:** `list_torrents`
*   **List downloading torrents:** `list_torrents filter="downloading"`
*   **Add a magnet link:** `add_torrent_url torrent_url="magnet:?xt=urn:btih:..." category="movies"`
*   **Pause a torrent:** `pause_torrent torrent_hash="aabbccddeeff..."`

## Troubleshooting

### Common Issues

1.  **Server not appearing in Claude Desktop (for STDIO)**
    *   Check the `command` and `args` path in `claude_desktop_config.json` is absolute and correct.
    *   Ensure Python is installed and accessible from Claude Desktop's environment.
    *   Verify `QBITTORRENT_MCP_TRANSPORT` is set to `stdio` in the Claude Desktop config's `env` section.
    *   Check Claude Desktop logs for errors related to server startup.

2.  **Connection issues (for SSE)**
    *   Ensure the `qbittorrent-mcp-server.py` script is running.
    *   Check that `QBITTORRENT_MCP_TRANSPORT` is set to `sse` (or defaulted) when you run the script.
    *   Verify the host and port (`QBITTORRENT_MCP_HOST`, `QBITTORRENT_MCP_PORT`) the server is listening on. Check server startup logs.
    *   Check firewalls or network configurations if accessing remotely or from a different container.
    *   Verify the URL in the client configuration matches the server's listening address and path (e.g., `http://localhost:8000/mcp`).

3.  **Authentication errors (to qBittorrent API)**
    *   Verify `QBITTORRENT_URL`, `QBITTORRENT_USER`, `QBITTORRENT_PASS` are correct in the environment where the MCP server is running (e.g., your main `.env` file or system environment variables).
    *   Check server logs for "Login failed" or "API Connection failed" messages.

4.  **Tool execution failures**
    *   Check server logs (`qbittorrent_mcp.log` and console output) for detailed error messages from the MCP server or the qBittorrent API.
    *   Ensure your qBittorrent instance is running and accessible from where the MCP server is running.

## FastMCP Implementation Notes
*   The server uses a `lifespan` context manager to initialize and manage the `qbittorrent-api` client, ensuring it's logged in at startup.
*   Synchronous calls to the `qbittorrent-api` library are run in a `ThreadPoolExecutor` to avoid blocking the asyncio event loop.
*   Logging is configured for both console and a rotating file (`qbittorrent_mcp.log`).
*   The transport method (SSE or STDIO) is configurable via the `QBITTORRENT_MCP_TRANSPORT` environment variable, defaulting to SSE. 