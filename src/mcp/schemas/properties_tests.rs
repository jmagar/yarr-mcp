use super::{properties, property_count, BASE_PROPERTIES};
use crate::actions::{all_action_names, curated_param_names};

#[test]
fn base_properties_are_present() {
    let props = properties();
    let obj = props.as_object().expect("properties is an object");
    for name in BASE_PROPERTIES {
        assert!(obj.contains_key(*name), "missing base property {name}");
    }
}

#[test]
fn action_enum_is_the_full_action_set() {
    let props = properties();
    let enum_values: Vec<&str> = props["action"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(enum_values, all_action_names());
}

#[test]
fn property_set_is_union_of_base_and_curated_params() {
    // With no curated commands (F4 state) the property count equals the base set.
    let expected = BASE_PROPERTIES.len() + curated_param_names().len();
    assert_eq!(property_count(), expected);
}
