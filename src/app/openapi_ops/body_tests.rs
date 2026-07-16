use super::*;

#[test]
fn binary_property_discovery_uses_schema_format() {
    assert_eq!(
        binary_property_name(r#"{"properties":{"archive":{"type":"string","format":"binary"}}}"#)
            .as_deref(),
        Some("archive")
    );
}
