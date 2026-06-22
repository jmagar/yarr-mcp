//! Deserialization fixtures for the MediaServer (Jellyfin) models.

use super::*;
use serde_json::json;

#[test]
fn query_result_decodes_items_and_counts() {
    let raw = json!({
        "Items": [{
            "Name": "The Matrix",
            "Id": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
            "ServerId": "srv-001",
            "Type": "Movie",
            "MediaType": "Video",
            "RunTimeTicks": 81_600_000_000_i64,
            "ProductionYear": 1999,
            "CommunityRating": 8.7,
            "ProviderIds": { "Imdb": "tt0133093", "Tmdb": "603" },
            "Genres": ["Action", "Sci-Fi"],
            "UserData": {
                "PlaybackPositionTicks": 0,
                "PlayCount": 3,
                "IsFavorite": true,
                "Played": true,
                "Key": "603",
                "ItemId": "a1b2c3d4e5f60718293a4b5c6d7e8f90"
            }
        }],
        "TotalRecordCount": 1,
        "StartIndex": 0
    });
    let page: BaseItemDtoQueryResult = serde_json::from_value(raw).unwrap();
    assert_eq!(page.total_record_count, 1);
    assert_eq!(page.start_index, 0);
    let item = &page.items[0];
    assert_eq!(item.name.as_deref(), Some("The Matrix"));
    assert_eq!(item.kind, BaseItemKind::Known("Movie".to_string()));
    assert_eq!(item.media_type, MediaType::Known("Video".to_string()));
    assert_eq!(item.run_time_ticks, Some(81_600_000_000));
    assert_eq!(item.genres, vec!["Action", "Sci-Fi"]);
    assert_eq!(
        item.provider_ids
            .as_ref()
            .unwrap()
            .get("Imdb")
            .map(String::as_str),
        Some("tt0133093")
    );
    let ud = item.user_data.as_ref().expect("user_data present");
    assert_eq!(ud.play_count, 3);
    assert!(ud.is_favorite);
    assert_eq!(ud.item_id, "a1b2c3d4e5f60718293a4b5c6d7e8f90");
}

#[test]
fn session_decodes_play_state_and_now_playing() {
    let raw = json!({
        "Id": "session-7",
        "UserId": "deadbeefdeadbeefdeadbeefdeadbeef",
        "UserName": "alice",
        "Client": "Jellyfin Web",
        "LastActivityDate": "2024-01-02T03:04:05.000Z",
        "LastPlaybackCheckIn": "2024-01-02T03:04:05.000Z",
        "IsActive": true,
        "SupportsMediaControl": true,
        "SupportsRemoteControl": false,
        "HasCustomDeviceName": false,
        "PlayState": {
            "PositionTicks": 12_000_000_000_i64,
            "CanSeek": true,
            "IsPaused": false,
            "IsMuted": false,
            "PlayMethod": "DirectPlay",
            "RepeatMode": "RepeatNone",
            "PlaybackOrder": "Default"
        },
        "NowPlayingItem": {
            "Name": "Dune: Part Two",
            "Type": "Movie",
            "MediaType": "Video"
        }
    });
    let session: SessionInfoDto = serde_json::from_value(raw).unwrap();
    assert_eq!(session.user_id, "deadbeefdeadbeefdeadbeefdeadbeef");
    assert_eq!(session.user_name.as_deref(), Some("alice"));
    assert!(session.is_active);
    let ps = session.play_state.expect("play_state present");
    assert!(ps.can_seek);
    assert!(!ps.is_paused);
    assert_eq!(ps.position_ticks, Some(12_000_000_000));
    assert_eq!(ps.repeat_mode, "RepeatNone");
    let now = session.now_playing_item.expect("now_playing present");
    assert_eq!(now.name.as_deref(), Some("Dune: Part Two"));
}

