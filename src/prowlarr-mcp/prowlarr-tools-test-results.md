# Prowlarr MCP Tools Test Results

This document summarizes the testing results of the Prowlarr MCP (Model Context Protocol) tools, executed on 2025-05-14.

## Tools Tested

1.  `mcp_mcp-prowlarr_list_indexers`
2.  `mcp_mcp-prowlarr_get_indexer_details`
3.  `mcp_mcp-prowlarr_search_releases`
4.  `mcp_mcp-prowlarr_test_indexer`
5.  `mcp_mcp-prowlarr_list_applications`
6.  `mcp_mcp-prowlarr_get_system_status`
7.  `mcp_mcp-prowlarr_get_indexer_categories`
8.  `mcp_mcp-prowlarr_get_history`
9.  `mcp_mcp-prowlarr_test_all_indexers`
10. `mcp_mcp-prowlarr_update_indexer` (Skipped due to complexity and risk of unintended changes without a specific update task)

## Overall Summary

Most tools functioned correctly. Key issues identified revolve around:
*   The format of the `categories` parameter for `search_releases`.
*   Strict integer type validation for parameters like `limit` (in `search_releases`) and `pageSize` (in `get_history`), similar to issues observed with other MCP servers.

## Detailed Test Results

### 1. `mcp_mcp-prowlarr_list_indexers`
    *   **Status**: Success
    *   **Input**: `random_string: "test"`
    *   **Output**: Successfully listed 3 indexers (NZBgeek, NZBPlanet, TorrentLeech) with their details.
    *   **Notes**: Works as expected.

### 2. `mcp_mcp-prowlarr_get_indexer_details`
    *   **Status**: Success
    *   **Input**: `id: 10` (NZBgeek)
    *   **Output**: Successfully retrieved detailed information for NZBgeek.
    *   **Notes**: Works as expected.

### 3. `mcp_mcp-prowlarr_search_releases`

    *   **Test 3a: Basic Query**
        *   **Status**: Success
        *   **Input**: `query: "ubuntu"`
        *   **Output**: Successfully returned 50 search results for "ubuntu".
        *   **Notes**: Basic search works.

    *   **Test 3b: Query with Categories**
        *   **Status**: Failure
        *   **Input**: `query: "avengers"`, `categories: [2000, 5000]`
        *   **Error**: Prowlarr API error: `The value '2000,5000' is not valid.`
        *   **Notes**: The Prowlarr API likely expects categories as a comma-separated string in the URL or multiple distinct query parameters, not a list that gets stringified directly with brackets/commas in a way that it interprets as a single invalid category ID. The MCP server needs to format this parameter correctly for the Prowlarr API call.

    *   **Test 3c: Query with Indexer ID**
        *   **Status**: Success
        *   **Input**: `query: "linux iso"`, `indexerIds: [10]`
        *   **Output**: Successfully returned 50 results for "linux iso" from NZBgeek.
        *   **Notes**: Filtering by `indexerIds` works.

    *   **Test 3d: Query with Limit**
        *   **Status**: Failure
        *   **Input**: `query: "debian"`, `limit: 3`
        *   **Error**: `Parameter 'limit' must be one of types [integer, null], got number`
        *   **Notes**: Integer typing issue. The MCP server or calling client needs to ensure `limit` is passed as a strict integer.

### 4. `mcp_mcp-prowlarr_test_indexer`
    *   **Status**: Failure (Reported by Prowlarr, tool call itself likely okay)
    *   **Input**: `id: 10` (NZBgeek)
    *   **Output**: Test failed for indexer 'NZBgeek'. Prowlarr API Error 400 - `VipExpiration`: "Must be a future date" (attemptedValue: "2025-04-12").
    *   **Notes**: The tool correctly triggered the Prowlarr test, which failed due to an indexer configuration issue in Prowlarr. See Test 9 for a related observation.

### 5. `mcp_mcp-prowlarr_list_applications`
    *   **Status**: Success
    *   **Input**: `random_string: "test_apps"`
    *   **Output**: Successfully listed 2 applications (Radarr, Sonarr) with configurations.
    *   **Notes**: Works as expected.

