use super::*;
use serde_json::json;

#[test]
fn indexer_resource_decodes_with_nested_capabilities_and_enums() {
    let value = json!({
        "id": 3,
        "name": "MyAnonamouse",
        "implementation": "Cardigann",
        "configContract": "CardigannSettings",
        "tags": [1, 2, 5],
        "indexerUrls": ["https://www.myanonamouse.net/"],
        "enable": true,
        "redirect": false,
        "supportsRss": true,
        "supportsSearch": true,
        "supportsRedirect": false,
        "supportsPagination": true,
        "appProfileId": 1,
        "protocol": "torrent",
        "privacy": "private",
        "priority": 25,
        "downloadClientId": 0,
        "added": "2023-04-01T12:00:00Z",
        "capabilities": {
            "id": 0,
            "limitsMax": 100,
            "limitsDefault": 50,
            "supportsRawSearch": false,
            "searchParams": ["q"],
            "tvSearchParams": ["q", "season", "ep"],
            "categories": [
                {
                    "id": 5000,
                    "name": "TV",
                    "subCategories": [{ "id": 5040, "name": "TV/HD" }]
                }
            ]
        },
        "message": { "message": "Indexer is fine", "type": "info" },
        "status": {
            "id": 7,
            "indexerId": 3,
            "disabledTill": "2023-04-02T00:00:00Z"
        }
    });

    let indexer: IndexerResource = serde_json::from_value(value).unwrap();
    assert_eq!(indexer.id, Some(3));
    assert_eq!(indexer.name.as_deref(), Some("MyAnonamouse"));
    assert_eq!(indexer.tags, vec![1, 2, 5]);
    assert_eq!(indexer.protocol, Some(DownloadProtocol::Torrent));
    assert_eq!(indexer.privacy, Some(IndexerPrivacy::Private));

    let caps = indexer.capabilities.expect("capabilities present");
    assert_eq!(caps.search_params, vec![SearchParam::Q]);
    assert_eq!(caps.categories.len(), 1);
    let tv = &caps.categories[0];
    assert_eq!(tv.id, Some(5000));
    // Recursive sub-category decodes.
    assert_eq!(tv.sub_categories[0].id, Some(5040));

    let msg = indexer.message.expect("message present");
    // `type` -> `kind` rename works for the enum.
    assert_eq!(msg.kind, Some(ProviderMessageType::Info));

    let status = indexer.status.expect("status present");
    assert_eq!(
        status.disabled_till.as_deref(),
        Some("2023-04-02T00:00:00Z")
    );
}

#[test]
fn release_resource_handles_int64_size_and_float_ages() {
    let value = json!({
        "id": 0,
        "guid": "https://indexer/details/abc",
        "age": 12,
        "ageHours": 296.5,
        "ageMinutes": 17790.25,
        "size": 8_589_934_592_i64,
        "indexerId": 3,
        "indexer": "MyAnonamouse",
        "title": "Some.Release.2160p",
        "imdbId": 0,
        "tmdbId": 0,
        "tvdbId": 0,
        "tvMazeId": 0,
        "publishDate": "2023-03-20T08:30:00Z",
        "downloadUrl": "https://indexer/download/abc.torrent",
        "magnetUrl": "magnet:?xt=urn:btih:deadbeef",
        "infoHash": "deadbeef",
        "seeders": 42,
        "leechers": 3,
        "protocol": "torrent",
        "categories": [{ "id": 2040, "name": "Movies/HD" }]
    });

    let release: ReleaseResource = serde_json::from_value(value).unwrap();
    // int64 byte count, not string-encoded.
    assert_eq!(release.size, Some(8_589_934_592));
    assert_eq!(release.age_hours, Some(296.5));
    assert_eq!(release.age_minutes, Some(17790.25));
    assert_eq!(release.protocol, Some(DownloadProtocol::Torrent));
    assert_eq!(release.seeders, Some(42));
    assert_eq!(release.categories[0].id, Some(2040));
}

