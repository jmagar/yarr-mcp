//! Colocated tests for the Radarr V3 models — proves the headline resources
//! decode from representative `/api/v3` JSON, that unknown upstream fields are
//! ignored, and that an empty object yields all-None / empty-Vec.

use super::*;
use serde_json::json;

#[test]
fn movie_resource_decodes_representative_payload() {
    // Trimmed `GET /api/v3/movie/{id}` body with nested ratings, language,
    // images, statistics and a movie file.
    let value = json!({
        "id": 42,
        "title": "Blade Runner 2049",
        "originalLanguage": { "id": 1, "name": "English" },
        "alternateTitles": [
            { "id": 7, "sourceType": "tmdb", "movieMetadataId": 3, "title": "BR2049" }
        ],
        "secondaryYearSourceId": 0,
        "sizeOnDisk": 12884901888_i64,
        "status": "released",
        "images": [
            { "coverType": "poster", "url": "/poster.jpg", "remoteUrl": "https://example/p.jpg" }
        ],
        "year": 2017,
        "qualityProfileId": 6,
        "hasFile": true,
        "movieFileId": 99,
        "monitored": true,
        "minimumAvailability": "announced",
        "isAvailable": true,
        "runtime": 164,
        "imdbId": "tt1856101",
        "tmdbId": 335984,
        "tags": [1, 2, 3],
        "genres": ["Science Fiction", "Drama"],
        "ratings": {
            "imdb": { "votes": 500000, "value": 8.0, "type": "user" },
            "tmdb": { "votes": 12000, "value": 7.5 }
        },
        "collection": { "title": "Blade Runner Collection", "tmdbId": 422837 },
        "popularity": 41.123,
        "statistics": {
            "movieFileCount": 1,
            "sizeOnDisk": 12884901888_i64,
            "releaseGroups": ["GROUP"]
        }
    });

    let movie: MovieResource = serde_json::from_value(value).expect("movie decodes");

    assert_eq!(movie.id, 42);
    assert_eq!(movie.title.as_deref(), Some("Blade Runner 2049"));
    assert_eq!(movie.status, Some(MovieStatusType::Released));
    assert_eq!(movie.minimum_availability, Some(MovieStatusType::Announced));
    assert_eq!(movie.year, 2017);
    assert_eq!(movie.quality_profile_id, 6);
    assert!(movie.monitored);
    assert_eq!(movie.size_on_disk, Some(12884901888));
    assert_eq!(movie.tags, vec![1, 2, 3]);
    assert_eq!(movie.genres.len(), 2);
    assert_eq!(movie.images.len(), 1);
    assert_eq!(movie.images[0].cover_type, Some(MediaCoverTypes::Poster));
    assert_eq!(movie.original_language.as_ref().unwrap().id, 1);
    assert_eq!(
        movie.alternate_titles[0].source_type,
        Some(SourceType::Tmdb)
    );

    // The reserved-word rename: `type` → `kind`.
    let imdb = movie.ratings.as_ref().unwrap().imdb.as_ref().unwrap();
    assert_eq!(imdb.kind, Some(RatingType::User));
    assert!((imdb.value - 8.0).abs() < f64::EPSILON);
    assert_eq!(imdb.votes, 500000);

    assert_eq!(movie.collection.as_ref().unwrap().tmdb_id, 422837);
    assert!((movie.popularity - 41.123).abs() < f64::EPSILON);
    assert_eq!(movie.statistics.as_ref().unwrap().movie_file_count, 1);
}

#[test]
fn queue_paging_resource_decodes_and_handles_lowercase_keys() {
    // `GET /api/v3/queue` paged envelope; note lowercase `sizeleft` /
    // `timeleft` and the date-span TimeSpan string.
    let value = json!({
        "page": 1,
        "pageSize": 20,
        "sortKey": "timeleft",
        "sortDirection": "ascending",
        "totalRecords": 1,
        "records": [
            {
                "id": 5,
                "movieId": 42,
                "customFormatScore": 0,
                "size": 8589934592.0,
                "title": "Some.Release.2160p",
                "status": "downloading",
                "trackedDownloadStatus": "ok",
                "trackedDownloadState": "downloading",
                "downloadClientHasPostImportCategory": false,
                "protocol": "torrent",
                "sizeleft": 1073741824.0,
                "timeleft": "00:10:00",
                "statusMessages": [
                    { "title": "Note", "messages": ["line one", "line two"] }
                ]
            }
        ]
    });

    let page: QueueResourcePagingResource =
        serde_json::from_value(value).expect("queue page decodes");

    assert_eq!(page.page, 1);
    assert_eq!(page.total_records, 1);
    assert_eq!(page.sort_direction, Some(SortDirection::Ascending));
    assert_eq!(page.records.len(), 1);

    let item = &page.records[0];
    assert_eq!(item.id, 5);
    assert_eq!(item.movie_id, Some(42));
    assert_eq!(item.status, Some(QueueStatus::Downloading));
    assert_eq!(item.protocol, Some(DownloadProtocol::Torrent));
    assert!((item.sizeleft - 1073741824.0).abs() < f64::EPSILON);
    assert_eq!(item.timeleft.as_deref(), Some("00:10:00"));
    assert_eq!(item.status_messages[0].messages.len(), 2);
}

