use super::*;

#[test]
fn multipart_file_payload_preserves_field_metadata_and_bytes() {
    let field = MultipartField::File {
        name: "archive".into(),
        file_name: "backup.zip".into(),
        media_type: "application/zip".into(),
        bytes: vec![0, 1, 2],
    };

    let MultipartField::File {
        name,
        file_name,
        media_type,
        bytes,
    } = field
    else {
        panic!("expected file field");
    };
    assert_eq!(name, "archive");
    assert_eq!(file_name, "backup.zip");
    assert_eq!(media_type, "application/zip");
    assert_eq!(bytes, [0, 1, 2]);
}
