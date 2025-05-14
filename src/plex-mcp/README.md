# Plex MCP Server

This server allows interaction with a Plex Media Server instance via the Model Context Protocol (MCP), enabling MCP-compatible clients to query and control Plex.

## Design Rationale
This server provides a comprehensive set of tools for common Plex interactions, including library browsing, searching, playback control, and server information retrieval, designed to be used by an LLM or other automated systems.

## Implemented Tools
*   `get_libraries()`: Retrieves a list of all library section names.
*   `search_library(query, library_name=None)`: Searches for media items, optionally within a specific library.
*   `play_media(item_title, client_name)`: Plays a media item on a specified Plex client.
*   `get_server_info()`: Retrieves basic information and status about the Plex server.
*   `list_clients()`: Lists available Plex clients connected to the server.
*   `get_active_sessions()`: Retrieves information about current playback sessions.
*   `control_playback(client_name, action, offset_ms=None)`: Controls playback on a client (play, pause, stop, seek).
*   `get_recently_added(library_name, limit=10)`: Retrieves recently added items from a specific library.
*   `get_library_size(library_name)`: Retrieves the total number of items in a specific library.
*   `list_all_library_titles(library_name)`: Retrieves all item titles from a specific library.
*   `get_library_episodes_count(library_name)`: Retrieves the total episode count for a TV library.
*   `get_music_library_stats(library_name)`: Retrieves artist, album, and track counts for a music library.
*   `media_stats()`: Retrieves comprehensive statistics about all media libraries.


## Quick Start

### Installation

1.  **Prerequisites:**
    *   A running Plex Media Server instance.
    *   Plex URL and a valid Plex Token.
    *   Python 3.10+.
    *   `uv` (recommended for package management).

2.  **Clone the repository (if you haven't already):**
    ```bash
    git clone https://github.com/jmagar/yarr-mcp.git
    cd yarr-mcp
    ```

3.  **Activate virtual environment and install dependencies:**
    Ensure you have activated the main project's virtual environment (e.g., `source .venv/bin/activate` from the `yarr-mcp` root). Dependencies (`fastmcp`, `python-dotenv`, `plexapi`) are managed by the main `yarr-mcp` project's `pyproject.toml`. If not already installed:
    ```bash
    # From the yarr-mcp project root
    # uv pip install -r requirements.txt (or equivalent for pyproject.toml)
    ```

### Configuration

Set the following environment variables. You can place them in a `.env` file in the `yarr-mcp` project root or ensure they are set in the environment where the MCP server will run:

*   `PLEX_URL`: The full URL of your Plex Media Server (e.g., `http://localhost:32400`).
*   `PLEX_TOKEN`: Your Plex access token.
*   `PLEX_MCP_TRANSPORT`: (Optional) `sse` (default) or `stdio`.
*   `PLEX_MCP_HOST`: (Optional) Host for SSE transport, defaults to `0.0.0.0`.
*   `PLEX_MCP_PORT`: (Optional) Port for SSE transport, defaults to `8000` in the script (or as set by `PLEX_MCP_PORT` env var, e.g. `6974` in `.env.example`).
*   `PLEX_LOG_LEVEL`: (Optional) Logging level, e.g., `INFO`, `DEBUG`. Defaults to `INFO`.

An example environment file (`.env.example`) is provided in the `src/plex-mcp/` directory.

### Running the Server

Make sure your virtual environment is activated. The server script is `src/plex-mcp/plex-mcp-server.py`.

Run the server directly using Python:
```bash
python src/plex-mcp/plex-mcp-server.py
```
This will typically start the server using SSE by default, as configured by `PLEX_MCP_TRANSPORT` or its internal default in the script.

## Claude Desktop Configuration

To use this server with Claude Desktop (primarily for STDIO transport):

**MacOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`  
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`  
**Linux**: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "plex-mcp": {
      "command": "python",
      "args": [
        "/absolute/path/to/yarr-mcp/src/plex-mcp/plex-mcp-server.py"
      ],
      "env": {
        "PLEX_URL": "YOUR_PLEX_URL_HERE",
        "PLEX_TOKEN": "YOUR_PLEX_TOKEN_HERE",
        "PLEX_MCP_TRANSPORT": "stdio" // Recommended for Claude Desktop
      }
    }
  }
}
```
**Important:** Replace `/absolute/path/to/yarr-mcp/` with the actual absolute path to the cloned `yarr-mcp` directory on your system, and fill in your Plex URL and Token.

**Note on SSE Transport with Claude Desktop:**
Claude Desktop does not natively manage SSE server connections through its config. For SSE:
1.  Ensure the server is configured for SSE (e.g., `PLEX_MCP_TRANSPORT=sse` in your `.env` or by script default).
2.  Run `plex-mcp-server.py` manually.
3.  Connect your SSE client (e.g., Cline) to `http://<PLEX_MCP_HOST>:<PLEX_MCP_PORT>/mcp`.

After configuration:
1.  (Claude Desktop with STDIO) Restart Claude Desktop.
2.  Look for the MCP icon (🔌) in your client.

## Usage Examples
(Conceptual examples, actual invocation depends on the MCP client)

*   List libraries: `get_libraries`
*   Search for "Avengers" in the "Movies" library: `search_library query="Avengers" library_name="Movies"`
*   Play "The Matrix" on client "Living Room TV": `play_media item_title="The Matrix" client_name="Living Room TV"`

## Troubleshooting

### Common Issues

1.  **Server not appearing in Claude Desktop (for STDIO)**
    *   Verify the absolute path in `claude_desktop_config.json` is correct.
    *   Ensure Python is accessible.
    *   Confirm `PLEX_MCP_TRANSPORT` is `stdio` in the Claude Desktop config.
    *   Check Claude Desktop logs.

2.  **Connection issues (for SSE)**
    *   Ensure `plex-mcp-server.py` is running.
    *   Verify `PLEX_MCP_TRANSPORT` is `sse`.
    *   Check server logs for host/port details.
    *   Check firewalls.

3.  **Authentication errors (to Plex API)**
    *   Verify `PLEX_URL` and `PLEX_TOKEN` in the server's environment.
    *   Check server logs for "Unauthorized" or "Failed to connect" messages.

4.  **Tool execution failures**
    *   Check server logs (`plex_mcp.log` and console) for errors.

## FastMCP Implementation Notes
*   The server uses a `lifespan` context manager for Plex connection management.
*   Logging includes console and a rotating file (`plex_mcp.log`).
*   Transport defaults to SSE, configurable via `PLEX_MCP_TRANSPORT`.
*   Relies on the `plexapi` library for Plex communication. 