#[test]
fn history_paging_resource_decodes_with_data_map() {
    // `GET /api/v3/history` — `data` is an additionalProperties:string map.
    let value = json!({
        "page": 1,
        "pageSize": 10,
        "totalRecords": 1,
        "records": [
            {
                "id": 88,
                "movieId": 42,
                "sourceTitle": "Some.Release.1080p",
                "customFormatScore": 25,
                "qualityCutoffNotMet": false,
                "date": "2026-06-22T12:00:00Z",
                "eventType": "downloadFolderImported",
                "data": { "indexer": "NZBgeek", "releaseGroup": "GRP" }
            }
        ]
    });

    let page: HistoryResourcePagingResource =
        serde_json::from_value(value).expect("history page decodes");

    assert_eq!(page.records.len(), 1);
    let row = &page.records[0];
    assert_eq!(row.id, 88);
    assert_eq!(
        row.event_type,
        Some(MovieHistoryEventType::DownloadFolderImported)
    );
    let data = row.data.as_ref().expect("data map present");
    assert_eq!(data.get("indexer").map(String::as_str), Some("NZBgeek"));
    assert_eq!(data.get("releaseGroup").map(String::as_str), Some("GRP"));
}

#[test]
fn unknown_fields_are_ignored() {
    // A root folder body carrying a field the model does not declare must still
    // decode (serde ignores unknown fields by default).
    let value = json!({
        "id": 1,
        "path": "/movies",
        "accessible": true,
        "freeSpace": 5497558138880_i64,
        "unmappedFolders": [
            { "name": "Loose Movie", "path": "/movies/loose", "relativePath": "loose" }
        ],
        "totallyNewUpstreamField": "ignore me",
        "anotherUnknown": { "nested": [1, 2, 3] }
    });

    let folder: RootFolderResource = serde_json::from_value(value).expect("unknown fields ignored");

    assert_eq!(folder.id, 1);
    assert_eq!(folder.path.as_deref(), Some("/movies"));
    assert!(folder.accessible);
    assert_eq!(folder.free_space, Some(5497558138880));
    assert_eq!(folder.unmapped_folders.len(), 1);
    assert_eq!(
        folder.unmapped_folders[0].relative_path.as_deref(),
        Some("loose")
    );
}

#[test]
fn empty_object_yields_none_and_empty_vecs() {
    // Only the spec's value-type primitives are required; every nullable
    // reference/string/array must default to None / empty.
    let media_info: MediaInfoResource = serde_json::from_value(json!({
        "id": 0,
        "audioBitrate": 0,
        "audioChannels": 0.0,
        "audioStreamCount": 0,
        "videoBitDepth": 0,
        "videoBitrate": 0,
        "videoFps": 0.0
    }))
    .expect("minimal media info decodes");

    assert_eq!(media_info.audio_codec, None);
    assert_eq!(media_info.resolution, None);
    assert_eq!(media_info.subtitles, None);

    // A custom format with only its required scalars: optional fields None,
    // arrays empty.
    let format: CustomFormatResource =
        serde_json::from_value(json!({ "id": 3 })).expect("minimal custom format decodes");
    assert_eq!(format.id, 3);
    assert_eq!(format.name, None);
    assert_eq!(format.include_custom_format_when_renaming, None);
    assert!(format.specifications.is_empty());

    // A status message object with no `messages` key → empty Vec.
    let msg: TrackedDownloadStatusMessage =
        serde_json::from_value(json!({ "title": "Just a title" })).expect("status message decodes");
    assert_eq!(msg.title.as_deref(), Some("Just a title"));
    assert!(msg.messages.is_empty());
}

#[test]
fn release_resource_indexer_flags_stays_raw_json() {
    // `indexerFlags` is untyped in the spec → kept as raw JSON, and the numeric
    // `imdbId` is an integer here (unlike MovieResource.imdbId).
    let value = json!({
        "id": 1,
        "customFormatScore": 0,
        "qualityWeight": 0,
        "age": 1,
        "ageHours": 12.5,
        "ageMinutes": 750.0,
        "size": 8589934592_i64,
        "indexerId": 4,
        "title": "Some.Release.2160p.WEB-DL",
        "sceneSource": false,
        "approved": true,
        "temporarilyRejected": false,
        "rejected": false,
        "tmdbId": 335984,
        "imdbId": 1856101,
        "movieRequested": true,
        "downloadAllowed": true,
        "releaseWeight": 0,
        "protocol": "torrent",
        "indexerFlags": 9
    });

    let release: ReleaseResource = serde_json::from_value(value).expect("release decodes");

    assert_eq!(release.id, 1);
    assert_eq!(release.imdb_id, 1856101);
    assert_eq!(release.tmdb_id, 335984);
    assert_eq!(release.protocol, Some(DownloadProtocol::Torrent));
    assert_eq!(release.indexer_flags, Some(json!(9)));
    assert!((release.age_hours - 12.5).abs() < f64::EPSILON);
}
