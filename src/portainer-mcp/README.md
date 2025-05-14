# Portainer MCP Server

This server provides tools to interact with a Portainer instance for managing Docker environments (endpoints, containers, stacks). It is built using FastMCP and communicates with the Portainer API.

## Design Rationale

The tools were chosen to provide essential functionalities for managing Docker environments through Portainer, focusing on common operations like listing resources, inspecting details, managing container lifecycles, and retrieving logs and stack files. Kubernetes-related tools were excluded based on user feedback.

## Implemented Tools

1.  `list_endpoints`
    *   **Purpose**: Retrieves a list of all configured Portainer environments (endpoints) that the authenticated user has access to.
    *   **Parameters**:
        *   `search` (optional, string): Keyword to filter endpoints by name.
        *   `group_ids` (optional, list[int]): Filter by group IDs.
        *   `tag_ids` (optional, list[int]): Filter by tag IDs.
        *   `types` (optional, list[int]): Filter by endpoint types (1: Docker, 2: Agent on Docker, 3: Azure ACI, 4: Edge Agent, 5: Kubernetes Local).
    *   **Returns**: A human-readable list of endpoints, including their Name, ID, Type, Status, and URL.

2.  `get_endpoint_details`
    *   **Purpose**: Retrieves detailed information about a specific Portainer environment (endpoint).
    *   **Parameters**:
        *   `endpoint_id` (required, int): The ID of the environment to inspect.
    *   **Returns**: A human-readable summary of the endpoint.

3.  `list_docker_containers`
    *   **Purpose**: Lists containers within a specific Docker environment.
    *   **Parameters**:
        *   `endpoint_id` (required, int): The ID of the Docker environment.
        *   `all_containers` (optional, bool, default: false): If true, lists all containers. If false, only running containers.
        *   `filters` (optional, string): JSON string for Docker API filters (e.g., `{\"name\": [\"my-container\"]}`).
    *   **Returns**: A human-readable list of containers, including Name, ID, Image, State, and Ports.

4.  `inspect_docker_container`
    *   **Purpose**: Retrieves detailed information about a specific container in a Docker environment.
    *   **Parameters**:
        *   `endpoint_id` (required, int): The ID of the Docker environment.
        *   `container_id` (required, string): The ID or name of the container.
    *   **Returns**: A human-readable summary of the container's configuration.

5.  `manage_docker_container`
    *   **Purpose**: Allows starting, stopping, restarting, pausing, unpausing, killing, or removing a container in a Docker environment.
    *   **Parameters**:
        *   `endpoint_id` (required, int): The ID of the Docker environment.
        *   `container_id` (required, string): The ID or name of the container.
        *   `action` (required, string): Action to perform. Valid values: "start", "stop", "restart", "pause", "unpause", "kill", "remove".
    *   **Returns**: A success or failure message.

6.  `get_docker_container_logs`
    *   **Purpose**: Fetches logs from a specific container in a Docker environment.
    *   **Parameters**:
        *   `endpoint_id` (required, int): The ID of the Docker environment.
        *   `container_id` (required, string): The ID or name of the container.
        *   `timestamps` (optional, bool, default: false): Show timestamps in logs.
        *   `tail` (optional, string, default: "100"): Number of lines to show from the end of the logs, or "all".
        *   `since` (optional, string): Show logs since a UNIX timestamp or relative time (e.g., "10m").
    *   **Returns**: The container logs as a string.

7.  `list_stacks`
    *   **Purpose**: Lists all stacks (Swarm or Compose) that the user has access to.
    *   **Parameters**:
        *   `filters` (optional, string): JSON encoded map for filtering (e.g., `{\"SwarmID\": \"abc...\", \"EndpointID\": 1}`).
    *   **Returns**: A human-readable list of stacks, including Name, ID, Type, Endpoint ID, and Status.

8.  `inspect_stack`
    *   **Purpose**: Retrieves detailed information about a specific stack.
    *   **Parameters**:
        *   `stack_id` (required, int): The ID of the stack.
    *   **Returns**: A human-readable summary of the stack.

