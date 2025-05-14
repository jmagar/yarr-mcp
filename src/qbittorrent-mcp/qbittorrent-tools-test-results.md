# qBittorrent MCP Tools Test Results

This document summarizes the testing results of the qBittorrent MCP (Model Context Protocol) tools.

## Successfully Tested Tools

1.  `list_torrents` (Test 1a: Default parameters)
    *   **Description**: Lists torrents from qBittorrent.
    *   **Input Parameters Used**: None (defaulted to `filter='all'`)
    *   **Results**: Successfully returned a list of torrents with details like hash, name, size, progress, status, etc.
    *   **Notes**: The tool executed successfully.

2.  `get_qb_transfer_info` (Test 5a: No parameters)
    *   **Description**: Retrieves global transfer information from qBittorrent.
    *   **Input Parameters Used**: `random_string: "test"` (dummy parameter)
    *   **Results**: Successfully returned a dictionary containing global transfer stats like connection status, DHT nodes, download/upload speeds and limits.
    *   **Notes**: The tool executed successfully.

3.  `get_qb_app_preferences` (Test 6a: No parameters)
    *   **Description**: Retrieves qBittorrent application preferences.
    *   **Input Parameters Used**: `random_string: "test"` (dummy parameter)
    *   **Results**: Successfully returned a comprehensive dictionary of qBittorrent application settings.
    *   **Notes**: The tool executed successfully.

## Tools with Errors

1.  `list_torrents` (Test 1b: `filter='downloading'`)
    *   **Description**: Lists torrents filtered by status.
    *   **Input Parameters Used**: `filter="downloading"`
    *   **Error**: `BaseEventLoop.run_in_executor() got an unexpected keyword argument 'filter'`
    *   **Server Logs**: (Assuming similar stack trace to other executor errors if logs were inspected)
        ```
        ERROR:asyncio:Task exception was never retrieved
        future: <Task finished name='Task-XYZ' coro=<Connection.handle_request() done, defined at ...> exception=TypeError("BaseEventLoop.run_in_executor() got an unexpected keyword argument 'filter'")>
        Traceback (most recent call last):
          ...
        TypeError: BaseEventLoop.run_in_executor() got an unexpected keyword argument 'filter'
        ```
    *   **Possible Cause**: The `run_sync_qb_tool` helper in `qbittorrent-mcp-server.py` isn't correctly passing named arguments to the `qbittorrentapi.Client` methods when invoked via `executor.submit(func, **kwargs_for_sync_call)`. The `qbittorrentapi.Client.torrents_info()` method expects `status_filter` not `filter`.
    *   **Suggested Fix**: Modify the `run_sync_qb_tool` or the tool implementation for `list_torrents` to correctly map the `filter` MCP parameter to the `status_filter` parameter for the `qbittorrentapi` call. Specifically, `qb_client.torrents_info(status_filter=filter, category=category, tag=tag)` should be called.

2.  `add_torrent_url` (Test 2a: Add Ubuntu magnet link)
    *   **Description**: Adds a new torrent to qBittorrent using a URL or magnet link.
    *   **Input Parameters Used**: `torrent_url = "magnet:?xt=urn:btih:1a8c983195416f65e665098983a37d6e719e6752&dn=ubuntu-24.04-desktop-amd64.iso"`
    *   **Error**: `BaseEventLoop.run_in_executor() got an unexpected keyword argument 'urls'`
    *   **Server Logs**: (Similar to above)
    *   **Possible Cause**: The `run_sync_qb_tool` helper or the tool implementation is passing `torrent_url` but the `qbittorrentapi.Client.torrents_add()` method expects the parameter `urls`.
    *   **Suggested Fix**: Ensure the `add_torrent_url` tool implementation calls `qb_client.torrents_add(urls=torrent_url, ...)` or maps the `torrent_url` parameter to `urls` correctly when calling the synchronous library method.

