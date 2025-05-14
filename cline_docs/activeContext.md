# Active Context

**Current Task:** Consolidating project knowledge and updating the Cline Memory Bank after the successful development and debugging of MCP servers for Plex, Overseerr, SABnzbd, qBittorrent, and Tautulli.

**Goal:** Ensure all Memory Bank files (`productContext.md`, `activeContext.md`, `systemPatterns.md`, `techContext.md`, `progress.md`) accurately reflect the current state, architecture, learnings, and capabilities of the `mcplex` project and its suite of servers.

**Recent Changes:**
*   Successfully implemented five distinct MCP servers for Plex, Overseerr, SABnzbd, qBittorrent, and Tautulli.
*   Developed individual API client utilities (`client.py`) for services not using a pre-existing comprehensive Python library.
*   Implemented FastMCP lifespan management in each server for API client initialization and cleanup.
*   Added numerous tools to each server for core functionalities of the respective services.
*   Created individual README files for each server module and updated the main project README.
*   Extensively debugged Python import resolution issues related to `fastmcp run path/to/script.py` execution context, settling on `sys.path` manipulation at the script start for servers importing local `client.py` modules.
*   Addressed API-specific challenges (e.g., URL encoding for Overseerr queries, understanding qBittorrent API client behavior).
*   Corrected folder naming conventions for server modules (e.g., `mc_overseerr` to `overseerr-mcp`).

**Next Steps:**
*   Awaiting new user directives for further development, enhancements to existing servers, or new projects.
*   Consider optional configuration of Pylance (e.g., via `pyrightconfig.json` or VSCode `python.analysis.extraPaths`) to resolve static analysis import errors related to runtime `sys.path` modifications if improved editor diagnostics are desired.
*   Review opportunities for more comprehensive unit and integration testing.

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