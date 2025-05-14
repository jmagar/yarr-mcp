# Plex MCP Tools Test Results

This document summarizes the testing results of the Plex MCP (Model Context Protocol) tools.

## Successfully Tested Tools

1.  `get_server_info` - Retrieves basic information and status about the connected Plex server.
    *   **Input Parameters Used**: None
    *   **Results**: Successfully returned server details: `{"friendlyName": "TOOTIE", "version": "1.41.7.9749-ce0b45d6e", "platform": "Linux", "platformVersion": "6.12.24-Unraid", "activeSessions": 0, "myPlexUsername": "jmagar@gmail.com"}`
    *   **Notes**: Works as expected.

2.  `list_clients` - Lists available Plex clients connected to the server.
    *   **Input Parameters Used**: None
    *   **Results**: `No clients found connected to the server.`
    *   **Notes**: This might be accurate if no controllable clients were active at the moment of the call.

3.  `get_libraries` - Retrieves a list of all library section names.
    *   **Input Parameters Used**: None
    *   **Results**: Successfully returned library names: `["Movies", "TV", "youTOOT", "LivePhish", "nugs.net"]`
    *   **Notes**: Works as expected. Provided context for other tool tests.

4.  `get_library_size` - Retrieves the total number of items in a specific library.
    *   **Input Parameters Used**: `library_name="Movies"`
    *   **Results**: Successfully returned item count: `{"library_name": "Movies", "item_count": 2017}`
    *   **Notes**: Works as expected.

5.  `list_all_library_titles` - Retrieves a list of all item titles from a specific library.
    *   **Input Parameters Used**: `library_name="TV"`
    *   **Results**: Successfully returned a very long list of TV show titles. Example: `["#FreeRayshawn", "3Below: Tales of Arcadia", ...]`
    *   **Notes**: Works as expected. The output can be very large for large libraries.

6.  `search_library` (specific library) - Searches for media items in a specific library.
    *   **Input Parameters Used**: `query="Inception"`, `library_name="Movies"`
    *   **Results**: Successfully found "Inception": `[{"title": "Inception", "type": "movie", "year": 2010, "summary": "...", "library": "Movies"}]`
    *   **Notes**: Works as expected.

7.  `search_library` (all libraries) - Searches for media items across all libraries.
    *   **Input Parameters Used**: `query="Dark"`
    *   **Results**: Successfully returned multiple items including shows, tracks, and movies related to "Dark". (Example: `[{"title": "Dark", "type": "show", "year": 2017, ...}, ...]`)
    *   **Notes**: Works as expected, demonstrating cross-library search.

8.  `get_library_episodes_count` - Retrieves the total number of episodes in a TV library.
    *   **Input Parameters Used**: `library_name="TV"`
    *   **Results**: Successfully returned episode count: `{"library_name": "TV", "episode_count": 33276}`
    *   **Notes**: Works as expected for a TV library.

9.  `get_music_library_stats` - Retrieves statistics for a music library.
    *   **Input Parameters Used**: `library_name="LivePhish"`
    *   **Results**: Successfully returned stats: `{"library_name": "LivePhish", "artist_count": 8, "album_count": 1169, "track_count": 22888, "total_duration_ms": 11117314606}`
    *   **Notes**: Works as expected for a music library.

10. `play_media` - Plays a media item on a specified Plex client.
    *   **Input Parameters Used**: `item_title="Inception"`, `client_name="PlaceholderClient"`
    *   **Results**: `Error: Plex client 'PlaceholderClient' not found or is not available, or media title 'Inception' not found.`
    *   **Notes**: This is an expected failure as `list_clients` returned no clients, and "PlaceholderClient" is not a real client. The tool correctly reported the issue.

11. `control_playback` - Controls playback on a specific client.
    *   **Input Parameters Used**: `client_name="PlaceholderClient"`, `action="pause"`
    *   **Results**: `Error: Plex client 'PlaceholderClient' not found or is not available.`
    *   **Notes**: Expected failure for the same reasons as `play_media`. Tool correctly reported.

12. `get_active_sessions` - Retrieves information about current playback sessions.
    *   **Input Parameters Used**: None
    *   **Results**: `[{"user": "Jacob Magar", "client": "iPad", "media_title": "The Circle of Strife", "state": "N/A", "progress_ms": 900000, "media_type": "episode"}]`
    *   **Notes**: Successfully retrieved an active session, which is interesting given `list_clients` showed none. This might indicate a difference in how PlexAPI reports clients vs. sessions.

13. `media_stats` - Retrieves comprehensive statistics about all media libraries.
    *   **Input Parameters Used**: None
    *   **Results**: Successfully returned a detailed multi-line string with comprehensive stats for Movies, TV Shows, Music, and Overall Totals (e.g., "Total Movies: 2,017", "Total Episodes: 35,337", "Total Storage: 146.84 TB").
    *   **Notes**: Works as expected, provides a good overview.

## Tools with Errors

1.  `get_recently_added` - Retrieves a list of recently added items from a specific library.
    *   **Input Parameters Used**: `library_name="Movies"`, `limit=2`
    *   **Error**: `Error calling tool: Parameter 'limit' must be one of types [integer, null], got number` (This error occurred on two attempts).
    *   **Server Logs**: *(Cannot access server logs directly. This error message is from the tool call response.)*
    *   **Possible Cause**: The MCP client system might be sending the `limit` parameter as a float or a generic number type, while the server-side tool (or Pydantic model used by FastMCP) strictly expects an integer or null. The tool schema definition in the `mcp_mcp-plex_get_recently_added` function likely specifies `limit: Optional[int] = 10`.
    *   **Suggested Fix**:
        *   Verify the `plex_mcp_server.py` tool definition for `get_recently_added` strictly uses `limit: Optional[int]`.
        *   If the server code is correct, the issue might be in the calling mechanism (how the current client I'm using interprets schema and sends parameters). Ensure integer types are preserved.
        *   As a workaround in the server, one could try `limit: Optional[Union[int, float]] = 10` and then cast `int(limit)` inside the function, but strict typing is generally preferred. The primary fix should be ensuring the client sends the correct type based on the tool's schema.

## Summary

-   **Total number of tools tested**: 14
-   **Number of successful tools**: 13 (most tools functioned as expected or failed gracefully with appropriate messages for non-existent resources)
-   **Number of tools with errors**: 1 (`get_recently_added` due to a parameter type mismatch)
-   **General observations**:
    *   The Plex MCP server is largely functional and robust.
    *   Tools that depend on specific client or media availability (like `play_media`, `control_playback`) correctly reported errors when those resources weren't found with placeholder names.
    *   There's a slight discrepancy noted: `list_clients` reported no clients, while `get_active_sessions` found an active session. This isn't necessarily an error but a point of observation about Plex API behavior.
-   **Recommendations for improvements**:
    *   Investigate the parameter type issue for the `limit` parameter in `get_recently_added`. Ensure the FastMCP server correctly defines it as `Optional[int]` and that the client system respects this typing when making the call. 