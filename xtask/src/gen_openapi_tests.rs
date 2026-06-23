//! Tests for the OpenAPI → operation/type table generator.
//!
//! These exercise the parse/emit core on tiny inline specs so a regression in
//! path-prefixing, param classification, the phantom-path-param skip, name
//! derivation, or the components→TS pass fails CI here rather than silently
//! shipping a broken generated table.

use super::*;
use serde_json::json;

#[test]
fn server_base_path_strips_host_and_keeps_api_prefix() {
    // Host-only URLs (the common Servarr/Jellyfin case) → empty base.
    assert_eq!(server_base_path(&json!({})), "");
    assert_eq!(
        server_base_path(&json!({ "servers": [{ "url": "http://localhost:7878" }] })),
        ""
    );
    assert_eq!(
        server_base_path(&json!({ "servers": [{ "url": "{protocol}://{host}" }] })),
        ""
    );
    // Overseerr-style API prefix is preserved (trailing slash trimmed).
    assert_eq!(
        server_base_path(&json!({ "servers": [{ "url": "{server}/api/v1/" }] })),
        "/api/v1"
    );
}

#[test]
fn extract_operations_prefixes_base_and_classifies_params() {
    let spec = json!({
        "servers": [{ "url": "{server}/api/v1" }],
        "paths": {
            "/movie/{id}": {
                "get": {
                    "operationId": "getMovieById",
                    "parameters": [
                        { "name": "id", "in": "path" },
                        { "name": "extended", "in": "query" }
                    ],
                    "responses": { "200": { "content": {
                        "application/json": { "schema": { "$ref": "#/components/schemas/Movie" } }
                    } } }
                },
                "post": {
                    "operationId": "updateMovie",
                    "parameters": [{ "name": "id", "in": "path" }],
                    "requestBody": { "content": {
                        "application/json": { "schema": { "$ref": "#/components/schemas/Movie" } }
                    } }
                }
            }
        }
    });
    let ops = extract_operations(&spec).unwrap();
    assert_eq!(ops.len(), 2);

    let get = ops.iter().find(|o| o.method == "GET").unwrap();
    // Server base path is prefixed onto the operation path.
    assert_eq!(get.path, "/api/v1/movie/{id}");
    assert_eq!(get.name, "get_movie_by_id");
    assert_eq!(get.path_params, vec!["id".to_string()]);
    assert_eq!(get.query_params, vec!["extended".to_string()]);
    assert!(!get.has_body);
    assert_eq!(get.response_type.as_deref(), Some("Movie"));

    let post = ops.iter().find(|o| o.method == "POST").unwrap();
    assert!(post.has_body);
    assert_eq!(post.request_type.as_deref(), Some("Movie"));
}

#[test]
fn extract_operations_skips_phantom_path_params() {
    // Declared `in:path` param `path` with no `{path}` placeholder (the Servarr SPA
    // catch-all) must be dropped, not emitted as an unaddressable op.
    let spec = json!({
        "paths": {
            "/": {
                "get": {
                    "operationId": "getByPath",
                    "parameters": [{ "name": "path", "in": "path" }],
                    "responses": { "200": {} }
                }
            }
        }
    });
    assert!(extract_operations(&spec).unwrap().is_empty());
}

#[test]
fn extract_operations_derives_name_when_operation_id_missing() {
    let spec = json!({
        "paths": {
            "/api/v3/system/status": { "get": { "responses": { "200": {} } } }
        }
    });
    let ops = extract_operations(&spec).unwrap();
    assert_eq!(ops.len(), 1);
    // `api` and the `v3` version segment are stripped from the derived name.
    assert_eq!(ops[0].name, "get_system_status");
}

#[test]
fn extract_operations_disambiguates_duplicate_names() {
    let spec = json!({
        "paths": {
            "/a": { "get": { "operationId": "ping", "responses": { "200": {} } } },
            "/b": { "get": { "operationId": "ping", "responses": { "200": {} } } }
        }
    });
    let mut names: Vec<String> = extract_operations(&spec)
        .unwrap()
        .into_iter()
        .map(|o| o.name)
        .collect();
    names.sort_unstable();
    assert_eq!(names, vec!["ping".to_string(), "ping_2".to_string()]);
}

#[test]
fn extract_types_renders_interface_and_enum() {
    let spec = json!({
        "components": { "schemas": {
            "Movie": { "type": "object", "properties": {
                "id": { "type": "integer" },
                "title": { "type": "string" }
            } },
            "Status": { "enum": ["pending", "done"] }
        } }
    });
    let types = extract_types(&spec);
    let movie = types.iter().find(|t| t.name == "Movie").unwrap();
    assert!(movie.ts.starts_with("export interface Movie"));
    assert!(movie.ts.contains("title"));
    let status = types.iter().find(|t| t.name == "Status").unwrap();
    assert!(status.ts.contains("export type Status ="));
    assert!(status.ts.contains("\"pending\""));
}

#[test]
fn extract_types_empty_when_no_components() {
    assert!(extract_types(&json!({ "paths": {} })).is_empty());
}