9.  `get_stack_file`
    *   **Purpose**: Retrieves the compose file content for a specific stack.
    *   **Parameters**:
        *   `stack_id` (required, int): The ID of the stack.
    *   **Returns**: The stack file content as a string.

## Quick Start

### Prerequisites
- Python 3.8+ (Python 3.10+ recommended for the `yarr-mcp` project)
- An operational Portainer instance (API v2 compatible).
- `uv` (recommended for package management within the `yarr-mcp` project).

### Installation

1.  **Clone the `yarr-mcp` repository (if you haven't already):**
    ```bash
    git clone https://github.com/jmagar/yarr-mcp.git
    cd yarr-mcp
    ```

2.  **Install dependencies:**
    Dependencies (`fastmcp`, `httpx`, `python-dotenv`, `pydantic`) are managed by the main `yarr-mcp` project's `pyproject.toml`.
    Ensure you have activated the main project's virtual environment (e.g., using `uv venv`):
    ```bash
    # From the yarr-mcp project root
    source .venv/bin/activate 
    # or on Windows (Git Bash/WSL)
    # source .venv/Scripts/activate
    ```
    *Note: The local `src/portainer-mcp/requirements.txt` file is redundant if using the main project setup and can be ignored or removed.*

3.  **Set up environment variables**:
    Create or update a `.env` file in the `yarr-mcp` project root, or specifically in the `src/portainer-mcp/` directory (server-specific `.env` will override project root settings).
    Refer to `src/portainer-mcp/.env.example` for all available options. Key variables:

    ```env
    PORTAINER_URL=https://your-portainer-instance.com
    PORTAINER_API_KEY=your_portainer_api_key_here
    
    PORTAINER_MCP_TRANSPORT=sse
    PORTAINER_MCP_HOST=0.0.0.0
    PORTAINER_MCP_PORT=6971 # Default port for Portainer MCP
    PORTAINER_MCP_LOG_LEVEL=INFO
    PORTAINER_MCP_LOG_FILE=portainer_mcp.log 
    ```
    Replace placeholders with your actual Portainer URL and API Key.

### Running the Server

Ensure your project's virtual environment is activated.

From the `yarr-mcp` project root, run:
```bash
python src/portainer-mcp/portainer-mcp-server.py
```

The server will start, by default using SSE transport on the host and port specified by `PORTAINER_MCP_HOST` and `PORTAINER_MCP_PORT` (defaults to `0.0.0.0:6971`). You should see log messages indicating the server has started and the SSE endpoint, e.g.:
`INFO - PortainerMCPServer - Portainer MCP Server SSE endpoint will be available at http://0.0.0.0:6971/mcp`

### Client Configuration

#### Claude Desktop Configuration (for STDIO Transport)

If you intend to run the server via Claude Desktop (which typically uses STDIO):

1.  Set `PORTAINER_MCP_TRANSPORT=stdio` in your `.env` file.
2.  Add the following to your Claude Desktop configuration file:

    **MacOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`  
    **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`  
    **Linux**: `~/.config/Claude/claude_desktop_config.json`

    ```json
    {
      "mcpServers": {
        "portainer-mcp": {
          "command": "python",
          "args": [
            "/absolute/path/to/yarr-mcp/src/portainer-mcp/portainer-mcp-server.py"
          ],
          "env": {
            // Ensure PORTAINER_URL and PORTAINER_API_KEY are available
            // either in the project's .env, server's .env, or system environment.
            // You can also explicitly set them here if needed, for example:
            // "PORTAINER_URL": "https://your-portainer-instance.com",
            // "PORTAINER_API_KEY": "your_portainer_api_key_here",
            "PORTAINER_MCP_TRANSPORT": "stdio" // Ensure this matches
          }
        }
      }
    }
    ```
    Replace `/absolute/path/to/` with the actual absolute path to the `yarr-mcp` project on your system.

#### SSE Transport (for Cline or other compatible clients)

If the server is running with SSE transport (the default, `PORTAINER_MCP_TRANSPORT=sse`):

1.  Ensure the server is running independently (e.g., by executing `python src/portainer-mcp/portainer-mcp-server.py` in a terminal).
2.  Configure your SSE-compatible client (like Cline) to connect to the SSE endpoint.

    **Cline Configuration Example (`cline_mcp_settings.json`):**
    ```json
    {
      "mcpServers": {
        "portainer-mcp-sse": {
          "url": "http://localhost:6971/mcp", // Or your_host:your_port/mcp
          "disabled": false,
          "autoApprove": [], // Optional: list tools to auto-approve
          "timeout": 60 // Optional: connection timeout in seconds
        }
      }
    }
    ```

After configuring your client:
1.  For STDIO via Claude Desktop: Restart Claude Desktop. The Portainer MCP tools should appear.
2.  For SSE: Ensure your MCP server Python script is running. Connect your client.
3.  Look for the MCP icon (🔌) in the text input area of your client.
4.  Click to see available tools from the Portainer MCP server.

## Usage Examples

*(Assuming the server is running and connected to your MCP client)*

**Example 1: List all Docker endpoints**
```
@PortainerMCP list_endpoints
```

**Example 2: Get details for endpoint with ID 1**
```
@PortainerMCP get_endpoint_details endpoint_id=1
```

**Example 3: List running containers in endpoint ID 2**
```
@PortainerMCP list_docker_containers endpoint_id=2 all_containers=false
```

**Example 4: Stop container 'my-web-app' in endpoint ID 1**
```
@PortainerMCP manage_docker_container endpoint_id=1 container_id="my-web-app" action="stop"
```

**Example 5: Get the last 50 lines of logs for container 'nginx-proxy' in endpoint ID 1**
```
@PortainerMCP get_docker_container_logs endpoint_id=1 container_id="nginx-proxy" tail="50"
```

## Troubleshooting

1.  **Server not starting / Connection issues (SSE)**:
    *   Ensure `portainer_mcp_server.py` is running without errors. Check its console output.
    *   Verify `PORTAINER_URL` and `PORTAINER_API_KEY` are correctly set in the `.env` file or system environment where the server is running.
    *   Check that the `PORTAINER_MCP_HOST` and `PORTAINER_MCP_PORT` (default `0.0.0.0:6971`) are not already in use by another application.
    *   Ensure your MCP client is configured with the correct URL for the SSE server (e.g., `http://localhost:6971/mcp`).
    *   Check firewalls if accessing the server remotely or from a different network namespace (e.g., Docker Desktop WSL integration).

2.  **Authentication errors (to Portainer API)**:
    *   Double-check that `PORTAINER_API_KEY` is valid and has the necessary permissions in Portainer.
    *   Verify `PORTAINER_URL` is correct.
    *   Check the Portainer MCP server logs (`src/portainer-mcp/portainer_mcp.log` as configured by `PORTAINER_MCP_LOG_FILE`, and console output) for detailed error messages from the Portainer API.

3.  **Tool execution failures**:
    *   Check server logs for detailed error messages from the MCP server or the Portainer API.
    *   Ensure the parameters you are providing to the tools are correct (e.g., valid `endpoint_id`, `container_id`).
    *   Verify the Portainer API endpoints are accessible from where the MCP server is running.

## FastMCP Implementation Notes
*   The server uses `httpx` for asynchronous HTTP requests to the Portainer API.
*   SSE transport is configured using `mcp.run(transport="sse", ...)` and is the default. STDIO is also supported via `PORTAINER_MCP_TRANSPORT`.
*   Pydantic `Field` is used for tool parameter definitions and descriptions.
*   Logging is configured to output to both console and a rotating file (path and level configurable via `PORTAINER_MCP_LOG_FILE` and `PORTAINER_MCP_LOG_LEVEL`).
*   Human-readable output is prioritized for tool results by transforming raw API responses where appropriate.
*   The server name registered with FastMCP is "Portainer MCP Server". 