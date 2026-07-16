use super::*;

#[test]
fn core_enums_follow_the_public_api_wire_values() {
    assert_eq!(
        serde_json::to_value(ServerType::Jellyfin).unwrap(),
        serde_json::json!("jellyfin")
    );
    assert_eq!(
        serde_json::from_value::<PlaybackState>(serde_json::json!("paused")).unwrap(),
        PlaybackState::Paused
    );
}

#[test]
fn optional_stream_details_accept_an_empty_object() {
    let details: StreamDetails = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(details.is_transcode, None);
    assert_eq!(details.source_video_details, None);
    assert_eq!(details.subtitle_info, None);
}
