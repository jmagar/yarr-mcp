//! Deserialization fixtures for the Overseerr models.

use super::*;
use serde_json::json;

#[test]
fn request_page_decodes_with_nested_media_and_users() {
    let raw = json!({
        "pageInfo": { "page": 1, "pages": 10, "results": 100, "pageSize": 10 },
        "results": [{
            "id": 42,
            "status": 2,
            "type": "movie",
            "is4k": false,
            "serverId": 0,
            "profileId": 4,
            "rootFolder": "/movies",
            "createdAt": "2020-09-12T10:00:27.000Z",
            "updatedAt": "2020-09-12T11:00:27.000Z",
            "media": {
                "id": 7,
                "tmdbId": 1234,
                "tvdbId": null,
                "status": 5,
                "status4k": 1,
                "mediaType": "movie"
            },
            "requestedBy": {
                "id": 1,
                "email": "hey@itsme.com",
                "username": "admin",
                "userType": 1,
                "permissions": 0,
                "createdAt": "2020-01-01T00:00:00.000Z",
                "updatedAt": "2020-01-02T00:00:00.000Z",
                "displayName": "Admin"
            },
            "modifiedBy": null,
            // unmodelled runtime keys must not break decoding:
            "seasons": [{ "id": 9, "seasonNumber": 1, "name": "Season 1", "episodeCount": 10 }],
            "extraUnknownKey": "ignored"
        }]
    });
    let page: MediaRequestPage = serde_json::from_value(raw).unwrap();
    let info = page.page_info.expect("pageInfo present");
    assert_eq!(info.results, Some(100));
    assert_eq!(info.page_size, Some(10));

    let req = &page.results[0];
    assert_eq!(req.id, Some(42));
    assert_eq!(req.status, Some(2));
    assert_eq!(req.kind.as_deref(), Some("movie"));
    assert_eq!(req.is_4k, Some(false));
    assert_eq!(req.created_at.as_deref(), Some("2020-09-12T10:00:27.000Z"));
    // anyOf [User, nullable string] -> None when null:
    assert!(req.modified_by.is_none());

    let media = req.media.as_ref().expect("media present");
    assert_eq!(media.tmdb_id, Some(1234));
    assert_eq!(media.tvdb_id, None);
    assert_eq!(media.status, Some(5));
    assert_eq!(media.status_4k, Some(1));

    let user = req.requested_by.as_ref().expect("requestedBy present");
    assert_eq!(user.id, Some(1));
    assert_eq!(user.email.as_deref(), Some("hey@itsme.com"));
    assert_eq!(user.user_type, Some(1));
    assert_eq!(user.display_name.as_deref(), Some("Admin"));

    assert_eq!(req.seasons[0].season_number, Some(1));
    assert_eq!(req.seasons[0].episode_count, Some(10));
}

#[test]
fn movie_and_tv_results_decode_with_floats() {
    let movie: MovieResult = serde_json::from_value(json!({
        "id": 1234,
        "mediaType": "movie",
        "title": "The Matrix",
        "popularity": 10.5,
        "voteAverage": 8.7,
        "voteCount": 24000,
        "genreIds": [28, 878],
        "releaseDate": "1999-03-31",
        "adult": false,
        "video": false,
        // unknown field is ignored:
        "tmdbExtra": "nope"
    }))
    .unwrap();
    assert_eq!(movie.id, Some(1234));
    assert_eq!(movie.media_type.as_deref(), Some("movie"));
    assert_eq!(movie.title.as_deref(), Some("The Matrix"));
    assert_eq!(movie.popularity, Some(10.5));
    assert_eq!(movie.vote_average, Some(8.7));
    assert_eq!(movie.genre_ids, vec![28, 878]);

    let tv: TvResult = serde_json::from_value(json!({
        "id": 1399,
        "mediaType": "tv",
        "name": "Game of Thrones",
        "originalName": "Game of Thrones",
        "originCountry": ["US"],
        "firstAirDate": "2011-04-17",
        "voteAverage": 8.4
    }))
    .unwrap();
    assert_eq!(tv.name.as_deref(), Some("Game of Thrones"));
    assert_eq!(tv.origin_country, vec!["US"]);
    assert_eq!(tv.first_air_date.as_deref(), Some("2011-04-17"));
    assert_eq!(tv.vote_average, Some(8.4));
}

#[test]
fn season_with_episodes_and_status_decode() {
    let season: Season = serde_json::from_value(json!({
        "id": 3624,
        "name": "Season 1",
        "seasonNumber": 1,
        "airDate": "2011-04-17",
        "episodeCount": 10,
        "episodes": [{
            "id": 63056,
            "name": "Winter Is Coming",
            "episodeNumber": 1,
            "seasonNumber": 1,
            "showId": 1399,
            "airDate": "2011-04-17",
            "stillPath": null,
            "voteAverage": 7.9,
            "voteCount": 200
        }]
    }))
    .unwrap();
    assert_eq!(season.season_number, Some(1));
    let ep = &season.episodes[0];
    assert_eq!(ep.name.as_deref(), Some("Winter Is Coming"));
    assert_eq!(ep.show_id, Some(1399));
    assert_eq!(ep.still_path, None);
    assert_eq!(ep.vote_average, Some(7.9));

    let status: Status = serde_json::from_value(json!({
        "version": "1.33.2",
        "commitTag": "local",
        "updateAvailable": false,
        "commitsBehind": 0,
        "restartRequired": false
    }))
    .unwrap();
    assert_eq!(status.version.as_deref(), Some("1.33.2"));
    assert_eq!(status.update_available, Some(false));
    assert_eq!(status.commits_behind, Some(0));
}

#[test]
fn empty_object_yields_none_and_empty_vecs() {
    // Optional/defaulted fields tolerate a bare object.
    let page: MediaRequestPage = serde_json::from_value(json!({})).unwrap();
    assert!(page.page_info.is_none());
    assert!(page.results.is_empty());

    let media: MediaInfo = serde_json::from_value(json!({})).unwrap();
    assert!(media.id.is_none());
    assert!(media.tmdb_id.is_none());
    assert!(media.requests.is_empty());

    // Only the spec-required fields are needed to decode a MediaRequest / User.
    let req: MediaRequest = serde_json::from_value(json!({ "id": 1, "status": 1 })).unwrap();
    assert_eq!(req.id, Some(1));
    assert!(req.media.is_none());
    assert!(req.seasons.is_empty());

    let user: User = serde_json::from_value(json!({
        "id": 9,
        "email": "a@b.com",
        "createdAt": "2020-01-01T00:00:00.000Z",
        "updatedAt": "2020-01-01T00:00:00.000Z"
    }))
    .unwrap();
    assert_eq!(user.id, Some(9));
    assert!(user.username.is_none());
    assert!(user.display_name.is_none());
}
