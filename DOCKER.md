# Running yarr-mcp with Docker and Docker Compose

This document explains how to build and run the `yarr-mcp` application using `docker-compose`. This setup allows you to run multiple MCP (Model Context Protocol) services, each configurable via environment variables loaded from an `.env` file.

## Prerequisites

- Docker installed on your system.
- Docker Compose installed on your system.

## Setup

1.  **Clone the Repository:** If you haven't already, clone the `yarr-mcp` project.
    ```bash
    # git clone <repository_url>
    cd yarr-mcp
    ```

2.  **Create `.env` File:**
    Copy the example environment file `.env.example` to `.env`:
    ```bash
    cp .env.example .env
    ```
    Edit the `.env` file and fill in your specific configurations for each service you intend to use. This includes API keys, URLs, hostnames, and ports. Pay close attention to the `SERVICENAME_MCP_PORT` variables, as these are used by `docker-compose.yml` to map ports to your host.

## Building and Running with Docker Compose

Navigate to the root directory of the `yarr-mcp` project (where `docker-compose.yml` is located).

1.  **Build and Start Services:**
    To build the Docker image (if it doesn't exist or if `Dockerfile` has changed) and start all configured services in detached mode, run:
    ```bash
    docker-compose up --build -d
    ```
    If you only want to start the services without rebuilding (assuming the image is already built and up-to-date):
    ```bash
    docker-compose up -d
    ```

2.  **Stopping Services:**
    To stop the running services:
    ```bash
    docker-compose down
    ```

3.  **Viewing Logs:**
    To view the combined logs from all services managed by `docker-compose`:
    ```bash
    docker-compose logs -f
    ```
    To view logs for a specific service (the default service name is `yarr-mcp-app`):
    ```bash
    docker-compose logs -f yarr-mcp-app
    ```

## Configuration via `.env` File

All configuration is managed through the `.env` file in the project root.

### Disabling Services

By default, all MCP services are **enabled**. To disable a specific service, set its corresponding `SERVICENAME_MCP_DISABLE` variable to `true` in your `.env` file.

Example from `.env`:
```env
# ... other variables ...

GOTIFY_MCP_DISABLE=false      # Gotify service will be enabled
SABNZBD_MCP_DISABLE=true      # Sabnzbd service will be disabled

# ... other variables ...
```
If a `SERVICENAME_MCP_DISABLE` variable is not set or is set to `false` (or any value other than `true`, case-insensitive), the corresponding service will attempt to start.

### Service-Specific Configuration

Each service requires its own set of environment variables for its specific operation (e.g., API URLs, tokens). These **must** be defined in your `.env` file.

Additionally, each service has:
- `SERVICENAME_MCP_HOST`: The host the MCP server will listen on *inside the container* (usually `0.0.0.0`).
- `SERVICENAME_MCP_PORT`: The port the MCP server will listen on *inside the container*. This value is crucial as `docker-compose.yml` uses it to map the service's port to your host machine.

**Example snippet from `.env`:**
```env
# Gotify MCP Service Configuration
GOTIFY_MCP_DISABLE=false
GOTIFY_MCP_HOST=0.0.0.0
GOTIFY_MCP_PORT=8001
GOTIFY_MCP_URL=http://your_gotify_instance:80
GOTIFY_MCP_TOKEN=your_gotify_client_token_here

# Portainer MCP Service Configuration
PORTAINER_MCP_DISABLE=false
PORTAINER_MCP_HOST=0.0.0.0
PORTAINER_MCP_PORT=8004
PORTAINER_MCP_URL=http://your_portainer_instance:9000
PORTAINER_MCP_API_KEY=your_portainer_api_key_here
```
Refer to `.env.example` for the full list of variables for all services.

### Port Mapping

The `docker-compose.yml` file is configured to map the `SERVICENAME_MCP_PORT` for each service (as defined in your `.env` file) to the same port number on your host machine.

For example, if in your `.env` file you have:
```env
GOTIFY_MCP_PORT=8001
PORTAINER_MCP_PORT=8004
```
Then:
- The Gotify MCP service will be accessible at `http://localhost:8001` on your host.
- The Portainer MCP service will be accessible at `http://localhost:8004` on your host.

If a service is disabled via `SERVICENAME_MCP_DISABLE=true`, or if its `SERVICENAME_MCP_PORT` is not set in the `.env` file, the port mapping for that service in `docker-compose.yml` will effectively be ignored (as Docker won't be able to map to a non-existent or unassigned variable, or the internal service won't start).

Ensure the ports you define in `.env` are free on your host machine or adjust them as needed (though you would then need to adjust the left-hand side of the port mapping in `docker-compose.yml` if you wanted the host port to differ from the container port, which is not the default setup).

## Service List

The Docker setup can manage the following MCP services (found in `src/`):

- `gotify-mcp`
- `overseerr-mcp`
- `plex-mcp`
- `portainer-mcp`
- `prowlarr-mcp`
- `qbittorrent-mcp`
- `sabnzbd-mcp`
- `tautulli-mcp`
- `unifi-mcp`
- `unraid-mcp`

For each service, ensure its `_DISABLE`, `_HOST`, `_PORT`, and any other required variables (API keys, URLs, etc.) are correctly set in your `.env` file. 