# Overseerr MCP Server

A FastMCP server that allows interaction with an Overseerr instance via the Model Context Protocol (MCP).
This enables MCP-compatible clients (like LLM applications) to search for media, request movies and TV shows, and view requests on Overseerr.

## Features

*   Connects to a specified Overseerr instance using an API Key.
*   Uses a custom asynchronous HTTP client for API interactions.
*   Exposes MCP tools:
    *   `search_media(query, media_type=None)`: Searches for movies or TV shows.
    *   `get_movie_details(tmdb_id)`: Retrieves details for a movie.
    *   `get_tv_show_details(tmdb_id)`: Retrieves details for a TV show.
    *   `request_movie(tmdb_id)`: Requests a movie.
    *   `request_tv_show(tmdb_id, seasons=None)`: Requests a TV show or specific seasons.
    *   `list_failed_requests(count=10, skip=0)`: Lists media requests that have a status of 'failed'.

## Setup

1.  **Prerequisites:**
    *   A running Overseerr instance.
    *   An API Key from your Overseerr instance (Settings -> General -> API Key).
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
    *   `OVERSEERR_URL`: The full base URL of your Overseerr instance (e.g., `http://localhost:5055`).
    *   `OVERSEERR_API_KEY`: Your Overseerr API Key.
    *   `OVERSEERR_MCP_TRANSPORT`: (Optional) `sse` (default) or `stdio`.
    *   `OVERSEERR_MCP_HOST`: (Optional) Host for SSE transport, defaults to `0.0.0.0`.
    *   `OVERSEERR_MCP_PORT`: (Optional) Port for SSE transport, defaults to `8001` in the script.
    *   `OVERSEERR_LOG_LEVEL`: (Optional) Logging level (e.g., `INFO`, `DEBUG`). Defaults to `INFO`.

    An example `.env.example` is in `src/overseerr-mcp/`.

## Running the Server

Make sure your virtual environment is activated. The server script is `src/overseerr-mcp/overseerr-mcp-server.py`.

Run the server directly:
    ```bash
python src/overseerr-mcp/overseerr-mcp-server.py
```
This will start the server using SSE by default (or as configured by `OVERSEERR_MCP_TRANSPORT`).

## Claude Desktop Configuration

For Claude Desktop (primarily with STDIO transport):

**MacOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`  
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`  
**Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "overseerr-mcp": {
      "command": "python",
      "args": [
        "/absolute/path/to/yarr-mcp/src/overseerr-mcp/overseerr-mcp-server.py"
      ],
      "env": {
        "OVERSEERR_URL": "YOUR_OVERSEERR_URL",
        "OVERSEERR_API_KEY": "YOUR_OVERSEERR_API_KEY",
        "OVERSEERR_MCP_TRANSPORT": "stdio" // Recommended for Claude Desktop
      }
    }
  }
}
```
**Important:** Replace paths and credentials as needed.

**Note on SSE Transport:**
Claude Desktop doesn't directly manage SSE servers. For SSE:
1.  Run `overseerr-mcp-server.py` manually (it defaults to SSE).
2.  Connect SSE clients (e.g., Cline) to `http://<OVERSEERR_MCP_HOST>:<OVERSEERR_MCP_PORT>/mcp`.

## Usage Examples
*   Search for "Inception": `search_media query="Inception" media_type="movie"`
*   Request movie with TMDB ID 123: `request_movie tmdb_id=123`
*   List failed requests: `list_failed_requests`

## Troubleshooting

### Common Issues
1.  **Server not in Claude Desktop (STDIO):** Check path, Python, `OVERSEERR_MCP_TRANSPORT="stdio"` in config, and Claude logs.
2.  **SSE Connection Issues:** Ensure server script is running, `OVERSEERR_MCP_TRANSPORT` is `sse`, check host/port in server logs, and firewalls.
3.  **Overseerr API Auth Errors:** Verify `OVERSEERR_URL` and `OVERSEERR_API_KEY`.
4.  **Tool Failures:** Check server logs (`overseerr_mcp.log` and console).

## FastMCP Implementation Notes
*   Uses a `lifespan` manager for the `OverseerrApiClient`.
*   Logging is to console and a rotating file (`overseerr_mcp.log`).
*   Transport defaults to SSE, configurable via `OVERSEERR_MCP_TRANSPORT`.
*   A custom `client.py` handles HTTP calls to the Overseerr API. 