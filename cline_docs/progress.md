# Progress

**Status:** Core MCP server functionalities for Plex, Overseerr, SABnzbd, qBittorrent, and Tautulli have been implemented. Project is in a usable state for these services via their respective MCP servers.

**What Works:**

*   **Plex MCP Server (`plex-mcp`):**
    *   Connection to Plex server via URL and Token.
    *   Tools: `get_libraries`, `search_library`, `play_media`, `get_server_info`, `list_clients`, `get_active_sessions`, `get_recently_added`, `get_library_size`, `list_all_library_titles`.
*   **Overseerr MCP Server (`overseerr-mcp`):**
    *   Connection to Overseerr via URL and API Key.
    *   API client (`OverseerrApiClient`) using `httpx`.
    *   Tools: `search_media`, `get_movie_details`, `get_tv_show_details`, `request_movie`, `request_tv_show`, `list_failed_requests`.
*   **SABnzbd MCP Server (`sabnzbd-mcp`):**
    *   Connection to SABnzbd via URL and API Key.
    *   API client (`SabnzbdApiClient`) using `httpx`.
    *   Tools: `get_sab_queue`, `get_sab_history`, `pause_sab_queue`, `resume_sab_queue`, `add_nzb_url`, `set_sab_speedlimit`.
*   **qBittorrent MCP Server (`qbittorrent-mcp`):**
    *   Connection to qBittorrent via URL, Username, Password using `qbittorrent-api`.
    *   Tools run synchronous client methods in thread executor: `list_torrents`, `add_torrent_url`, `pause_torrent`, `resume_torrent`, `get_qb_transfer_info`, `get_qb_app_preferences`.
*   **Tautulli MCP Server (`tautulli-mcp`):**
    *   Connection to Tautulli via URL and API Key.
    *   API client (`TautulliApiClient`) using `httpx`.
    *   Tools: `get_tautulli_activity`, `get_tautulli_home_stats`, `get_tautulli_history`, `get_tautulli_libraries`, `get_tautulli_users`.
*   **General:**
    *   Modular project structure (`src/xxx-mcp/`).
    *   Lifespan management for API clients in each server.
    *   Configuration via a root `.env` file.
    *   Basic logging implemented in all servers.
    *   Individual READMEs for each server module and an updated main project README.
    *   Resolved critical Python import issues for running servers via `fastmcp run path/to/server.py:mcp`.
*   **Unifi Site Manager MCP Server:**
    *   **Core Server Implementation**: The FastMCP server (`unifi_mcp_server.py`) is created and configured for SSE transport.
    *   **Environment Configuration**: Setup for `.env` file to manage API key and URLs is in place.
    *   **API Interaction Helper**: The `_make_api_request` function successfully handles:
        *   Authentication with the Unifi API.
        *   Asynchronous GET and POST requests using `httpx`.
        *   Basic error handling for HTTP status codes.
        *   API-specific error message parsing.
        *   Rate limit detection and `Retry-After` logic.
        *   Automatic fetching of all pages for paginated list endpoints.
    *   **Implemented Tools**: All 9 planned tools have been implemented:
        1.  `list_hosts`: Fetches all hosts with pagination.
        2.  `get_host_by_id`: Fetches a specific host.
        3.  `list_sites`: Fetches all sites with pagination.
        4.  `list_devices`: Fetches all devices (grouped by host) with pagination and optional filters.
        5.  `get_isp_metrics` (EA): Fetches ISP metrics for all sites.
        6.  `query_isp_metrics` (EA): Fetches ISP metrics based on specific queries.
        7.  `list_sdwan_configs` (EA): Fetches all SD-WAN configurations.
        8.  `get_sdwan_config_by_id` (EA): Fetches a specific SD-WAN configuration.
        9.  `get_sdwan_config_status` (EA): Fetches the status of an SD-WAN configuration.
    *   **Project Documentation**: `README.md` created with setup, usage, and client examples.
    *   **Dependency Management**: `requirements.txt` is up to date.
    *   **Initial Debugging**: Resolved `ImportError` issues related to FastMCP context parameters by removing the unused `ctx` argument from tool definitions.

**What's Left to Build (Potential Future Enhancements):**

*   **More Advanced Tools:** For any server, e.g., specific item manipulation (delete, modify), deeper filtering in searches, more granular controls.
*   **Plex Specific:** More playback controls (volume, skip, stop), managing playlists, editing metadata (if API allows and desired).
*   **Overseerr Specific:** Approving/denying requests, managing users (if permissions allow).
*   **SABnzbd Specific:** Individual job pause/resume/delete, script management.
*   **qBittorrent Specific:** Deleting torrents (with data), managing trackers, more detailed torrent properties.
*   **Tautulli Specific:** More specific stat queries, generating charts/graphs (if feasible via MCP return types), triggering notifications.
*   **Cross-Server Workflows:** Tools that coordinate actions between multiple MCP servers (e.g., search Overseerr, if not available request it, then monitor SABnzbd/qBittorrent, then see it in Plex via Tautulli stats).
*   **Error Handling:** More structured error responses from tools instead of just strings (e.g., JSON with error codes/messages).
*   **Testing:** Comprehensive unit and integration tests for all servers and tools.
*   **Pylance/Static Analysis Configuration:** Set up `pyrightconfig.json` or VSCode `python.analysis.extraPaths` to correctly resolve imports for a smoother development experience.
*   **Alternative Transports:** Further testing or implementation of SSE/HTTP transports if STDIO proves limiting for certain client use cases.
*   **Packaging/Deployment:** If the project were to be distributed.

**Overall Progress:** Significantly advanced. A suite of functional MCP servers for key media applications is now available. Approximately 75-85% complete for the initially scoped multi-server setup with core functionalities.

## Unifi Site Manager MCP Server

### What's Left to Build / Test
- **Comprehensive Live Testing**: 
    - Test all tools against a live Unifi Site Manager API with actual data.
    - Verify automatic pagination works correctly for `list_hosts`, `list_sites`, and `list_devices` with multiple pages of data.
    - Confirm behavior with Early Access endpoints (`get_isp_metrics`, `query_isp_metrics`, `list_sdwan_configs`, `get_sdwan_config_by_id`, `get_sdwan_config_status`), especially regarding data structure and rate limits.
    - Test error handling for various scenarios (invalid API key, network issues, non-existent IDs, API rate limits).
    - Validate the structure and completeness of data returned by each tool.
    - Test `list_devices` filtering options (`host_ids`, `time`).
    - Test `get_isp_metrics` with different `metric_type` and time filter combinations.
    - Test `query_isp_metrics` with valid and potentially problematic `sites_query` structures.
- **Client Integration Testing**: Test connecting to the SSE server using MCP clients like Cline or the VS Code extension to ensure proper communication and tool invocation.
- **Refinement (Post-Testing)**:
    - Improve error messages or data transformations based on testing outcomes.
    - Optimize `_make_api_request` if any performance bottlenecks are identified with pagination.

### Progress Status
- **Development**: 95% (Core implementation complete, awaiting thorough testing).
- **Documentation**: 90% (Initial README and Memory Bank files created, will be finalized after testing).
- **Testing**: 10% (Only basic import/runtime checks done, full functional testing pending). 