#[test]
fn indexer_stats_envelope_decodes_three_arrays() {
    let value = json!({
        "id": 1,
        "indexers": [
            {
                "indexerId": 3,
                "indexerName": "MyAnonamouse",
                "averageResponseTime": 412,
                "averageGrabResponseTime": 980,
                "numberOfQueries": 100,
                "numberOfGrabs": 12,
                "numberOfRssQueries": 50,
                "numberOfAuthQueries": 4,
                "numberOfFailedQueries": 1,
                "numberOfFailedGrabs": 0,
                "numberOfFailedRssQueries": 2,
                "numberOfFailedAuthQueries": 0
            }
        ],
        "userAgents": [
            { "userAgent": "Prowlarr/1.0", "numberOfQueries": 80, "numberOfGrabs": 10 }
        ],
        "hosts": [
            { "host": "www.myanonamouse.net", "numberOfQueries": 100, "numberOfGrabs": 12 }
        ]
    });

    let stats: IndexerStatsResource = serde_json::from_value(value).unwrap();
    assert_eq!(stats.indexers.len(), 1);
    assert_eq!(stats.indexers[0].number_of_queries, Some(100));
    assert_eq!(
        stats.user_agents[0].user_agent.as_deref(),
        Some("Prowlarr/1.0")
    );
    assert_eq!(stats.hosts[0].host.as_deref(), Some("www.myanonamouse.net"));
}

#[test]
fn field_keeps_string_hidden_and_untyped_value() {
    let value = json!({
        "order": 0,
        "name": "baseUrl",
        "label": "Base URL",
        "value": ["a", "b"],
        "type": "select",
        "advanced": true,
        "hidden": "hiddenIfNotSet",
        "privacy": "apiKey",
        "isFloat": false,
        "selectOptions": [
            { "value": 1, "name": "First", "order": 0 }
        ]
    });

    let field: Field = serde_json::from_value(value).unwrap();
    // `type` -> `kind` rename.
    assert_eq!(field.kind.as_deref(), Some("select"));
    // `hidden` stays a string.
    assert_eq!(field.hidden.as_deref(), Some("hiddenIfNotSet"));
    // Untyped freeform value preserved verbatim.
    assert_eq!(field.value, Some(json!(["a", "b"])));
    assert_eq!(field.privacy, Some(PrivacyLevel::ApiKey));
    assert_eq!(field.select_options[0].value, Some(1));
}

#[test]
fn unknown_fields_are_ignored() {
    let value = json!({
        "id": 9,
        "label": "anime",
        "color": "#ff0000",
        "createdAt": "2024-01-01T00:00:00Z",
        "totallyNewProwlarrField": { "nested": true }
    });

    let tag: TagResource = serde_json::from_value(value).unwrap();
    assert_eq!(tag.id, Some(9));
    assert_eq!(tag.label.as_deref(), Some("anime"));
}

#[test]
fn empty_object_yields_all_none_and_empty_vecs() {
    let indexer: IndexerResource = serde_json::from_value(json!({})).unwrap();
    assert_eq!(indexer.id, None);
    assert_eq!(indexer.name, None);
    assert_eq!(indexer.protocol, None);
    assert!(indexer.fields.is_empty());
    assert!(indexer.tags.is_empty());
    assert!(indexer.presets.is_empty());
    assert!(indexer.indexer_urls.is_empty());

    let release: ReleaseResource = serde_json::from_value(json!({})).unwrap();
    assert_eq!(release.size, None);
    assert_eq!(release.age_hours, None);
    assert!(release.categories.is_empty());
    assert!(release.indexer_flags.is_empty());

    let system: SystemResource = serde_json::from_value(json!({})).unwrap();
    assert_eq!(system.app_name, None);
    assert_eq!(system.mode, None);
    assert_eq!(system.database_type, None);
}

#[test]
fn health_resource_renames_type_enum() {
    let value = json!({
        "id": 1,
        "source": "IndexerStatusCheck",
        "type": "warning",
        "message": "Indexers unavailable",
        "wikiUrl": "https://wiki/health"
    });

    let health: HealthResource = serde_json::from_value(value).unwrap();
    assert_eq!(health.kind, Some(HealthCheckResult::Warning));
    assert_eq!(health.source.as_deref(), Some("IndexerStatusCheck"));
}
