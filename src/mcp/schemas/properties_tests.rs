use super::{BASE_PROPERTIES, properties, property_count};
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
    // The property set is the de-duplicated union of base props and curated
    // params. C1's only curated param is `service`, which is already a base prop,
    // so the count stays at the base size — assert via the actual union so this
    // holds as later beads add genuinely new params.
    let mut union: Vec<&str> = BASE_PROPERTIES.to_vec();
    for p in curated_param_names() {
        if !union.contains(&p) {
            union.push(p);
        }
    }
    assert_eq!(property_count(), union.len());
}
