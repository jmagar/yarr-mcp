//! Request-argument and request-body synthesis for live contract invocations —
//! picks concrete path/query/body values from seeded fixtures for a given
//! operation. Split out of `contract.rs` to keep that module under the
//! PATTERNS.md file-size limit.

use serde_json::{Map, Value, json};

use yarr::ServiceKind;
use yarr::openapi::{HttpMethod, OperationSpec};

use super::FixtureStore;
use super::is_scalar;

pub(super) fn can_reuse_fixture_body(op: &OperationSpec) -> bool {
    matches!(op.method, HttpMethod::Put | HttpMethod::Patch)
        || op.path.ends_with("/test")
        || op.path.contains("/test/")
        || op.path.contains("/action/")
}

pub(super) fn live_fixture_body_for_op(
    kind: ServiceKind,
    op: &OperationSpec,
    fixtures: &FixtureStore,
) -> Option<Value> {
    match (kind, op.name) {
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_command") => {
            Some(json!({ "name": "RefreshMonitoredDownloads" }))
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr | ServiceKind::Prowlarr, "post_tag") => {
            Some(json!({ "label": unique_live_label(kind, op.name) }))
        }
        // Provider-backed resources (downloadclient/indexer/notification/metadata)
        // validate `implementation`/`configContract`/`fields` server-side in a way
        // the generic OpenAPI-schema synthesizer can't discover. Reuse the live
        // `GET .../schema` template primed into `fixtures.provider` — it's already
        // a valid, ready-to-POST body — with a fresh unique `name`.
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_downloadclient") => {
            named_provider_template(&fixtures.provider.downloadclient, kind, op.name)
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_indexer") => {
            named_provider_template(&fixtures.provider.indexer, kind, op.name)
        }
        // The `/test` sibling of a create is also a POST, so it runs in the SAME
        // seed phase (see `seed_phase`) — cross-op fixture harvesting can't see a
        // same-phase create's result yet, so `/test` needs its own copy of the
        // same valid template rather than relying on `can_reuse_fixture_body`'s
        // reuse-from-fixtures fallback (which would otherwise pick up whatever a
        // PRIOR phase happened to harvest, e.g. a seeded fixture with an invalid
        // path for this specific check).
        (
            ServiceKind::Sonarr | ServiceKind::Radarr,
            "post_notification" | "post_notification_test",
        ) => named_provider_template(&fixtures.provider.notification, kind, op.name),
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_metadata") => {
            named_provider_template(&fixtures.provider.metadata, kind, op.name)
        }
        // ImportListResource also requires a real `rootFolderPath` and
        // `qualityProfileId` that the schema template leaves as placeholders.
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_importlist" | "post_importlist_test") => {
            let mut body = named_provider_template(&fixtures.provider.importlist, kind, op.name)?;
            let obj = body.as_object_mut()?;
            if let Some(path) = &fixtures.provider.root_folder_path {
                obj.insert("rootFolderPath".into(), json!(path));
            }
            if let Some(id) = &fixtures.provider.quality_profile_id {
                obj.insert("qualityProfileId".into(), id.clone());
            }
            Some(body)
        }
        // AutoTagging/CustomFormat both reject an empty `tags`/`specifications`
        // array (business-rule validation, not visible in the OpenAPI schema).
        // `GET .../schema` gives a valid specification item; a live tag was
        // created during priming for the `tags` requirement. The specification
        // item's own `name` ("condition name") isn't in the schema template
        // either — Sonarr rejects it as empty unless set explicitly.
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_autotagging") => {
            let spec = named_provider_template(
                &fixtures.provider.autotagging_spec,
                kind,
                "autotagging-condition",
            )?;
            let tag = fixtures.provider.tag_id.clone()?;
            Some(json!({
                "name": unique_live_label(kind, op.name),
                "removeTagsAutomatically": false,
                "tags": [tag],
                "specifications": [spec],
            }))
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_customformat") => {
            let spec = named_provider_template(
                &fixtures.provider.customformat_spec,
                kind,
                "customformat-condition",
            )?;
            Some(json!({
                "name": unique_live_label(kind, op.name),
                "specifications": [spec],
            }))
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_delayprofile") => {
            let tag = fixtures.provider.tag_id.clone()?;
            Some(json!({
                "enableUsenet": true,
                "enableTorrent": true,
                "preferredProtocol": "usenet",
                "usenetDelay": 0,
                "torrentDelay": 0,
                "bypassIfHighestQuality": false,
                "bypassIfAboveCustomFormatScore": false,
                "minimumCustomFormatScore": 0,
                "order": 1,
                "tags": [tag],
            }))
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_rootfolder") => fixtures
            .provider
            .unmapped_root_folder_path
            .as_ref()
            .map(|path| json!({ "path": path })),
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_remotepathmapping") => Some(json!({
            "host": unique_live_label(kind, op.name),
            "remotePath": "/downloads/",
            "localPath": fixtures.provider.root_folder_path.as_deref().unwrap_or("/data"),
        })),
        // `ReleaseProfileResource.required`/`ignored` are untyped (nullable) in the
        // spec, so the generic synthesizer emits `{}`; Sonarr needs at least one of
        // the two populated with a term (string or string array both accepted).
        (ServiceKind::Sonarr | ServiceKind::Radarr, "post_releaseprofile") => Some(json!({
            "required": unique_live_label(kind, op.name),
        })),
        // Bulk PUT ops need a real `ids: [...]` array pulled from a resource this
        // same contract sweep already created earlier in phase 2 (POST creates run
        // before PUTs; see `seed_phase`) — the generic synth leaves `ids` empty.
        (ServiceKind::Sonarr | ServiceKind::Radarr, "put_customformat_bulk") => {
            bulk_ids_body(fixtures, "/api/v3/customformat")
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "put_downloadclient_bulk") => {
            bulk_ids_body(fixtures, "/api/v3/downloadclient")
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "put_importlist_bulk") => {
            bulk_ids_body(fixtures, "/api/v3/importlist")
        }
        (ServiceKind::Sonarr | ServiceKind::Radarr, "put_indexer_bulk") => {
            bulk_ids_body(fixtures, "/api/v3/indexer")
        }
        _ => None,
    }
}

