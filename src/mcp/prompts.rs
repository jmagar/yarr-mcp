//! MCP prompts for the rustarr server.
//!
//! Prompts are pre-canned message templates that MCP clients can invoke.
//! They appear in the "Prompts" section of compatible MCP UIs.
//!
//! Add Rustarr-specific prompts here when recurring operator workflows become
//! worth exposing directly to MCP clients.

use rmcp::model::{
    GetPromptRequestParams, GetPromptResult, ListPromptsResult, Prompt, PromptMessage,
    PromptMessageRole,
};

pub(super) fn list_prompts() -> ListPromptsResult {
    ListPromptsResult {
        prompts: vec![Prompt::new(
            "quick_start",
            Some(
                "Write a short Code Mode script that discovers a configured service \
                 and fetches its status to verify the MCP connection end-to-end.",
            ),
            None,
        )],
        ..Default::default()
    }
}

pub(super) fn get_prompt(request: GetPromptRequestParams) -> anyhow::Result<GetPromptResult> {
    match request.name.as_str() {
        "quick_start" => Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            "Call the `yarr` tool with a Code Mode script. Inside it, use \
             codemode.search('status') to find a service's status callable, then invoke it \
             (e.g. `await sonarr.service_status()`) and return the result. The service is baked \
             into each callable, so you never pass a `service` argument. Report back what you found.",
        )])
        .with_description("Verify Rustarr MCP connectivity with a Code Mode discovery + status call")),
        other => Err(anyhow::anyhow!("unknown prompt: {other}")),
    }
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
