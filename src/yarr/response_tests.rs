use base64::Engine as _;
use reqwest::StatusCode;

use super::{ResponseMode, binary_response, decode_success};
use crate::config::{ServiceConfig, ServiceKind};

#[test]
fn binary_response_preserves_bytes_and_metadata() {
    let bytes = [0, 255, 42];
    let value = binary_response(
        StatusCode::OK,
        "application/octet-stream",
        Some("attachment; filename=data.bin"),
        &bytes,
    );

    assert_eq!(value["status"], 200);
    assert_eq!(value["mediaType"], "application/octet-stream");
    assert_eq!(
        value["base64"],
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
}

#[test]
fn binary_schema_preserves_text_plain_response_as_base64() {
    let service = ServiceConfig {
        name: "jellyfin".into(),
        kind: ServiceKind::Jellyfin,
        base_url: "http://localhost".into(),
        ..ServiceConfig::default()
    };
    let value = decode_success(
        &service,
        StatusCode::OK,
        Some("text/plain".into()),
        None,
        vec![0xff, 0x00],
        ResponseMode::OpenApi {
            expected_encoding: crate::openapi::BodyEncoding::Binary,
            expected_media_type: "text/plain".into(),
        },
    )
    .unwrap();

    assert_eq!(value["mediaType"], "text/plain");
    assert_eq!(value["base64"], "/wA=");
}
