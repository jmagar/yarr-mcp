use super::*;
use serde_json::json;

#[test]
fn label_and_matrix_serialization_preserve_style() {
    assert_eq!(
        serialize_named("id", ParameterStyle::Label, true, &json!([1, 2])).unwrap(),
        vec![("id".to_string(), ".1.2".to_string())]
    );
    assert_eq!(
        serialize_named("id", ParameterStyle::Matrix, false, &json!([1, 2])).unwrap(),
        vec![("id".to_string(), ";id=1,2".to_string())]
    );
}
