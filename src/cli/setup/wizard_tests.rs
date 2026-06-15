//! Tests for wizard pure helpers: `dotenv_assignment` quoting and the
//! [`SetupCommand`] enum.

use super::*;

#[test]
fn dotenv_assignment_leaves_safe_values_bare() {
    assert_eq!(
        dotenv_assignment("RUSTARR_MCP_PORT", "40070").unwrap(),
        "RUSTARR_MCP_PORT=40070"
    );
}

#[test]
fn dotenv_assignment_quotes_values_with_spaces() {
    assert_eq!(
        dotenv_assignment("KEY", "two words").unwrap(),
        "KEY=\"two words\""
    );
}

#[test]
fn dotenv_assignment_rejects_newlines() {
    assert!(dotenv_assignment("KEY", "a\nb").is_err());
}

#[test]
fn setup_command_equality() {
    assert_eq!(SetupCommand::Check, SetupCommand::Check);
    assert_ne!(SetupCommand::Check, SetupCommand::Repair);
    assert_ne!(
        SetupCommand::PluginHook { no_repair: true },
        SetupCommand::PluginHook { no_repair: false }
    );
}
