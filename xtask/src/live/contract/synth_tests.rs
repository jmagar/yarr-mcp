//! Pure-logic tests for the contract-harness spec helpers: the OpenAPI-3.0 →
//! JSON-Schema relaxation, response validation, schema sampling, and arg synthesis.
//! These run without the live stack (no `Spec::load` file IO — `Spec` is built
//! inline).

use super::*;
use serde_json::{Map, json};
use std::collections::BTreeMap;

fn spec_with_components(components: Value) -> Spec {
    Spec {
        doc: json!({ "components": { "schemas": components } }),
        ops: BTreeMap::new(),
    }
}

// --- relax_for_client (OpenAPI nullable / additionalProperties → JSON Schema) ---

#[test]
fn relax_makes_nullable_string_accept_null() {
    let mut s = json!({ "type": "string", "nullable": true });
    relax_for_client(&mut s);
    assert_eq!(s["type"], json!(["string", "null"]));
    assert!(s.get("nullable").is_none(), "nullable marker removed");
}

#[test]
fn relax_appends_null_to_a_type_array_without_duplicating() {
    let mut s = json!({ "type": ["string", "null"], "nullable": true });
    relax_for_client(&mut s);
    assert_eq!(s["type"], json!(["string", "null"]));
}

#[test]
fn relax_drops_additional_properties_false() {
    let mut s = json!({ "type": "object", "additionalProperties": false, "properties": {} });
    relax_for_client(&mut s);
    assert!(s.get("additionalProperties").is_none());
}

#[test]
fn relax_wraps_a_typeless_nullable_ref_in_anyof_with_null() {
    let mut s = json!({ "$ref": "#/components/schemas/Foo", "nullable": true });
    relax_for_client(&mut s);
    let any = s["anyOf"].as_array().expect("wrapped in anyOf");
    assert_eq!(any.len(), 2);
    assert!(any.iter().any(|x| x["type"] == json!("null")));
    assert!(any.iter().any(|x| x.get("$ref").is_some()));
}

#[test]
fn relax_recurses_into_nested_properties_and_arrays() {
    let mut s = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string", "nullable": true },
            "tags": { "type": "array", "items": { "type": "string", "nullable": true } }
        }
    });
    relax_for_client(&mut s);
    assert_eq!(s["properties"]["name"]["type"], json!(["string", "null"]));
    assert_eq!(
        s["properties"]["tags"]["items"]["type"],
        json!(["string", "null"])
    );
}

// --- validate_response (nullable + array-element + extra-field tolerance) ---

#[test]
fn validate_accepts_null_on_a_nullable_field() {
    let spec = spec_with_components(json!({
        "User": { "type": "object", "properties": { "name": { "type": "string", "nullable": true } } }
    }));
    assert!(
        spec.validate_response("User", &json!({ "name": null }))
            .is_ok()
    );
}

#[test]
fn validate_still_rejects_null_on_a_non_nullable_field() {
    let spec = spec_with_components(json!({
        "User": { "type": "object", "properties": { "name": { "type": "string" } } }
    }));
    assert!(
        spec.validate_response("User", &json!({ "name": null }))
            .is_err()
    );
}

#[test]
fn validate_is_array_aware_and_empty_array_passes() {
    let spec = spec_with_components(json!({
        "Item": { "type": "object", "properties": { "id": { "type": "integer" } } }
    }));
    assert!(spec.validate_response("Item", &json!([])).is_ok());
    assert!(
        spec.validate_response("Item", &json!([{ "id": 1 }, { "id": 2 }]))
            .is_ok()
    );
    assert!(
        spec.validate_response("Item", &json!([{ "id": "x" }]))
            .is_err()
    );
}

#[test]
fn validate_tolerates_extra_server_fields_on_a_closed_schema() {
    let spec = spec_with_components(json!({
        "Closed": {
            "type": "object",
            "additionalProperties": false,
            "properties": { "a": { "type": "integer" } }
        }
    }));
    assert!(
        spec.validate_response("Closed", &json!({ "a": 1, "extra": true }))
            .is_ok()
    );
}

#[test]
fn validate_accepts_servarr_http_uri_string_drift() {
    let spec = spec_with_components(json!({
        "Health": {
            "type": "object",
            "properties": {
                "wikiUrl": { "$ref": "#/components/schemas/HttpUri" }
            }
        },
        "HttpUri": {
            "type": "object",
            "properties": {
                "fullUri": { "type": "string", "nullable": true }
            }
        }
    }));
    assert!(
        spec.validate_response(
            "Health",
            &json!({ "wikiUrl": "https://wiki.servarr.com/sonarr/system" })
        )
        .is_ok()
    );
}

#[test]
fn validate_accepts_overseerr_plex_user_without_local_username() {
    let spec = spec_with_components(json!({
        "User": {
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "email": { "type": "string" },
                "username": { "type": "string" },
                "createdAt": { "type": "string" },
                "updatedAt": { "type": "string" }
            },
            "required": ["id", "email", "createdAt", "updatedAt"]
        },
        "NotificationAgentTypes": { "type": "number" },
        "UserSettingsNotifications": {
            "type": "object",
            "properties": {
                "notificationTypes": { "$ref": "#/components/schemas/NotificationAgentTypes" }
            }
        }
    }));
    assert!(
        spec.validate_response(
            "User",
            &json!({
                "id": 1,
                "email": "jmagar@example.com",
                "username": null,
                "createdAt": "2022-09-03T02:33:50.000Z",
                "updatedAt": "2026-06-21T02:37:34.000Z"
            })
        )
        .is_ok()
    );
}

