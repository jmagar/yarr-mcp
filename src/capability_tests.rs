use super::*;
use crate::config::ServiceKind;

#[test]
fn every_kind_has_descriptor_and_capability() {
    for kind in ServiceKind::ALL {
        // descriptor() and capability() must agree and not panic.
        assert_eq!(kind.capability(), kind.descriptor().capability);
    }
}

#[test]
fn arr_version_and_resource_table_is_correct() {
    assert_eq!(ServiceKind::Sonarr.capability(), Capability::ArrManager);
    assert_eq!(ServiceKind::Sonarr.descriptor().api_prefix, "/api/v3");
    assert_eq!(
        ServiceKind::Sonarr.descriptor().resource_noun,
        Some("series")
    );

    assert_eq!(ServiceKind::Radarr.capability(), Capability::ArrManager);
    assert_eq!(ServiceKind::Radarr.descriptor().api_prefix, "/api/v3");
    assert_eq!(
        ServiceKind::Radarr.descriptor().resource_noun,
        Some("movie")
    );
}

#[test]
fn metadata_profile_axis_is_typed_per_kind_not_a_deny_list() {
    // C3 typed seam: supported arr kinds do not carry a separate metadata axis.
    // Future applicability differences can key off this typed flag rather than a
    // per-(action, kind) deny list.
    assert!(!ServiceKind::Sonarr.descriptor().has_metadata_profiles);
    assert!(!ServiceKind::Radarr.descriptor().has_metadata_profiles);
    // Non-arr kinds never carry the flag.
    for kind in [
        ServiceKind::Plex,
        ServiceKind::Sabnzbd,
        ServiceKind::Prowlarr,
    ] {
        assert!(!kind.descriptor().has_metadata_profiles);
    }
}

#[test]
fn capability_classes_match_kinds() {
    assert_eq!(ServiceKind::Prowlarr.capability(), Capability::Indexer);
    assert_eq!(ServiceKind::Overseerr.capability(), Capability::Requests);
    assert_eq!(ServiceKind::Tautulli.capability(), Capability::Stats);
    assert_eq!(
        ServiceKind::Sabnzbd.capability(),
        Capability::DownloadClient
    );
    assert_eq!(
        ServiceKind::Qbittorrent.capability(),
        Capability::DownloadClient
    );
    assert_eq!(ServiceKind::Plex.capability(), Capability::MediaServer);
    assert_eq!(ServiceKind::Jellyfin.capability(), Capability::MediaServer);
    assert_eq!(ServiceKind::Bazarr.capability(), Capability::Subtitles);
    assert_eq!(ServiceKind::Tracearr.capability(), Capability::Trace);
}

#[test]
fn generic_only_kinds() {
    assert!(
        ServiceKind::ALL
            .iter()
            .all(|kind| kind.capability() != Capability::GenericOnly)
    );
}

#[test]
fn auth_style_table() {
    assert_eq!(
        ServiceKind::Sonarr.descriptor().auth_style,
        AuthStyle::ApiKeyHeader
    );
    assert_eq!(
        ServiceKind::Sabnzbd.descriptor().auth_style,
        AuthStyle::QueryApiKey
    );
    assert!(ServiceKind::Sabnzbd.descriptor().query_api());
    assert_eq!(
        ServiceKind::Qbittorrent.descriptor().auth_style,
        AuthStyle::CookieSession
    );
    assert_eq!(
        ServiceKind::Plex.descriptor().auth_style,
        AuthStyle::PlexToken
    );
    // query_api is derived from auth_style: PlexToken implies query_api=true.
    assert!(ServiceKind::Plex.descriptor().query_api());
    assert_eq!(
        ServiceKind::Jellyfin.descriptor().auth_style,
        AuthStyle::JellyfinToken
    );
    // JellyfinToken is a header style -> NOT a query-api kind.
    assert!(!ServiceKind::Jellyfin.descriptor().query_api());
    // Header-auth arr kinds are also not query-api.
    assert!(!ServiceKind::Sonarr.descriptor().query_api());
    assert!(ServiceKind::Tautulli.descriptor().query_api());
    assert_eq!(
        ServiceKind::Tracearr.descriptor().auth_style,
        AuthStyle::BearerToken
    );
}
