use super::*;

#[test]
fn rest_help_lists_actions_and_examples() {
    let help = rest_help();
    let actions = help.get("actions").and_then(|v| v.as_array()).unwrap();
    let names: Vec<&str> = actions.iter().filter_map(|v| v.as_str()).collect();
    assert!(names.contains(&"integrations"));
    assert!(names.contains(&"api_get"));
    assert!(names.contains(&"help"));
    assert!(help.get("examples").unwrap().get("api_delete").is_some());
}