#[test]
fn validate_accepts_overseerr_settings_and_job_drift() {
    let spec = spec_with_components(json!({
        "NotificationAgentTypes": { "type": "number" },
        "UserSettingsNotifications": { "type": "object" },
        "RadarrSettings": {
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "hostname": { "type": "string" },
                "port": { "type": "integer" }
            },
            "required": ["name", "hostname", "port"]
        },
        "SonarrSettings": {
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "hostname": { "type": "string" },
                "port": { "type": "integer" }
            },
            "required": ["name", "hostname", "port"]
        },
        "Job": {
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "interval": { "type": "string", "enum": ["short", "long", "fixed"] }
            }
        }
    }));
    assert!(
        spec.validate_response("RadarrSettings", &json!({ "name": "radarr" }))
            .is_ok()
    );
    assert!(
        spec.validate_response("SonarrSettings", &json!({ "name": "sonarr" }))
            .is_ok()
    );
    assert!(
        spec.validate_response("Job", &json!({ "id": "plex-sync", "interval": "minutes" }))
            .is_ok()
    );
}

#[test]
fn validate_accepts_overseerr_user_without_email() {
    let spec = spec_with_components(json!({
        "NotificationAgentTypes": { "type": "number" },
        "UserSettingsNotifications": { "type": "object" },
        "User": {
            "type": "object",
            "properties": {
                "id": { "type": "integer" },
                "email": { "type": "string" },
                "createdAt": { "type": "string" },
                "updatedAt": { "type": "string" }
            },
            "required": ["id", "email", "createdAt", "updatedAt"]
        }
    }));
    assert!(
        spec.validate_response(
            "User",
            &json!({
                "id": 2,
                "email": null,
                "createdAt": "2022-09-03T02:33:50.000Z",
                "updatedAt": "2026-06-21T02:37:34.000Z"
            })
        )
        .is_ok()
    );
}

// --- sample / sample_depth (request-body synthesis) ---

#[test]
fn sample_object_populates_props_but_skips_readonly_and_top_level_id() {
    let spec = spec_with_components(json!({}));
    let schema = json!({
        "type": "object",
        "properties": {
            "id": { "type": "integer" },
            "name": { "type": "string" },
            "ro": { "type": "string", "readOnly": true }
        }
    });
    let obj = spec.sample(&schema).unwrap();
    let obj = obj.as_object().unwrap();
    assert!(
        !obj.contains_key("id"),
        "top-level server-assigned id excluded"
    );
    assert!(!obj.contains_key("ro"), "readOnly field excluded");
    assert!(obj.contains_key("name"));
}

#[test]
fn sample_keeps_nested_id_but_drops_only_the_top_level_one() {
    let spec = spec_with_components(json!({}));
    let schema = json!({
        "type": "object",
        "properties": {
            "id": { "type": "integer" },
            "child": { "type": "object", "properties": { "id": { "type": "integer" } } }
        }
    });
    let obj = spec.sample(&schema).unwrap();
    assert!(obj.get("id").is_none());
    assert_eq!(obj["child"]["id"], json!(1), "nested id kept");
}

#[test]
fn sample_prefers_example_then_enum() {
    let spec = spec_with_components(json!({}));
    assert_eq!(
        spec.sample(&json!({ "type": "string", "example": "ex" }))
            .unwrap(),
        json!("ex")
    );
    assert_eq!(
        spec.sample(&json!({ "enum": ["a", "b"] })).unwrap(),
        json!("a")
    );
}

// --- build_args (required query params + body) ---

#[test]
fn build_args_synthesizes_required_query_and_body() {
    let mut ops = BTreeMap::new();
    ops.insert(
        ("POST".to_string(), "/x".to_string()),
        json!({
            "parameters": [
                { "name": "q", "in": "query", "required": true, "schema": { "type": "string" } },
                { "name": "opt", "in": "query", "required": false, "schema": { "type": "string" } }
            ],
            "requestBody": { "content": { "application/json": {
                "schema": { "type": "object", "properties": { "title": { "type": "string" } } }
            } } }
        }),
    );
    let spec = Spec {
        doc: json!({ "components": { "schemas": {} } }),
        ops,
    };
    let args = spec.build_args("POST", "/x", &Map::new()).unwrap();
    assert!(args.contains_key("q"), "required query param synthesized");
    assert!(!args.contains_key("opt"), "optional query param skipped");
    assert!(args.contains_key("body"));
    assert_eq!(args["body"]["title"], json!("x"));
}

#[test]
fn build_args_reads_multipart_fixture_and_submits_base64_filename() {
    let mut ops = BTreeMap::new();
    ops.insert(
        ("POST".to_string(), "/upload".to_string()),
        json!({
            "requestBody": { "content": { "multipart/form-data": {
                "schema": { "type": "object", "properties": {
                    "archive": { "type": "string", "format": "binary" }
                } }
            } } }
        }),
    );
    let spec = Spec {
        doc: json!({}),
        ops,
    };
    let fixture = std::path::Path::new("target/live-full/tmp/synth-test-fixture.zip");
    std::fs::create_dir_all(fixture.parent().unwrap()).unwrap();
    std::fs::write(fixture, [0_u8, 1, 2, 255]).unwrap();
    let args = spec
        .build_args_with_multipart_fixture("POST", "/upload", &Map::new(), fixture)
        .unwrap();
    assert_eq!(args["multipartField"], "archive");
    assert_eq!(args["fileName"], "synth-test-fixture.zip");
    assert_eq!(args["multipartFileBase64"], "AAEC/w==");
    std::fs::remove_file(fixture).unwrap();
}
