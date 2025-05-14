# MCP Server Testing Prompt

Use this prompt after creating a new MCP server with the `create-mcp-server_v2.md` template to thoroughly test all available tools and document the results.

## Instructions for Testing an MCP Server

I'd like you to help me test all the tools available in the newly created MCP server for [SERVICE_NAME]. Please follow these steps:

**IMPORTANT: During this testing phase, do NOT make any changes to the MCP server's code, even if you identify errors. Your role is to test, document errors, and then suggest fixes or changes in the summary report or as a separate follow-up action after all tests are complete.**

1. First, identify all available tools provided by the MCP server by examining the connected MCP servers section in your environment details.

2. For each tool:
   - Analyze its input schema to understand required and optional parameters
   - Create appropriate test parameters based on the schema
   - Execute the tool using the `use_mcp_tool` capability
   - Document the result (success or failure)
   - For successful executions, note the key information returned
   - For failures, document the error message

3. Create a comprehensive markdown report of all test results in a file named `[SERVICE_NAME]-tools-test-results.md` with the following structure:

```markdown
# [SERVICE_NAME] MCP Tools Test Results

This document summarizes the testing results of the [SERVICE_NAME] MCP (Model Context Protocol) tools.

## Successfully Tested Tools

1. `[tool_name]` - [Brief description of what the tool does]
   - **Input Parameters Used**: [List the parameters you used for testing]
   - **Results**: [Summarize the key information returned]
   - **Notes**: [Any observations about the tool's behavior, performance, or output format]

2. `[tool_name]` - [Brief description of what the tool does]
   - **Input Parameters Used**: [List the parameters you used for testing]
   - **Results**: [Summarize the key information returned]
   - **Notes**: [Any observations about the tool's behavior, performance, or output format]

[Continue for all successfully tested tools]

## Tools with Errors

1. `[tool_name]` - [Brief description of what the tool was supposed to do]
   - **Input Parameters Used**: [List the parameters you used for testing]
   - **Error**: [The error message received]
   - **Server Logs**: [Relevant log entries from the server log file]
   - **Possible Cause**: [Your analysis of what might be causing the error]
   - **Suggested Fix**: [If applicable, suggestions for fixing the issue]

[Continue for all tools that encountered errors]

## Summary

[Overall assessment of the MCP server's functionality, including:
- Total number of tools tested
- Number of successful tools
- Number of tools with errors
- General observations about the server's performance and reliability
- Any patterns in errors that might indicate systemic issues
- Recommendations for improvements]
```

4. Be thorough in your testing:
   - Try different parameter combinations when appropriate
   - Test edge cases if you can identify them
   - Note any performance issues or unexpected behaviors
   - Consider real-world use cases when selecting test parameters

5. For tools that require specific resources (like IDs, URLs, etc.):
   - Use the most generic/common values possible
   - If you need specific IDs that you don't have, try listing resources first to obtain valid IDs
   - Document any assumptions you made about parameter values

6. For tools that encounter errors:
   - Check the server log file for relevant error messages and stack traces
   - The log file is typically located in the same directory as the server script with a name like `[service]_mcp.log`
   - Include the most relevant portions of the logs in the "Server Logs" section of your documentation
   - Focus on log entries that correspond to the specific error and provide context about what went wrong
   - If the logs are extensive, include the most important parts and summarize the rest

Please begin by identifying all the tools available in the [SERVICE_NAME] MCP server and proceed with testing each one systematically.

## Example Usage

Replace `[SERVICE_NAME]` with the actual name of your MCP server (e.g., "unraid", "plex", "overseerr", etc.) and submit this prompt to the assistant after your MCP server has been created and connected.

The assistant will:
1. Identify all tools from the specified MCP server
2. Test each tool with appropriate parameters
3. Document the results in a structured markdown file
4. Provide a comprehensive summary of the testing

The resulting markdown file will serve as documentation for the MCP server's capabilities and current status, which can be useful for debugging, improvement planning, and user documentation.
