//! Colocated tests for the Bazarr models.
//!
//! Decode representative fixtures for each envelope convention, prove the mixed
//! camelCase/snake_case renames land on the right fields, and cover the
//! "unknown fields are ignored" and "empty object decodes to all-None / empty Vec"
//! invariants.

use super::*;
use serde_json::json;

#[test]
fn system_status_decodes_with_float_start_time_and_null_cpu_cores() {
    let v = json!({
        "data": {
            "bazarr_version": "v1.4.3",
            "package_version": "",
            "sonarr_version": "4.0.0.0",
            "radarr_version": "",
            "operating_system": "Linux-6.0.0",
            "python_version": "3.11.2",
            "database_engine": "Postgresql 15.2",
            "database_migration": "abc123",
            "bazarr_directory": "/app/bazarr",
            "bazarr_config_directory": "/config",
            "start_time": 1719000000.123,
            "timezone": "America/New_York",
            "cpu_cores": null
        }
    });

    let status: SystemStatus = serde_json::from_value(v).unwrap();
    let data = status.data.expect("data present");
    assert_eq!(data.bazarr_version.as_deref(), Some("v1.4.3"));
    assert_eq!(data.package_version.as_deref(), Some(""));
    assert_eq!(data.database_engine.as_deref(), Some("Postgresql 15.2"));
    assert_eq!(data.start_time, Some(1_719_000_000.123));
    // os.cpu_count() can be null.
    assert_eq!(data.cpu_cores, None);
}

#[test]
fn movies_response_decodes_paged_envelope_with_renames_and_string_year() {
    let v = json!({
        "data": [
            {
                "alternativeTitles": ["The Matrix Reloaded", "Matrix 2"],
                "audio_language": [
                    { "name": "English", "code2": "en", "code3": "eng" }
                ],
                "fanart": "/proxy/fanart.jpg",
                "imdbId": "tt0133093",
                "missing_subtitles": [
                    { "name": "French", "code2": "fr", "code3": "fra",
                      "forced": false, "hi": false }
                ],
                "monitored": true,
                "overview": "A computer hacker learns the truth.",
                "path": "/movies/The Matrix (1999)/The.Matrix.mkv",
                "poster": "/proxy/poster.jpg",
                "profileId": 1,
                "radarrId": 42,
                "sceneName": "The.Matrix.1999.1080p.BluRay",
                "subtitles": [
                    { "name": "English", "code2": "en", "code3": "eng",
                      "path": "/movies/The Matrix (1999)/The.Matrix.en.srt",
                      "forced": false, "hi": false, "file_size": 54321 }
                ],
                "tags": ["sci-fi"],
                "title": "The Matrix",
                "year": "1999"
            }
        ],
        "total": 1
    });

    let resp: MoviesGetResponse = serde_json::from_value(v).unwrap();
    assert_eq!(resp.total, Some(1));
    assert_eq!(resp.data.len(), 1);

    let m = &resp.data[0];
    assert_eq!(
        m.alternative_titles,
        vec!["The Matrix Reloaded", "Matrix 2"]
    );
    assert_eq!(m.imdb_id.as_deref(), Some("tt0133093"));
    assert_eq!(m.radarr_id, Some(42));
    assert_eq!(m.profile_id, Some(1));
    assert_eq!(
        m.scene_name.as_deref(),
        Some("The.Matrix.1999.1080p.BluRay")
    );
    assert_eq!(m.monitored, Some(true));
    // year is a string-encoded number on the wire.
    assert_eq!(m.year.as_deref(), Some("1999"));

    // audio_language is a LIST despite the flask_restx single-Nested declaration.
    assert_eq!(m.audio_language.len(), 1);
    assert_eq!(m.audio_language[0].code2.as_deref(), Some("en"));

    // subtitles carry path + file_size.
    assert_eq!(m.subtitles.len(), 1);
    assert_eq!(m.subtitles[0].file_size, Some(54321));
    assert_eq!(m.tags, vec!["sci-fi"]);
}

#[test]
fn episodes_data_envelope_and_wanted_episode_label_decode() {
    // Episodes endpoint: { "data": [...] } with NO total.
    let v = json!({
        "data": [
            {
                "audio_language": [{ "name": "English", "code2": "en", "code3": "eng" }],
                "episode": 3,
                "missing_subtitles": [
                    { "name": "Spanish", "code2": "es", "code3": "spa",
                      "forced": false, "hi": false }
                ],
                "monitored": true,
                "path": "/tv/Show/S01E03.mkv",
                "season": 1,
                "sonarrEpisodeId": 1001,
                "sonarrSeriesId": 7,
                "subtitles": [],
                "title": "Pilot, Part 3",
                "sceneName": "Show.S01E03.1080p"
            }
        ]
    });

    #[derive(serde::Deserialize)]
    struct EpisodesEnvelope {
        #[serde(default)]
        data: Vec<EpisodeSubtitleStatus>,
    }
    let env: EpisodesEnvelope = serde_json::from_value(v).unwrap();
    assert_eq!(env.data.len(), 1);
    let e = &env.data[0];
    assert_eq!(e.episode, Some(3));
    assert_eq!(e.season, Some(1));
    assert_eq!(e.sonarr_episode_id, Some(1001));
    assert_eq!(e.sonarr_series_id, Some(7));
    assert_eq!(e.scene_name.as_deref(), Some("Show.S01E03.1080p"));
    assert_eq!(e.missing_subtitles.len(), 1);
    assert!(e.subtitles.is_empty());

    // Wanted episode: episode_number is a '1x3' label string.
    let wv = json!({
        "data": [
            {
                "seriesTitle": "Some Show",
                "episode_number": "1x3",
                "episodeTitle": "Pilot, Part 3",
                "missing_subtitles": [],
                "sonarrSeriesId": 7,
                "sonarrEpisodeId": 1001,
                "sceneName": "Show.S01E03.1080p",
                "tags": [],
                "seriesType": "standard"
            }
        ],
        "total": 1
    });
    let resp: WantedEpisodesResponse = serde_json::from_value(wv).unwrap();
    assert_eq!(resp.total, Some(1));
    let w = &resp.data[0];
    assert_eq!(w.episode_number.as_deref(), Some("1x3"));
    assert_eq!(w.series_title.as_deref(), Some("Some Show"));
    assert_eq!(w.series_type.as_deref(), Some("standard"));
    assert!(w.missing_subtitles.is_empty());
}

