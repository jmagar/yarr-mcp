use super::*;
use crate::actions::curated_command;

#[test]
fn trace_commands_are_registered_to_trace_capability() {
    assert_eq!(TRACE_COMMANDS.len(), 9);
    for command in TRACE_COMMANDS {
        assert_eq!(command.capability, Capability::Trace);
        assert_eq!(curated_command(command.name).unwrap().name, command.name);
    }
}

#[test]
fn terminate_stream_is_the_only_destructive_trace_command() {
    for command in TRACE_COMMANDS {
        if command.name == "trace_terminate_stream" {
            assert!(command.mutates);
            assert!(command.destructive);
        } else {
            assert!(!command.mutates);
            assert!(!command.destructive);
        }
    }
}