### 6. `mcp_mcp-prowlarr_get_system_status`
    *   **Status**: Success
    *   **Input**: `random_string: "test_status"`
    *   **Output**: Successfully retrieved Prowlarr system status (Version 1.35.1.5034, OS: arch, Docker: Yes, etc.).
    *   **Notes**: Works as expected.

### 7. `mcp_mcp-prowlarr_get_indexer_categories`
    *   **Status**: Success
    *   **Input**: `random_string: "test_categories"`
    *   **Output**: Successfully retrieved 9 top-level indexer categories and their subcategories.
    *   **Notes**: Works as expected.

### 8. `mcp_mcp-prowlarr_get_history`

    *   **Test 8a: No Parameters**
        *   **Status**: Success
        *   **Input**: None
        *   **Output**: Retrieved first 20 (of 21192) history events.
        *   **Notes**: Basic history retrieval works.

    *   **Test 8b: With `pageSize`**
        *   **Status**: Failure
        *   **Input**: `pageSize: 5`
        *   **Error**: `Parameter 'pageSize' must be one of types [integer, null], got number`
        *   **Notes**: Integer typing issue.

    *   **Test 8c: With `indexerIds`**
        *   **Status**: Success
        *   **Input**: `indexerIds: [10]`
        *   **Output**: Successfully retrieved history for NZBgeek (20 of 6927 events).
        *   **Notes**: Filtering by `indexerIds` works.

    *   **Test 8d: With `eventType`**
        *   **Status**: Success
        *   **Input**: `eventType: [1]`
        *   **Output**: Successfully retrieved 'releaseGrabbed' events (20 of 1403).
        *   **Notes**: Works, though the exact integer mapping for event types (e.g., 1 = releaseGrabbed, 2 = indexerQuery, etc.) should be confirmed from Prowlarr API documentation for robust use.

### 9. `mcp_mcp-prowlarr_test_all_indexers`
    *   **Status**: Success
    *   **Input**: `random_string: "test_all"`
    *   **Output**: "Tested 3 indexers. Passed: 3, Failed: 0." Indexers 2, 10, 11 reported as `isValid: true`.
    *   **Notes**: Interestingly, this reported indexer 10 (NZBgeek) as passing, while the individual `test_indexer` (Test 4) reported a failure for it due to `VipExpiration`. This might indicate different pass/fail criteria between the two API calls in Prowlarr or that the `VipExpiration` is a warning not a hard failure for `test_all`.

### 10. `mcp_mcp-prowlarr_update_indexer`
    *   **Status**: Skipped
    *   **Notes**: This tool requires a detailed `indexer_config` (full `IndexerResource` or subset of fields) and was skipped to avoid unintended configuration changes without a specific update task and known valid configuration.

## Recommendations

1.  **Strict Integer Typing**: Ensure that parameters defined as integers in the Prowlarr API (e.g., `limit` for `search_releases`, `pageSize` for `get_history`) are strictly passed as `int` by the MCP server and its calling mechanisms. Pydantic models on the server should use `Optional[int]`.
2.  **`search_releases` Categories Parameter Formatting**: Investigate the Prowlarr API documentation for the `/api/v1/search` endpoint (or equivalent used by the tool) to determine the correct way to pass multiple category IDs. The MCP server's client code needs to format the `List[int]` of categories into the expected format (e.g., comma-separated string like `&cat=2000,5000` or multiple `&cat=2000&cat=5000` parameters) before making the `httpx` request.
3.  **Clarify `eventType` Integer Mapping for `get_history`**: For robust use of the `eventType` filter in `get_history`, the integer codes corresponding to Prowlarr's event type strings (e.g., `indexerQuery`, `releaseGrabbed`) should be mapped and used accurately.
4.  **`test_indexer` vs `test_all_indexers` Discrepancy**: While not critical, note the slight difference in reporting for indexer 10 between the single and 'test all' calls. This might be Prowlarr API behavior. 