# Active Context

## Current Task & State (as of last memory cycle)

The primary ongoing task was a systematic review and standardization of all MCP servers within the `src/` directory. The goal was to ensure alignment with the `create-mcp-server_v2.md` template and best practices. This involved:
*   Standardizing environment variable usage (for API credentials, MCP transport, host, port, logging).
*   Ensuring servers default to SSE transport.
*   Implementing robust logging (console and file).
*   Verifying correct `if __name__ == "__main__":` blocks and startup procedures, including critical env var checks.
*   Consolidating `.env` file loading to the project root (`../../.env`).
*   Removing redundant `requirements.txt` and `__init__.py` files from individual server directories.
*   Testing the tools for each server and creating/updating `<service>-mcp-tools-test-results.md` files.

The project was also renamed from `mcplex` to `yarr-mcp`.

The main project `README.md` was just updated to reflect the new project name, list all currently developed/reviewed servers, and provide revised setup, run, and client connection instructions (including SSE examples).

The current active step *is* this memory bank update.

## Recent Changes (Summary)

*   **Servers Processed**:
    *   `qbittorrent-mcp`: Reviewed, updated.
    *   `plex-mcp`: Reviewed, updated, `README.md` created.
    *   `overseerr-mcp`: Reviewed, updated.
    *   `tautulli-mcp`: Reviewed, updated.
    *   `sabnzbd-mcp`: Reviewed, updated.
    *   `prowlarr-mcp`: Reviewed, updated (was mostly compliant), tested.
    *   `portainer-mcp`: Reviewed, significant fixes to tool parameter handling, updated, tested.
    *   `unifi-mcp`: Reviewed, updated, tools tested.
    *   `unraid-mcp`: Reviewed, updated, tools tested (noted some API/server-side limitations).
*   **Key Technical Fixes**:
    *   Corrected `.env` loading paths in server scripts to point to the project root.
    *   Resolved `FastMCP` API usage issues (e.g., `mcp.run()` vs `mcp.run_server()`, parameter passing in `_portainer_request`).
    *   Addressed tool signature mismatches causing failures (e.g., Portainer tools).
*   **Project-Level Changes**:
    *   Renamed project to `yarr-mcp`.
    *   Updated main `README.md` extensively.
    *   Confirmed dependency management via root `pyproject.toml`.
*   **Documentation**:
    *   Created/Updated test result markdown files for reviewed servers.
    *   Individual server `README.md` files were updated during their review process.

## Next Steps

1.  **Complete this Memory Bank Update**: Ensure all five `cline_docs/` files are accurate and comprehensive.
2.  **Process `gotify-mcp`**: This server is listed in the main `README.md` but has not yet been explicitly reviewed, updated against the template, or tested as part of the recent sweep. This is the most likely next development task.
3.  **Verify Tool Testing for All Servers**: While many servers were tested, ensure comprehensive tool testing reports exist and are accurate for *all* servers (including those updated earlier in the process like qbittorrent, plex, etc.).
4.  **Review Individual READMEs for Root .env Path**: Confirm that individual server `README.md` files accurately reflect that the `.env` file is loaded from the project root, not their local directory.

# Active Context: yarr-mcp Project

## What are we working on now?

As of the last interaction, the primary task of Dockerizing the `yarr-mcp` application suite and updating associated documentation (`README.md`, `.env.example`) has been completed. The immediate active task is the **creation and population of all five Memory Bank files** (`productContext.md`, `activeContext.md`, `systemPatterns.md`, `techContext.md`, `progress.md`) to ensure full context for future work or after a memory reset.

We are currently in the process of generating these Memory Bank files.

## Recent Changes (Leading to Current State)

*   **Dockerization Implementation (Climb CmpS - Completed):**
    *   A `Dockerfile` was created to build a single Docker image containing all MCP services. This involved:
        *   Using `python:3.13-slim` as a base.
        *   Installing `bash` and `uv` (Python package installer).
        *   Copying `pyproject.toml` and `uv.lock` to install all project dependencies using `uv pip install . --system --no-cache-dir`.
        *   Correctly installing `uv` by copying the binary from `ghcr.io/astral-sh/uv:latest` after several attempts with `curl` and `PATH` adjustments failed.
        *   Copying all service application code from `src/SERVICENAME-mcp/` to `/app/services/SERVICENAME-mcp/`.
        *   Setting up an `entrypoint.sh` script.
    *   An `entrypoint.sh` script was developed to:
        *   Iterate through known MCP services.
        *   Check `SERVICENAME_MCP_DISABLE` environment variables (defaulting to enabled if not `true`).
        *   Start the corresponding `SERVICENAME-mcp-server.py` for each enabled service in the background.
        *   Warn if `SERVICENAME_MCP_HOST` or `SERVICENAME_MCP_PORT` are not set for an enabled service.
    *   A `docker-compose.yml` file was created to:
        *   Define the main application service (`yarr-mcp-app`).
        *   Specify the build context (`.`).
        *   Load environment variables from an `.env` file.
        *   Set a `restart: unless-stopped` policy.
        *   Dynamically map ports for each service based on `SERVICENAME_MCP_PORT` variables from the `.env` file.
    *   An `.env.example` file was created and iteratively refined to provide a correct template for user configuration, reflecting the variables used in `.env` (e.g., `PLEX_URL`, `PLEX_TOKEN`, `SERVICENAME_MCP_PORT`, `SERVICENAME_MCP_DISABLE`, `SERVICENAME_LOG_LEVEL`).
