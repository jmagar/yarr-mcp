use super::match_quality_profile_id;
use serde_json::json;

fn profiles() -> serde_json::Value {
    json!([
        {"id": 1, "name": "Any"},
        {"id": 4, "name": "HD-1080p"},
        {"id": 6, "name": "Ultra-HD"}
    ])
}

#[test]
fn resolves_name_to_id_case_insensitively() {
    assert_eq!(
        match_quality_profile_id(&profiles(), "HD-1080p").unwrap(),
        4
    );
    assert_eq!(
        match_quality_profile_id(&profiles(), "hd-1080p").unwrap(),
        4
    );
    assert_eq!(
        match_quality_profile_id(&profiles(), "  Ultra-HD ").unwrap(),
        6
    );
}

#[test]
fn unknown_name_errors_with_available_list() {
    let err = match_quality_profile_id(&profiles(), "4K").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no quality profile named `4K`"), "{msg}");
    // Teaching error lists the available profile names.
    assert!(
        msg.contains("Any") && msg.contains("HD-1080p") && msg.contains("Ultra-HD"),
        "{msg}"
    );
}

#[test]
fn empty_profile_list_errors() {
    assert!(match_quality_profile_id(&json!([]), "HD-1080p").is_err());
}