#[test]
fn provider_and_language_bare_array_decode() {
    // Providers: { "data": [...] }.
    let pv = json!({
        "data": [
            { "name": "opensubtitles", "status": "Good", "retry": "-" },
            { "name": "addic7ed", "status": "Throttled: too many requests",
              "retry": "in 5 minutes" }
        ]
    });

    #[derive(serde::Deserialize)]
    struct ProvidersEnvelope {
        #[serde(default)]
        data: Vec<ProviderStatus>,
    }
    let env: ProvidersEnvelope = serde_json::from_value(pv).unwrap();
    assert_eq!(env.data.len(), 2);
    assert_eq!(env.data[0].status.as_deref(), Some("Good"));
    assert_eq!(env.data[0].retry.as_deref(), Some("-"));
    assert_eq!(env.data[1].retry.as_deref(), Some("in 5 minutes"));

    // system/languages: a BARE array, no envelope.
    let lv = json!([
        { "name": "English", "code2": "en", "code3": "eng", "enabled": true },
        { "name": "French", "code2": "fr", "code3": "fra", "enabled": false }
    ]);
    let langs: Vec<Language> = serde_json::from_value(lv).unwrap();
    assert_eq!(langs.len(), 2);
    assert_eq!(langs[0].code2.as_deref(), Some("en"));
    assert_eq!(langs[0].enabled, Some(true));
    assert_eq!(langs[1].enabled, Some(false));
}

#[test]
fn unknown_fields_are_ignored() {
    // Extra upstream keys must deserialize-and-ignore.
    let v = json!({
        "data": {
            "bazarr_version": "v1.4.3",
            "some_future_field": "ignored",
            "nested_unknown": { "a": 1, "b": [2, 3] }
        }
    });
    let status: SystemStatus = serde_json::from_value(v).unwrap();
    let data = status.data.expect("data present");
    assert_eq!(data.bazarr_version.as_deref(), Some("v1.4.3"));
    assert_eq!(data.cpu_cores, None);

    // Same for a movie row carrying surprise fields.
    let mv = json!({
        "title": "Unknown Movie",
        "radarrId": 9,
        "unexpected": "drop me",
        "weird_nested": { "x": true }
    });
    let m: MovieSubtitleStatus = serde_json::from_value(mv).unwrap();
    assert_eq!(m.title.as_deref(), Some("Unknown Movie"));
    assert_eq!(m.radarr_id, Some(9));
    assert!(m.tags.is_empty());
}

#[test]
fn empty_objects_decode_to_all_none_and_empty_vecs() {
    let empty = json!({});

    let data: SystemStatusData = serde_json::from_value(empty.clone()).unwrap();
    assert_eq!(data.bazarr_version, None);
    assert_eq!(data.start_time, None);
    assert_eq!(data.cpu_cores, None);

    let movie: MovieSubtitleStatus = serde_json::from_value(empty.clone()).unwrap();
    assert_eq!(movie.title, None);
    assert_eq!(movie.year, None);
    assert!(movie.alternative_titles.is_empty());
    assert!(movie.audio_language.is_empty());
    assert!(movie.missing_subtitles.is_empty());
    assert!(movie.subtitles.is_empty());
    assert!(movie.tags.is_empty());

    let episode: EpisodeSubtitleStatus = serde_json::from_value(empty.clone()).unwrap();
    assert_eq!(episode.episode, None);
    assert_eq!(episode.sonarr_series_id, None);
    assert!(episode.audio_language.is_empty());
    assert!(episode.missing_subtitles.is_empty());
    assert!(episode.subtitles.is_empty());

    // Paged envelope with no fields -> empty data, None total.
    let movies: MoviesGetResponse = serde_json::from_value(empty.clone()).unwrap();
    assert!(movies.data.is_empty());
    assert_eq!(movies.total, None);

    let provider: ProviderStatus = serde_json::from_value(empty.clone()).unwrap();
    assert_eq!(provider.name, None);
    assert_eq!(provider.status, None);
    assert_eq!(provider.retry, None);

    let language: Language = serde_json::from_value(empty).unwrap();
    assert_eq!(language.name, None);
    assert_eq!(language.enabled, None);
}
