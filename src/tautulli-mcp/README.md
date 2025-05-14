# Tautulli MCP Server

A FastMCP server that allows interaction with a Tautulli instance via the Model Context Protocol (MCP) to retrieve Plex media server statistics and activity.

## Features

*   Connects to a specified Tautulli instance using an API Key.
*   Uses a custom asynchronous HTTP client for API interactions.
*   Exposes MCP tools:
    *   `get_tautulli_activity()`: Retrieves current Plex activity.
    *   `get_tautulli_home_stats(time_range=None, stats_count=None)`: Gets homepage statistics.
    *   `get_tautulli_history(user_id=None, section_id=None, length=25)`: Retrieves watch history with optional filters.
    *   `get_tautulli_users()`: Lists users known to Tautulli.

## Setup

1.  **Prerequisites:**
    *   A running Tautulli instance.
    *   The API Key from your Tautulli instance (Settings -> Web Interface -> API -> API Key).
    *   Python 3.10+.
    *   `uv` (recommended for package management).

2.  **Installation (as part of the `yarr-mcp` project):**
    Dependencies (`fastmcp`, `python-dotenv`, `httpx`) are managed by the main `yarr-mcp` project's `pyproject.toml`.
    
    Clone the repository (if not already done):
    ```bash
    git clone https://github.com/jmagar/yarr-mcp.git
    cd yarr-mcp
    ```

    Ensure you have activated the main project's virtual environment:
    ```bash
    # From the yarr-mcp project root
    source .venv/bin/activate
    # If dependencies are not installed: uv pip install -r requirements.txt (or similar)
    ```

3.  **Configuration:**
    Set the following environment variables (e.g., in a `.env` file in the `yarr-mcp` project root):
    *   `TAUTULLI_URL`: The full base URL of your Tautulli instance (e.g., `http://localhost:8181`).
    *   `TAUTULLI_API_KEY`: Your Tautulli API Key.
    *   `TAUTULLI_MCP_TRANSPORT`: (Optional) `sse` (default) or `stdio`.
    *   `TAUTULLI_MCP_HOST`: (Optional) Host for SSE transport, defaults to `0.0.0.0`.
    *   `TAUTULLI_MCP_PORT`: (Optional) Port for SSE transport, defaults to `8002` in the script.
    *   `TAUTULLI_LOG_LEVEL`: (Optional) Logging level (e.g., `INFO`, `DEBUG`). Defaults to `INFO`.

    An example `.env.example` is in `src/tautulli-mcp/`.

## Running the Server

Make sure your virtual environment is activated. The server script is `src/tautulli-mcp/tautulli-mcp-server.py`.

Run the server directly:
    ```bash
python src/tautulli-mcp/tautulli-mcp-server.py
```
This will start the server using SSE by default (or as configured by `TAUTULLI_MCP_TRANSPORT`).

## Claude Desktop Configuration

For Claude Desktop (primarily with STDIO transport):

**MacOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`  
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`  
**Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "tautulli-mcp": {
      "command": "python",
      "args": [
        "/absolute/path/to/yarr-mcp/src/tautulli-mcp/tautulli-mcp-server.py"
      ],
      "env": {
        "TAUTULLI_URL": "YOUR_TAUTULLI_URL",
        "TAUTULLI_API_KEY": "YOUR_TAUTULLI_API_KEY",
        "TAUTULLI_MCP_TRANSPORT": "stdio" // Recommended for Claude Desktop
      }
    }
  }
}
```
**Important:** Replace paths and credentials as needed.

**Note on SSE Transport:**
Claude Desktop doesn't directly manage SSE servers. For SSE:
1.  Run `tautulli-mcp-server.py` manually (it defaults to SSE).
2.  Connect SSE clients (e.g., Cline) to `http://<TAUTULLI_MCP_HOST>:<TAUTULLI_MCP_PORT>/mcp`.

## Usage Examples
*   Get current activity: `get_tautulli_activity`
*   Get Tautulli users: `get_tautulli_users`
*   Get recent history for user 123: `get_tautulli_history user_id=123 length=10`

## Troubleshooting

### Common Issues
1.  **Server not in Claude Desktop (STDIO):** Check path, Python, `TAUTULLI_MCP_TRANSPORT="stdio"` in config, and Claude logs.
2.  **SSE Connection Issues:** Ensure server script is running, `TAUTULLI_MCP_TRANSPORT` is `sse`, check host/port/path in server logs, and firewalls.
3.  **Tautulli API Auth Errors:** Verify `TAUTULLI_URL` and `TAUTULLI_API_KEY`.
4.  **Tool Failures:** Check server logs (`tautulli_mcp.log` and console).

## FastMCP Implementation Notes
*   Uses a `lifespan` manager for the `TautulliApiClient`.
*   Logging is to console and a rotating file (`tautulli_mcp.log`).
*   Transport defaults to SSE, configurable via `TAUTULLI_MCP_TRANSPORT`.
*   A custom `client.py` handles HTTP calls to the Tautulli API. 