#[test]
fn extra_unknown_fields_are_ignored() {
    let raw = json!({
        "Name": "Server",
        "Version": "10.9.11",
        "Id": "srv-xyz",
        "ProductName": "Jellyfin Server",
        "FutureFieldNotYetModelled": { "deep": [1, 2, 3] },
        "AnotherUnknown": "ignored"
    });
    let info: PublicSystemInfo = serde_json::from_value(raw).unwrap();
    assert_eq!(info.version.as_deref(), Some("10.9.11"));
    assert_eq!(info.product_name.as_deref(), Some("Jellyfin Server"));
    assert_eq!(info.id.as_deref(), Some("srv-xyz"));
}

#[test]
fn empty_objects_yield_none_and_empty_vecs() {
    // An empty BaseItemDto still requires its non-nullable enum fields, which
    // default to their `Default` impls (`kind` -> empty, `media_type` -> Unknown).
    let item: BaseItemDto = serde_json::from_value(json!({})).unwrap();
    assert!(item.name.is_none());
    assert!(item.id.is_none());
    assert!(item.provider_ids.is_none());
    assert!(item.user_data.is_none());
    assert!(item.current_program.is_none());
    assert!(item.genres.is_empty());
    assert!(item.media_streams.is_empty());
    assert!(item.external_urls.is_empty());
    assert_eq!(item.kind, BaseItemKind::default());
    assert_eq!(item.media_type, MediaType::default());

    // A library row defaults Locations to an empty Vec when absent.
    let folder: VirtualFolderInfo = serde_json::from_value(json!({})).unwrap();
    assert!(folder.name.is_none());
    assert!(folder.locations.is_empty());
    assert!(folder.collection_type.is_none());
}

#[test]
fn virtual_folder_and_user_policy_decode() {
    let folder: VirtualFolderInfo = serde_json::from_value(json!({
        "Name": "Movies",
        "Locations": ["/data/movies", "/data/movies-4k"],
        "CollectionType": "movies",
        "ItemId": "folder-1"
    }))
    .unwrap();
    assert_eq!(folder.name.as_deref(), Some("Movies"));
    assert_eq!(folder.locations, vec!["/data/movies", "/data/movies-4k"]);

    let user: UserDto = serde_json::from_value(json!({
        "Name": "admin",
        "Id": "ffffffffffffffffffffffffffffffff",
        "LastLoginDate": "2024-06-01T12:00:00.000Z",
        "Policy": {
            "IsAdministrator": true,
            "IsHidden": false,
            "EnableCollectionManagement": true,
            "EnableSubtitleManagement": true,
            "EnableLyricManagement": false,
            "IsDisabled": false,
            "EnableUserPreferenceAccess": true,
            "EnableRemoteControlOfOtherUsers": false,
            "EnableSharedDeviceControl": true,
            "EnableRemoteAccess": true,
            "EnableLiveTvManagement": true,
            "EnableLiveTvAccess": true,
            "EnableMediaPlayback": true,
            "EnableAudioPlaybackTranscoding": true,
            "EnableVideoPlaybackTranscoding": true,
            "EnablePlaybackRemuxing": true,
            "ForceRemoteSourceTranscoding": false,
            "EnableContentDeletion": false,
            "EnableContentDownloading": true,
            "EnableSyncTranscoding": true,
            "EnableMediaConversion": true,
            "EnableAllDevices": true,
            "EnableAllChannels": true,
            "EnableAllFolders": true,
            "InvalidLoginAttemptCount": 0,
            "LoginAttemptsBeforeLockout": -1,
            "MaxActiveSessions": 0,
            "EnablePublicSharing": true,
            "RemoteClientBitrateLimit": 0,
            "SyncPlayAccess": "CreateAndJoinGroups"
        }
    }))
    .unwrap();
    assert_eq!(user.name.as_deref(), Some("admin"));
    let policy = user.policy.expect("policy present");
    assert!(policy.is_administrator);
    assert_eq!(policy.login_attempts_before_lockout, -1);
    assert_eq!(policy.sync_play_access, "CreateAndJoinGroups");
    // Absent Vec fields on the policy default cleanly.
    assert!(policy.blocked_tags.is_empty());
    assert!(policy.enabled_folders.is_empty());
}