fn named_provider_template(
    template: &Option<Value>,
    kind: ServiceKind,
    op_name: &str,
) -> Option<Value> {
    let mut body = template.clone()?;
    body.as_object_mut()?
        .insert("name".into(), json!(unique_live_label(kind, op_name)));
    Some(body)
}

fn bulk_ids_body(fixtures: &FixtureStore, path: &str) -> Option<Value> {
    let ids = fixtures.values_for(path)?;
    if ids.is_empty() {
        return None;
    }
    Some(json!({ "ids": ids }))
}

pub(super) fn unique_live_label(kind: ServiceKind, op_name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("yarr-live-{}-{op_name}-{nanos}", kind.as_str())
}

pub(super) fn apply_fixture_args(
    kind: ServiceKind,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    args: &mut Map<String, Value>,
) {
    for param in op.query_params {
        if let Some(value) = fixture_arg_value(kind, op, fixtures, param)
            && (args.contains_key(*param) || should_seed_optional_query(param))
        {
            args.insert((*param).to_string(), value);
        }
    }
}

pub(super) fn should_seed_optional_query(param: &str) -> bool {
    let lower = param.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "seriesid"
            | "movieid"
            | "episodeids"
            | "episodefileids"
            | "itemid"
            | "userid"
            | "parentid"
            | "sectionid"
            | "librarysectionid"
            | "ratingkey"
            | "metadataitemid"
            | "path"
            | "term"
            | "query"
    )
}

