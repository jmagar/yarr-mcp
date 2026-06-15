use super::{BASE_PROPERTIES, properties, property_count};
use crate::actions::registry::{ParamType, curated_param_type};
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

/// P2-4: every curated param's advertised JSON `type` (and array `items`) must
/// match the [`ParamType`] declared on its descriptor — so a new non-string param
/// can no longer silently fall through to the `string` fallback under
/// `additionalProperties:false`. Params no command declares a type for fall back
/// to string (the documented default for plain string params).
#[test]
fn curated_param_schema_type_matches_descriptor() {
    let props = properties();
    let obj = props.as_object().expect("properties is an object");

    for param in curated_param_names() {
        // `service` is declared as a base property, not via the curated schema.
        if BASE_PROPERTIES.contains(&param) {
            continue;
        }
        let schema = obj
            .get(param)
            .unwrap_or_else(|| panic!("curated param {param} missing from properties"));

        let expected = curated_param_type(param)
            .unwrap_or(ParamType::String)
            .json_schema_type();

        assert_eq!(
            schema.get("type"),
            expected.get("type"),
            "param `{param}` advertises type {:?} but descriptor implies {:?}",
            schema.get("type"),
            expected.get("type"),
        );
        // Array params must also advertise matching `items`.
        assert_eq!(
            schema.get("items"),
            expected.get("items"),
            "param `{param}` advertises items {:?} but descriptor implies {:?}",
            schema.get("items"),
            expected.get("items"),
        );
    }
}

/// Guard the specific non-string params so a refactor that drops a `ParamType`
/// declaration (reverting to the string fallback) is caught loudly.
#[test]
fn known_typed_params_are_not_strings() {
    let cases = [
        ("ids", ParamType::IntegerArray),
        ("seasons", ParamType::IntegerArray),
        ("title", ParamType::StringArray),
        ("media_id", ParamType::Integer),
        ("take", ParamType::Integer),
        ("skip", ParamType::Integer),
        ("start", ParamType::Integer),
        ("length", ParamType::Integer),
        ("confirm", ParamType::Boolean),
        ("bulk", ParamType::Boolean),
        ("delete_files", ParamType::Boolean),
    ];
    for (param, expected) in cases {
        assert_eq!(
            curated_param_type(param),
            Some(expected),
            "param `{param}` lost its non-string ParamType declaration",
        );
    }
}
