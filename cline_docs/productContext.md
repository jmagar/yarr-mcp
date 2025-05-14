# Product Context: Unifi Site Manager MCP Server

## Why This Project Exists
This project was initiated to create a Model Context Protocol (MCP) server that interfaces with the Ubiquiti Unifi Site Manager API. The primary goal is to enable Language Models (LLMs) and other automated systems to programmatically access and interact with Unifi network configurations, device information, and performance metrics.

## What Problems It Solves
- **Programmatic Access for LLMs**: Provides a structured way for LLMs to query Unifi network data and potentially trigger management actions (though current tools are read-only).
- **Automation**: Facilitates automation of network monitoring, reporting, and potentially management tasks by integrating Unifi data into larger workflows.
- **Centralized Interaction**: Offers a standardized MCP interface, abstracting the direct complexities of the Unifi Site Manager API for client applications.

## How It Should Work
The system is an MCP server built using Python and the FastMCP library. It operates using Server-Sent Events (SSE) for transport, as requested.

Key operational aspects:
- **Authentication**: Uses an API key (`UNIFI_API_KEY`) provided via an environment variable to authenticate with the Unifi Site Manager API.
- **Tool-Based Interface**: Exposes functionalities as discrete tools, each corresponding to one or more Unifi Site Manager API endpoints.
- **Automatic Pagination**: For API endpoints that return lists of items (e.g., hosts, sites, devices), the server automatically handles pagination to fetch and return all available results in a single tool call.
- **Rate Limiting Resilience**: The server includes logic to respect `Retry-After` headers from the API when rate limits are encountered.
- **Error Handling**: Standard HTTP errors and API-specific errors are caught and reported.
- **Early Access (EA) Endpoints**: Includes tools that utilize EA endpoints from the Unifi API, as per user confirmation, with awareness of their potential instability and stricter rate limits.
- **Configuration**: Relies on environment variables for critical settings like API key and base URL. 