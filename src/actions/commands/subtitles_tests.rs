use super::*;
use crate::actions::curated_command;

#[test]
fn subtitles_commands_are_registered_to_subtitles_capability() {
    assert_eq!(SUBTITLES_COMMANDS.len(), 7);
    for command in SUBTITLES_COMMANDS {
        assert_eq!(command.capability, Capability::Subtitles);
        assert_eq!(curated_command(command.name).unwrap().name, command.name);
        assert!(!command.mutates);
        assert!(!command.destructive);
    }
}

#[test]
fn paged_subtitles_commands_declare_integer_pagination() {
    let movies = curated_command("subtitles_movies").unwrap();
    assert_eq!(movies.optional_params, &["start", "length"]);
    assert!(movies.typed_params.contains(&("start", Integer)));
    assert!(movies.typed_params.contains(&("length", Integer)));
}