3.  `pause_torrent` (Test 3a: Pause a torrent)
    *   **Description**: Pauses a specific torrent.
    *   **Input Parameters Used**: `torrent_hash = "d5f3c4bdd1b9df80197b933b658c2ed33985cb50"`
    *   **Error**: `BaseEventLoop.run_in_executor() got an unexpected keyword argument 'torrent_hashes'`
    *   **Server Logs**: (Similar to above)
    *   **Possible Cause**: The `run_sync_qb_tool` helper or the tool implementation is passing `torrent_hash` but the `qbittorrentapi.Client.torrents_pause()` method expects `torrent_hashes`.
    *   **Suggested Fix**: Ensure the `pause_torrent` tool implementation calls `qb_client.torrents_pause(torrent_hashes=torrent_hash, ...)` or maps the `torrent_hash` parameter to `torrent_hashes`.

4.  `resume_torrent` (Test 4a: Resume a torrent)
    *   **Description**: Resumes a specific torrent.
    *   **Input Parameters Used**: `torrent_hash = "d5f3c4bdd1b9df80197b933b658c2ed33985cb50"`
    *   **Error**: `BaseEventLoop.run_in_executor() got an unexpected keyword argument 'torrent_hashes'`
    *   **Server Logs**: (Similar to above)
    *   **Possible Cause**: The `run_sync_qb_tool` helper or the tool implementation is passing `torrent_hash` but the `qbittorrentapi.Client.torrents_resume()` method expects `torrent_hashes`.
    *   **Suggested Fix**: Ensure the `resume_torrent` tool implementation calls `qb_client.torrents_resume(torrent_hashes=torrent_hash, ...)` or maps the `torrent_hash` parameter to `torrent_hashes`.

## Summary

*   **Total tools tested**: 6
*   **Successful tools**: 3 (`list_torrents` with default params, `get_qb_transfer_info`, `get_qb_app_preferences`)
*   **Tools with errors**: 4 instances across 4 unique tools (`list_torrents` with filter, `add_torrent_url`, `pause_torrent`, `resume_torrent`)

**General Observations**:
The qBittorrent MCP server has a critical issue in how it passes parameters to the synchronous `qbittorrentapi` library calls via the `run_sync_qb_tool` and `ThreadPoolExecutor`. The errors consistently point to `TypeError: BaseEventLoop.run_in_executor() got an unexpected keyword argument '...'`. This indicates that the keyword arguments provided to the MCP tools are not being correctly mapped or passed through to the underlying library functions, especially when the MCP parameter name differs from the library's expected parameter name.

Tools that take no arguments (other than the dummy `random_string`) or where the MCP parameter name coincidentally matches the library's expected name (or the library can infer it positionally, though less likely with `**kwargs`) seem to work.

**Recommendations for Improvements**:
1.  **Fix Parameter Passing in `run_sync_qb_tool`**: The primary issue lies in the `async def run_sync_qb_tool(func, *args, **kwargs): return await asyncio.get_event_loop().run_in_executor(executor, func, *args, **kwargs)` function within `qbittorrent-mcp-server.py`.
    *   When calling `func` inside the executor, the `kwargs` are being passed directly to `run_in_executor` itself, not as arguments to `func`.
    *   It should be something like: `return await asyncio.get_event_loop().run_in_executor(executor, lambda: func(*args, **kwargs))` or by using `functools.partial`.
    *   Example using `partial`:
        ```python
        from functools import partial
        # ...
        async def run_sync_qb_tool(func, *args, **kwargs):
            loop = asyncio.get_event_loop()
            pfunc = partial(func, *args, **kwargs)
            return await loop.run_in_executor(executor, pfunc)
        ```

2.  **Verify Parameter Name Mapping**: For each tool, ensure that the parameter names exposed by the MCP tool are correctly mapped to the parameter names expected by the corresponding `qbittorrentapi.Client` method.
    *   `list_torrents`: MCP `filter` -> `qbittorrentapi` `status_filter`.
    *   `add_torrent_url`: MCP `torrent_url` -> `qbittorrentapi` `urls`.
    *   `pause_torrent`: MCP `torrent_hash` -> `qbittorrentapi` `torrent_hashes`.
    *   `resume_torrent`: MCP `torrent_hash` -> `qbittorrentapi` `torrent_hashes`.

    This mapping should occur within each tool's implementation before calling `run_sync_qb_tool`. For example, in `list_torrents`:
    ```python
    # Inside list_torrents tool function
    # ...
    return await run_sync_qb_tool(
        qb_client.torrents_info, 
        status_filter=filter,  # Correct mapping
        category=category, 
        tag=tag
    )
    ```

Addressing these two points should resolve all the observed errors. 