*   **Testing and Debugging:**
    *   The Docker build process was debugged, particularly the `uv` installation method.
    *   Basic functionality of all MCP services running within the Docker container was tested by making sample API calls (e.g., `mcp_mcp-plex_get_server_info`, `mcp_mcp-overseerr_search_media`).
    *   An issue with a `mcp_mcp-gotify_get_messages` tool call (parameter type error) was noted during testing, though this was an issue with the AI's tool invocation, not necessarily the Gotify service itself.
*   **Documentation:**
    *   The main `README.md` was significantly updated to include comprehensive instructions for Docker-based setup and usage, integrating content previously in a separate `DOCKER.md` file (which was then deleted).
    *   `DOCKER.md` (now part of `README.md`) was updated to use `docker compose` (V2 command) and to correctly reflect example URLs (`/mcp` path) and `.env` variable structure.
*   **Version Control:**
    *   Local changes were committed with the message "added Dockerfile & docker-compose.yml".
    *   A `git pull` was performed to resolve divergent branches with the remote, merging a minor change to `README.md` from the remote.
    *   Local changes were successfully pushed to the remote repository.
*   **Memory Bank Files:**
    *   The process of creating Memory Bank files was initiated by the user's request.
    *   `productContext.md` was the first file created in this current activity.

## Next Steps

1.  Complete the creation and population of the remaining Memory Bank files:
    *   `systemPatterns.md`
    *   `techContext.md`
    *   `progress.md`
2.  Once all Memory Bank files are populated, await further instructions or new tasks from the user.
3.  If the user wishes, revisit the Gotify MCP tool call issue to ensure it's fully resolved or understood, though the service was assumed to be running correctly in Docker.

# Active Context: Unifi Site Manager MCP Server Development

## What You're Working On Now
Completed the initial implementation and debugging phase for the Unifi Site Manager MCP server. The immediate last step was to synthesize the project's progress and details into the Memory Bank files, as requested by the user.

## Recent Changes (This Session)
- **Service Research & API Analysis**: 
    - Fetched FastMCP best practices documentation.
    - Researched the Unifi Site Manager API using web search and by directly fetching its documentation pages (overview, getting started, response format, version control, and all specific endpoint details).
- **Tool Design & Iteration**:
    - Proposed an initial set of tools based on the API capabilities.
    - Collaboratively refined the toolset with the user:
        - Confirmed SSE as the transport method.
        - Agreed to make list operations fetch all pages automatically.
        - Confirmed comfort with using Early Access (EA) API endpoints.
        - Removed the `get_device_details` tool due to implementation complexity (would have required client-side filtering of `list_devices` output).
- **Implementation**: 
    - Created the main server script `src/unifi-mcp/unifi_mcp_server.py` using FastMCP and `httpx`.
    - Implemented a helper function `_make_api_request` to handle API calls, authentication, error handling, rate limiting, and automatic pagination.
    - Implemented all approved tools: `list_hosts`, `get_host_by_id`, `list_sites`, `list_devices`, `get_isp_metrics`, `query_isp_metrics`, `list_sdwan_configs`, `get_sdwan_config_by_id`, `get_sdwan_config_status`.
    - Configured SSE transport for the server.
- **Project Files Created**:
    - `src/unifi-mcp/requirements.txt`
    - `src/unifi-mcp/.env.example`
    - `src/unifi-mcp/README.md` (including setup, usage, and client configuration examples).
- **Debugging**: 
    - Resolved `ImportError` for `ToolContext` by first trying `fastmcp.server.ToolContext` and then correctly removing the `ctx` parameter from tool definitions as it was unused and causing issues with current FastMCP patterns.

## Next Steps
- **Testing**: The server needs to be thoroughly tested against a live Unifi Site Manager API to ensure all tools function as expected, especially with pagination and EA endpoints.
- **User Feedback**: Await user feedback after they test the server.
- **Memory Bank Update**: This current action – updating the memory bank files.
- **Further Development**: Based on testing and user feedback, potential future work could include refining error messages, adding more specific data transformations, or implementing new tools if the API expands or new use cases arise. 