# Product Context: yarr-mcp

## Why This Project Exists

The `yarr-mcp` project exists to provide a standardized and robust way to interact with a variety of media ecosystem applications (such as Plex, Overseerr, qBittorrent, Prowlarr, etc.) using the Model Context Protocol (MCP).

## What Problems It Solves

1.  **Standardized Access**: It offers a consistent interface (MCP) to diverse services, each with its own API eccentricities.
2.  **Simplified Integration**: Enables easier integration of these services with MCP-compatible clients, including Large Language Models (LLMs), AI assistants, and custom automation scripts.
3.  **Enhanced Control & Querying**: Allows for sophisticated querying and control of these media services beyond what might be easily scriptable directly against their native APIs.
4.  **Centralized Management**: Aims to provide a suite of well-maintained and consistently structured MCP servers.

## How It Should Work

`yarr-mcp` is a collection of individual Python-based MCP servers, each dedicated to a specific media application. Key operational aspects include:

*   **FastMCP Framework**: Each server is built using the FastMCP library for Python.
*   **Individual Servers**: Each target service (e.g., Plex, Prowlarr) has its own server script located in `src/<service>-mcp/`.
*   **Standardized Template**: Servers are designed to align with a common template (`create-mcp-server_v2.md`), ensuring consistency in:
    *   Environment variable naming (e.g., `<SERVICE_NAME_UPPER>_API_URL`, `_MCP_PORT`).
    *   Logging mechanisms (console and rotating file output).
    *   Startup procedures (`if __name__ == "__main__":`).
    *   Critical environment variable checks.
*   **Configuration**:
    *   Primary configuration is through environment variables.
    *   A central `.env` file located in the project root (`yarr-mcp/.env`) is used by all servers, with each server loading variables relevant to its operation and its specific MCP transport/host/port settings.
*   **Transport Methods**:
    *   **SSE (Server-Sent Events)**: The default transport mechanism for remote and concurrent client access.
    *   **STDIO (Standard Input/Output)**: Available as a configurable alternative, suitable for local client use (e.g., Claude Desktop).
*   **Tool Exposure**: Each server exposes a set of tools relevant to the capabilities of the underlying service's API, allowing clients to perform actions and retrieve data.
*   **Documentation**: Each server has its own `README.md` detailing its specific setup, tools, and default port. A main project `README.md` provides an overview and general setup instructions. 