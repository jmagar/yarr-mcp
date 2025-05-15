# Product Context: yarr-mcp Dockerization

## Why this project exists

This project, specifically the Dockerization effort, exists to create a unified, easily deployable, and manageable environment for the `yarr-mcp` application. The `yarr-mcp` application itself consists of multiple MCP (Model Context Protocol) services designed to interact with various media and home automation applications (like Plex, Overseerr, SABnzbd, qBittorrent, Tautulli, Portainer, Prowlarr, Unifi, Unraid, and Gotify).

The primary goal of this Dockerization is to simplify the setup and running of these diverse MCP services, which would otherwise require individual Python environments and process management.

## What problems it solves

The Dockerized `yarr-mcp` solution addresses several challenges:

1.  **Simplified Deployment:** Provides a single Docker container to run all selected MCP services, abstracting away individual setup complexities.
2.  **Consistent Runtime Environment:** Ensures all MCP services run in a predictable environment (Python 3.13, specific OS base) regardless of the host system.
3.  **Centralized Dependency Management:** Uses a root `pyproject.toml` and `uv` to manage Python dependencies for all services, avoiding conflicts and simplifying updates.
4.  **Easy Configuration:** Leverages environment variables (via an `.env` file and Docker Compose) for straightforward configuration of service URLs, API keys, ports, and enable/disable states.
5.  **Orchestration:** Uses Docker Compose (`docker-compose.yml`) for easy building, starting, stopping, and managing the lifecycle of the application and its services.
6.  **Scalability (Conceptual):** While the current setup is a single container, Dockerization lays the groundwork for more scalable deployment patterns if needed in the future.
7.  **Portability:** Allows the `yarr-mcp` suite to run on any system that supports Docker.

## How it should work

The system is designed to function as follows:

1.  **Docker Image:** A single Docker image is built using the `Dockerfile`. This image:
    *   Is based on `python:3.13-slim`.
    *   Installs `bash` and `uv`.
    *   Copies the project's `pyproject.toml` and `uv.lock` to install all Python dependencies for `yarr-mcp` and its constituent MCP services.
    *   Copies all MCP service application code from the `src/SERVICENAME-mcp` directories into the image at `/app/services/SERVICENAME-mcp`.
    *   Copies an `entrypoint.sh` script and makes it executable.
    *   Sets `/app` as the working directory.

2.  **Entrypoint Script (`entrypoint.sh`):**
    *   This script is the `ENTRYPOINT` for the Docker container.
    *   It iterates through a predefined list of known MCP services.
    *   For each service, it checks an environment variable `SERVICENAME_MCP_DISABLE`.
    *   If `SERVICENAME_MCP_DISABLE` is not set to `true`, the script:
        *   Verifies that `SERVICENAME_MCP_HOST` and `SERVICENAME_MCP_PORT` are set, warning if not.
        *   Navigates to the service's directory (`/app/services/SERVICENAME-mcp`).
        *   Starts the corresponding Python server script (`python ./SERVICENAME-mcp-server.py`) in the background.
    *   The script aims to launch all enabled services concurrently.

3.  **Docker Compose (`docker-compose.yml`):**
    *   Defines a single service named `yarr-mcp-app`.
    *   Builds the Docker image from the current directory (`.`).
    *   Assigns a container name (`yarr-mcp`).
    *   Loads environment variables from an `.env` file.
    *   Sets a `restart: unless-stopped` policy.
    *   Maps ports for each MCP service from the host to the container. The port numbers are dynamically taken from environment variables like `${SERVICENAME_MCP_PORT}`. For example, `"${GOTIFY_MCP_PORT}:${GOTIFY_MCP_PORT}"`.

4.  **Configuration (`.env` file):**
    *   A user creates a `.env` file (typically by copying `.env.example`).
    *   This file contains all necessary configurations:
        *   `SERVICENAME_MCP_DISABLE` (e.g., `GOTIFY_MCP_DISABLE=false`): To enable/disable individual services.
        *   `SERVICENAME_MCP_HOST` (e.g., `GOTIFY_MCP_HOST=0.0.0.0`): Host for the MCP service inside the container.
        *   `SERVICENAME_MCP_PORT` (e.g., `GOTIFY_MCP_PORT=6972`): Port for the MCP service inside the container and for host mapping.
        *   Service-specific variables like `GOTIFY_URL`, `PLEX_TOKEN`, etc., for the MCP services to connect to their target applications.
        *   `LOG_LEVEL` for each service.

5.  **User Interaction:**
    *   The user clones the repository.
    *   Creates and configures their `.env` file.
    *   Uses `docker compose up --build -d` to build and start the application.
    *   MCP services, once started by `entrypoint.sh`, listen on their configured host/port inside the container (e.g., `0.0.0.0:6972`) at the `/mcp` path.
    *   These services are then accessible on the host machine via the mapped ports (e.g., `http://localhost:6972/mcp`).

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