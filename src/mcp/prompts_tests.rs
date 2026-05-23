//! Unit tests for src/mcp/prompts.rs

use super::*;

#[test]
fn list_prompts_returns_quick_start() {
    let result = list_prompts();
    let names: Vec<&str> = result.prompts.iter().map(|p| p.name.as_str()).collect();
    assert!(
        names.contains(&"quick_start"),
        "expected quick_start prompt"
    );
}

#[test]
fn get_prompt_quick_start_returns_message() {
    let result = get_prompt(rmcp::model::GetPromptRequestParams::new("quick_start"))
        .expect("quick_start should resolve");
    assert!(
        !result.messages.is_empty(),
        "prompt should have at least one message"
    );
}

#[test]
fn get_prompt_unknown_returns_err() {
    let result = get_prompt(rmcp::model::GetPromptRequestParams::new("nonexistent"));
    assert!(result.is_err(), "unknown prompt should return Err");
}
