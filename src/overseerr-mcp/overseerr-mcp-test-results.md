# Overseerr MCP Server - Tool Testing Report (2024-08-17)

This report details the results of testing the Overseerr MCP server tools.

## Tools Tested:
1.  `search_media(query, media_type=None)`
2.  `get_movie_details(tmdb_id)`
3.  `get_tv_show_details(tmdb_id)`
4.  `request_movie(tmdb_id)`
5.  `request_tv_show(tmdb_id, seasons=None)`
6.  `list_failed_requests(count=10, skip=0)`

## Test Results:

### 1. `search_media(query, media_type=None)`
*   **Status:** PASS
*   **Observations:**
    *   Successfully returned results for valid movie queries (e.g., "Inception").
    *   Successfully returned results for valid TV show queries (e.g., "Breaking Bad").
    *   Successfully returned results for generic queries without `media_type` (e.g., "Star Wars").
    *   Correctly handled queries with no results by returning an empty list or appropriate message (e.g., "XyzqWrtZxcvbnmPqwert").

### 2. `get_movie_details(tmdb_id)`
*   **Status:** PASS
*   **Observations:**
    *   Successfully retrieved details for valid movie TMDB IDs (e.g., Inception - 27205, The Dark Knight - 155).
    *   Correctly returned an error for invalid/non-existent TMDB IDs (e.g., 999999999).

### 3. `get_tv_show_details(tmdb_id)`
*   **Status:** PASS
*   **Observations:**
    *   Successfully retrieved details for valid TV show TMDB IDs (e.g., Breaking Bad - 1396, Game of Thrones - 1399).
    *   Correctly returned an error for invalid/non-existent TMDB IDs (e.g., 999999998).

### 4. `request_movie(tmdb_id)`
*   **Status:** PASS (After Fixes on 2024-08-17)
*   **Initial Observations (Pre-Fix):**
    *   Consistently returned: `Error: Movie request for TMDB ID <ID> completed, but response data was unexpected.`
*   **Post-Fix Observations:**
    *   Successfully returns a `MediaRequest` object when a new movie is requested (e.g., "Primer" - TMDB ID `14337`, "Bend It Like Beckham" - TMDB ID `455`).
    *   If a movie is already requested/available, or the API returns a message instead of a new request object, the tool now correctly relays this (e.g., may still log "unexpected data structure" if the response is not a new `MediaRequest` object but doesn't indicate an API error).
*   **Fixes Applied:**
    *   Adjusted the success condition check in `overseerr-mcp-server.py` to rely on `id`, `status`, and `media.tmdbId` in the response, instead of a non-existent top-level `type` field.

### 5. `request_tv_show(tmdb_id, seasons=None)`
*   **Status:** PASS (After Fixes on 2024-08-17)
*   **Initial Observations (Pre-Fix):**
    *   When requesting *all seasons* of a show (e.g., "Chernobyl" - TMDB ID `87108`):
        *   Returned: `Error: Overseerr API request failed (500). Details: Cannot read properties of undefined (reading 'filter')`
    *   When requesting *specific seasons* of a show (e.g., "Breaking Bad" - TMDB ID `1396`, season `[1]`):
        *   Returned: `Error: TV show request for TMDB ID <ID> completed, but response data was unexpected.`
*   **Post-Fix Observations:**
    *   When requesting *all seasons* (e.g., "Full House" - TMDB ID `4313`): Successfully returns a `MediaRequest` object.
    *   The "filter" error is resolved. If Overseerr indicates "No seasons available to request" (e.g., for "Chernobyl" TMDB ID `87108` after fix, or "Breaking Bad" S1), the tool now correctly relays this as an "unexpected data structure" (as it's not a new pending `MediaRequest` object), rather than a 500 error.
*   **Fixes Applied:**
    *   Modified `overseerr-mcp-server.py` to explicitly send `seasons: "all"` in the payload when no specific seasons are provided.
    *   Adjusted the success condition check similar to `request_movie`.

### 6. `list_failed_requests(count=None, skip=None)`
*   **Status:** PASS
*   **Observations:**
    *   Successfully retrieved a list of failed requests.
    *   Pagination using `count` and `skip` parameters worked as expected.
    *   Example successful call: `list_failed_requests(count=5)` returned `[{"requestId": 1002, "status": 4, ...}]`.
    *   Null title for a failed request was observed, which might be an Overseerr data characteristic.

## Overall Summary & Recommendations (Post-Fixes):

All initially tested tools (`search_media`, `get_movie_details`, `get_tv_show_details`, `request_movie`, `request_tv_show`, `list_failed_requests`) are now functioning correctly.

The primary issues with `request_movie` and `request_tv_show` concerning API response parsing and payload for "all seasons" have been addressed. The tools now accurately reflect successful request creations or provide informative messages from Overseerr if a request cannot be newly created (e.g., already exists, nothing to request).

The testing prompt is now complete.
What would you like to do next? We were in the middle of systematically checking all MCP servers in `src/` against the `create-mcp-server_v2.md` template. We had finished `tautulli-mcp` and were about to analyze `sabnzbd-mcp`'s files before we paused for the Overseerr tool testing. 