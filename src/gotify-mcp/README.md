# Gotify MCP Server

This server provides tools to interact with a Gotify instance using the Model Context Protocol (MCP). It is built with FastMCP and allows you to send messages, manage applications, clients, and retrieve information such as health and version status.

This server implements the tool set approved during the collaborative design phase.

## Design Rationale

The tools were chosen to cover the core functionalities of Gotify, enabling users to:
- Send notifications (`create_message`).
- Retrieve and manage messages (`get_messages`, `delete_message`).
- Manage applications that send messages (`create_application`, `get_applications`, `update_application`, `delete_application`).
- Manage clients that receive messages (`create_client`, `get_clients`).
- Monitor the Gotify server (`get_health`, `get_version`).

The `create_message` tool specifically requires an `app_token` parameter, as per Gotify's API requirements for sending messages. Other management tools use a globally configured `GOTIFY_CLIENT_TOKEN`.

## Implemented Tools

- `create_message(app_token: str, message: str, title: Optional[str], priority: Optional[int], extras: Optional[Dict])`: Sends a new message.
- `get_messages(limit: Optional[int], since: Optional[int])`: Retrieves messages.
- `delete_message(message_id: int)`: Deletes a specific message.
- `delete_all_messages()`: Deletes all messages.
- `create_application(name: str, description: Optional[str], default_priority: Optional[int])`: Creates an application.
- `get_applications()`: Retrieves all applications.
- `update_application(app_id: int, name: Optional[str], description: Optional[str], default_priority: Optional[int])`: Updates an application.
- `delete_application(app_id: int)`: Deletes an application.
- `create_client(name: str)`: Creates a client.
- `get_clients()`: Retrieves all clients.
- `get_health()`: Checks Gotify server health.
- `get_version()`: Retrieves Gotify server version.

## Implemented Resources

- `gotify://application/{app_id}/messages?limit={limit}&since_id={since_id}`: Lists messages for a specific application.
- `gotify://currentuser`: Provides details about the currently authenticated user (via `GOTIFY_CLIENT_TOKEN`).

## Quick Start

### Prerequisites
- Python 3.8+
- A running Gotify server instance.

### Installation

1.  **Clone or download the server files:**
    Place `gotify_mcp_server.py`, `requirements.txt`, and `.env.example` in a directory (e.g., `gotify-mcp`).

2.  **Navigate to the server directory:**
    ```bash
    cd path/to/gotify-mcp
    ```

3.  **Create a virtual environment (recommended):**
    ```bash
    python -m venv .venv
    source .venv/bin/activate  # On Windows: .venv\Scripts\activate
    ```

4.  **Install dependencies:**
    ```bash
    pip install -r requirements.txt
    ```

5.  **Set up environment variables:**
    Copy the example environment file:
    ```bash
    cp .env.example .env
    ```
    Edit the `.env` file with your Gotify server details:
    ```ini
    GOTIFY_API_URL="YOUR_GOTIFY_SERVER_URL" # e.g., http://localhost:80 or https://gotify.example.com
    GOTIFY_CLIENT_TOKEN="YOUR_GOTIFY_ADMIN_CLIENT_TOKEN" # A client token with permissions to manage apps/messages

    # Optional: SSE server configuration
    # GOTIFY_MCP_HOST="0.0.0.0" # Host for the MCP SSE server to listen on
    # GOTIFY_MCP_PORT="8000"    # Port for the MCP SSE server

    # Optional: Logging configuration
    # LOG_LEVEL="INFO" # Can be DEBUG, INFO, WARNING, ERROR, CRITICAL
    ```
    - `GOTIFY_API_URL`: The full base URL of your Gotify server.
    - `GOTIFY_CLIENT_TOKEN`: A **client token** from your Gotify server. This token should have permissions to manage applications, clients, and messages if you intend to use all tools. For `create_message`, you will pass an `app_token` directly to the tool.

### Running the Server

Execute the Python script:
```bash
python gotify_mcp_server.py
```
By default, the server will start an SSE service on `http://0.0.0.0:8000/mcp`.
You should see log output indicating the server has started.

## Client Configuration

This server runs using SSE (Server-Sent Events). You can connect to it using any MCP client that supports SSE, such as Cline or a custom script using `fastmcp-client`.

**Example Cline Configuration (`cline_mcp_settings.json`):**

Ensure Cline is installed and configured. Add the following to your `cline_mcp_settings.json` file (usually found in `~/.cline/` or `~/.config/cline/`):

```json
{
  "mcpServers": {
    "gotify-mcp": {
      "url": "http://localhost:8000/mcp", // Adjust if your host/port differs
      "disabled": false,
      "autoApprove": [], // Optional: list tools to auto-approve
      "timeout": 30 // Optional: request timeout in seconds
    }
    // ... other servers
  }
}
```

- If your MCP server is running on a different host or port than `localhost:8000`, update the `url` field accordingly.
- After adding the configuration, (re)start Cline. The Gotify MCP tools should become available.

## Usage Examples

Assuming your MCP client (e.g., Cline) is connected to the server:

**1. Send a message:**
```
[tool call: gotify-mcp.create_message(app_token="YourAppTokenHere", title="Backup Complete", message="Server backup finished successfully.", priority=5)]
```
*Replace `"YourAppTokenHere"` with an actual application token from your Gotify server.*

**2. Get recent messages:**
```
[tool call: gotify-mcp.get_messages(limit=10)]
```

**3. Create a new application:**
```
[tool call: gotify-mcp.create_application(name="My New Alerting App", description="Sends critical alerts from my script")]
```
*(This will use the `GOTIFY_CLIENT_TOKEN` configured on the server.)*

**4. List all applications:**
```
[tool call: gotify-mcp.get_applications()]
```

**5. Get server health:**
```
[tool call: gotify-mcp.get_health()]
```

## Troubleshooting

-   **"GOTIFY_API_URL must be set" error:** Ensure `GOTIFY_API_URL` is correctly set in your `.env` file and the server script is loading it.
-   **Authentication errors from Gotify (e.g., 401 Unauthorized):**
    -   For `create_message`: Verify the `app_token` you are passing to the tool is valid and active.
    -   For other tools: Verify `GOTIFY_CLIENT_TOKEN` in your `.env` file is a valid client token with sufficient permissions on your Gotify server.
-   **Connection issues to the MCP server:**
    -   Ensure the `gotify_mcp_server.py` script is running.
    -   Check that no firewall is blocking the `GOTIFY_MCP_HOST` and `GOTIFY_MCP_PORT` (default `0.0.0.0:8000`).
    -   Verify the URL in your MCP client configuration matches the address the server is listening on.
-   **Tool execution failures:** Check the server logs (console output of `gotify_mcp_server.py`) for detailed error messages from the MCP server or the Gotify API itself.

## FastMCP Implementation Notes

-   The server uses `httpx` for asynchronous HTTP requests to the Gotify API.
-   A shared `_request` helper function handles common request logic, including authentication and error parsing.
-   Logging is configured to output to the console. File logging can be enabled in the script if needed.
-   Environment variables are loaded using `python-dotenv`.
-   The server runs using FastMCP's SSE transport. 