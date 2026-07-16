use super::*;

#[test]
fn generated_reference_lists_every_bazarr_and_tracearr_curated_action() {
    let document = render();
    for command in curated_commands().iter().filter(|command| {
        matches!(
            command.capability,
            Capability::Subtitles | Capability::Trace
        )
    }) {
        assert!(
            document.contains(&format!("| `{}` |", command.name)),
            "missing generated row for {}",
            command.name
        );
    }
    assert!(document.contains("## Bazarr Subtitle Actions"));
    assert!(document.contains("## Tracearr Actions"));
    assert!(!document.contains("## GenericOnly Services"));
    assert!(document.contains("last_reviewed: \"2026-07-16\""));
}
