use super::*;
pub(super) fn relax_for_client(v: &mut Value) {
    match v {
        Value::Object(map) => {
            if map.get("additionalProperties") == Some(&Value::Bool(false)) {
                map.remove("additionalProperties");
            }
            let nullable = map.remove("nullable").and_then(|n| n.as_bool()) == Some(true);
            for child in map.values_mut() {
                relax_for_client(child);
            }
            if nullable {
                match map.get("type").cloned() {
                    Some(Value::String(t)) => {
                        map.insert("type".into(), json!([t, "null"]));
                    }
                    Some(Value::Array(mut arr)) => {
                        if !arr.iter().any(|x| x == "null") {
                            arr.push(json!("null"));
                        }
                        map.insert("type".into(), Value::Array(arr));
                    }
                    // No explicit type (a `$ref` or composition): wrap the whole
                    // schema in `anyOf` so `null` is accepted alongside it.
                    _ => {
                        let inner = std::mem::take(map);
                        map.insert("anyOf".into(), json!([inner, { "type": "null" }]));
                    }
                }
            }
        }
        Value::Array(arr) => {
            for e in arr.iter_mut() {
                relax_for_client(e);
            }
        }
        _ => {}
    }
}

pub(super) fn relax_known_server_drifts(schemas: &mut Value) {
    let Some(map) = schemas.as_object_mut() else {
        return;
    };
    if let Some(http_uri) = map.get_mut("HttpUri") {
        let object_shape = http_uri.clone();
        *http_uri = json!({
            "anyOf": [
                object_shape,
                { "type": "string" },
                { "type": "null" }
            ]
        });
    }
    if map.contains_key("NotificationAgentTypes")
        && map.contains_key("UserSettingsNotifications")
        && let Some(username) = map
            .get_mut("User")
            .and_then(|schema| schema.get_mut("properties"))
            .and_then(|properties| properties.get_mut("username"))
    {
        allow_null_for_schema(username);
    }
    if map.contains_key("NotificationAgentTypes") {
        remove_required(map, "User", "email");
        for field in [
            "hostname",
            "port",
            "apiKey",
            "useSsl",
            "activeProfileName",
            "minimumAvailability",
        ] {
            remove_required(map, "RadarrSettings", field);
        }
        for field in [
            "hostname",
            "port",
            "apiKey",
            "useSsl",
            "activeProfileName",
            "enableSeasonFolders",
        ] {
            remove_required(map, "SonarrSettings", field);
        }
        if let Some(properties) = map
            .get_mut("User")
            .and_then(|schema| schema.get_mut("properties"))
            .and_then(Value::as_object_mut)
        {
            for field in ["email", "username", "plexToken", "plexUsername", "avatar"] {
                if let Some(schema) = properties.get_mut(field) {
                    allow_null_for_schema(schema);
                }
            }
        }
        if let Some(interval) = map
            .get_mut("Job")
            .and_then(|schema| schema.get_mut("properties"))
            .and_then(|properties| properties.get_mut("interval"))
            .and_then(Value::as_object_mut)
        {
            interval.remove("enum");
        }
        for schema_name in ["MovieDetails", "TvDetails"] {
            if let Some(watch_providers) = map
                .get_mut(schema_name)
                .and_then(|schema| schema.get_mut("properties"))
                .and_then(|properties| properties.get_mut("watchProviders"))
            {
                *watch_providers = json!({});
            }
        }
        if let Some(properties) = map
            .get_mut("PersonDetails")
            .and_then(|schema| schema.get_mut("properties"))
            .and_then(Value::as_object_mut)
        {
            for field in [
                "deathday",
                "knownForDepartment",
                "biography",
                "placeOfBirth",
                "profilePath",
                "imdbId",
                "homepage",
            ] {
                if let Some(schema) = properties.get_mut(field) {
                    allow_null_for_schema(schema);
                }
            }
            for field in ["gender", "popularity"] {
                if let Some(schema) = properties.get_mut(field) {
                    allow_number_or_null_for_schema(schema);
                }
            }
        }
        if let Some(ratings) = map
            .get_mut("SonarrSeries")
            .and_then(|schema| schema.get_mut("properties"))
            .and_then(|properties| properties.get_mut("ratings"))
        {
            allow_any_object_or_null_for_schema(ratings);
        }
    }
    if map.contains_key("MediaContainerWithMetadata") {
        clear_required(map, "Metadata");
        clear_required(map, "Part");
        clear_required(map, "Stream");
        if let Some(stream_identifier) = map
            .get_mut("Stream")
            .and_then(|schema| schema.get_mut("properties"))
            .and_then(|properties| properties.get_mut("streamIdentifier"))
        {
            allow_string_or_integer_for_schema(stream_identifier);
        }
        remove_required(map, "MediaContainerWithMetadata", "key");
        remove_required(map, "MediaContainerWithNestedMetadata", "key");
        clear_required(map, "PlexDevice");
        clear_required(map, "UserPlexAccount");
    }
    if let Some(timer) = map.get_mut("TimerInfoDto") {
        allow_null_for_schema(timer);
    }
    if map.contains_key("IActionResult") {
        map.insert("IActionResult".into(), json!({}));
    }
}

pub(super) fn allow_any_object_or_null_for_schema(schema: &mut Value) {
    let original = schema.clone();
    *schema = json!({
        "anyOf": [
            original,
            { "type": "object" },
            { "type": "null" }
        ]
    });
}

pub(super) fn allow_number_or_null_for_schema(schema: &mut Value) {
    let original = schema.clone();
    *schema = json!({
        "anyOf": [
            original,
            { "type": "number" },
            { "type": "null" }
        ]
    });
}

pub(super) fn allow_string_or_integer_for_schema(schema: &mut Value) {
    let original = schema.clone();
    *schema = json!({
        "anyOf": [
            original,
            { "type": "string" },
            { "type": "integer" }
        ]
    });
}

pub(super) fn allow_null_for_schema(schema: &mut Value) {
    let Value::Object(map) = schema else {
        return;
    };
    match map.get("type").cloned() {
        Some(Value::String(ty)) => {
            map.insert("type".into(), json!([ty, "null"]));
        }
        Some(Value::Array(mut types)) => {
            if !types.iter().any(|ty| ty == "null") {
                types.push(json!("null"));
            }
            map.insert("type".into(), Value::Array(types));
        }
        _ => {
            let inner = std::mem::take(map);
            map.insert("anyOf".into(), json!([inner, { "type": "null" }]));
        }
    }
}

pub(super) fn remove_required(map: &mut Map<String, Value>, schema_name: &str, field: &str) {
    let Some(required) = map
        .get_mut(schema_name)
        .and_then(|schema| schema.get_mut("required"))
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    required.retain(|item| item.as_str() != Some(field));
}

pub(super) fn clear_required(map: &mut Map<String, Value>, schema_name: &str) {
    if let Some(schema) = map.get_mut(schema_name).and_then(Value::as_object_mut) {
        schema.remove("required");
    }
}
