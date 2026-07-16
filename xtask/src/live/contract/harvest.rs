use super::*;
pub(super) fn harvest_objects(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter(|v| v.is_object())
            .cloned()
            .collect::<Vec<_>>(),
        // A single-resource response almost always carries its own scalar id —
        // treat that as a resource, not a paginated-envelope wrapper, even if it
        // also happens to have an unrelated array-valued field (`tags`, `fields`,
        // `specifications`, ...). Nearly every Servarr resource has one of those,
        // so unconditionally unwrapping the first array field (the old behavior)
        // silently dropped real single-resource create/update responses in favor
        // of whatever unrelated nested array happened to sort first.
        Value::Object(map) if first_id_value(value).is_none() => {
            if let Some(items) = map.values().find_map(Value::as_array) {
                items
                    .iter()
                    .filter(|v| v.is_object())
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                vec![value.clone()]
            }
        }
        Value::Object(_) => vec![value.clone()],
        _ => Vec::new(),
    }
}

pub(super) fn harvest_id_values(value: &Value) -> Vec<Value> {
    match value {
        Value::Object(_) => first_id_value(value).into_iter().collect(),
        _ => Vec::new(),
    }
}

pub(super) fn first_id_value(value: &Value) -> Option<Value> {
    let obj = value.as_object()?;
    for key in ["id", "Id", "ID", "ratingKey", "key", "Guid", "guid"] {
        if let Some(v) = obj.get(key).filter(|v| is_scalar(v)) {
            return Some(v.clone());
        }
    }
    None
}

pub(super) fn dedupe_values(values: &mut Vec<Value>) {
    let mut seen = std::collections::BTreeSet::new();
    values.retain(|value| seen.insert(value.to_string()));
}

pub(super) fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}