fn fixture_arg_value(
    kind: ServiceKind,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    param: &str,
) -> Option<Value> {
    let lower = param.to_ascii_lowercase();
    if lower == "path" {
        return Some(json!(live_root_path(kind)));
    }
    if lower == "term" || lower == "query" || lower == "searchterm" {
        return Some(json!(live_search_term(kind)));
    }
    if lower == "prefs" {
        return Some(json!(["FriendlyName=Yarr Live Plex"]));
    }
    if lower == "imagetype" {
        return Some(json!("Primary"));
    }
    if lower == "imageindex" || lower == "index" || lower == "routeindex" {
        return Some(json!(0));
    }
    if lower == "container" || lower == "format" || lower == "routeformat" {
        return Some(json!("mp4"));
    }
    if lower == "language" {
        return Some(json!("eng"));
    }
    if lower == "width" || lower == "maxwidth" {
        return Some(json!(320));
    }
    if lower == "height" || lower == "maxheight" {
        return Some(json!(180));
    }
    if lower == "year" {
        return Some(json!(2026));
    }
    if lower == "percentplayed" || lower == "unplayedcount" || lower.ends_with("ticks") {
        return Some(json!(0));
    }
    if lower == "seriesid" {
        return fixture_first_id(fixtures, &["/api/v3/series"]);
    }
    if lower == "movieid" {
        return fixture_first_id(fixtures, &["/api/v3/movie"]);
    }
    if lower == "episodeids" {
        return fixture_first_id(fixtures, &["/api/v3/episode"]).map(|id| json!([id]));
    }
    if lower == "episodefileids" {
        return fixture_first_id(fixtures, &["/api/v3/episodefile"]).map(|id| json!([id]));
    }
    if lower == "userid" {
        return fixture_first_id(fixtures, &["/Users", "/users"]);
    }
    if lower == "itemid"
        || lower == "videoid"
        || lower == "routeitemid"
        || lower == "parentid"
        || lower == "artistid"
        || lower == "albumid"
    {
        return fixture_first_id(fixtures, &["/Items", "/library/metadata"]);
    }
    if lower == "mediasourceid" || lower == "routemediasourceid" {
        return fixture_first_media_source_id(fixtures)
            .or_else(|| fixture_first_id(fixtures, &["/Items", "/library/metadata"]));
    }
    if lower == "sectionid" || lower == "librarysectionid" {
        return fixture_first_id(fixtures, &["/library/sections/all"]);
    }
    if lower == "ratingkey" || lower == "metadataitemid" {
        return fixture_first_id(fixtures, &["/library/metadata", "/library/sections/all"]);
    }
    if lower == "id" || lower.ends_with("id") || lower == "ids" {
        let parent = fixture_parent_path(op.path);
        let id = fixture_path_value(fixtures, parent, param)
            .or_else(|| fixture_first_id(fixtures, &[parent]));
        return if lower == "ids" {
            id.map(|value| json!([value]))
        } else {
            id
        };
    }
    if lower.contains("name") {
        let parent = fixture_parent_path(op.path);
        return fixture_path_value(fixtures, parent, param).or_else(|| Some(json!("yarr-live")));
    }
    None
}

fn fixture_first_id(fixtures: &FixtureStore, paths: &[&str]) -> Option<Value> {
    paths.iter().find_map(|path| {
        fixtures
            .values_for(path)
            .and_then(|values| values.first().cloned())
    })
}

fn fixture_first_media_source_id(fixtures: &FixtureStore) -> Option<Value> {
    fixtures.bodies.values().flatten().find_map(|body| {
        body.pointer("/MediaSources/0/Id")
            .or_else(|| body.pointer("/media/0/id"))
            .filter(|value| is_scalar(value))
            .cloned()
    })
}

fn live_root_path(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Sonarr => "/data/media/tv",
        ServiceKind::Radarr => "/data/media/movies",
        ServiceKind::Jellyfin | ServiceKind::Plex => "/data/yarr-live-plex-movies",
        _ => "/tmp",
    }
}

fn live_search_term(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Sonarr => "silo",
        ServiceKind::Radarr => "the matrix",
        ServiceKind::Prowlarr => "ubuntu",
        ServiceKind::Jellyfin | ServiceKind::Plex => "yarr",
        _ => "yarr",
    }
}

#[path = "fixture_args/values.rs"]
mod values;
pub(in crate::live) use values::*;
