//! MCP prompts for the rustarr server.
//!
//! Prompts are pre-canned message templates that MCP clients can invoke.
//! They appear in the "Prompts" section of compatible MCP UIs.
//!
//! **Template**: replace `quick_start` with prompts relevant to your domain.

use rmcp::model::{
    GetPromptRequestParams, GetPromptResult, ListPromptsResult, Prompt, PromptMessage,
    PromptMessageRole,
};

pub(super) fn list_prompts() -> ListPromptsResult {
    ListPromptsResult {
        prompts: vec![Prompt::new(
            "quick_start",
            Some(
                "Inspect configured media integrations and fetch one configured \
                 service status to verify the MCP connection end-to-end.",
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
            "Use the rustarr tool with action=integrations to list configured services. \
             If at least one service is configured, call action=service_status with that service name. \
             Report back both results.",
        )])
        .with_description("Verify Rustarr MCP connectivity with integrations and service status")),
        other => Err(anyhow::anyhow!("unknown prompt: {other}")),
    }
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
