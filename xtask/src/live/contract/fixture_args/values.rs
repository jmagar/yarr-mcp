use super::*;
pub(in crate::live) fn fixture_parent_path(path: &str) -> &str {
    let parent = path.split_once("/{").map(|(a, _)| a).unwrap_or(path);
    for suffix in [
        "/action", "/test", "/testall", "/failed", "/grab", "/reorder", "/refresh",
    ] {
        if let Some(stripped) = parent.strip_suffix(suffix) {
            return stripped;
        }
    }
    parent
}

pub(in crate::live) fn fixture_path_value(
    fixtures: &FixtureStore,
    parent: &str,
    param: &str,
) -> Option<Value> {
    let lower = param.to_ascii_lowercase();
    if lower == "index" || lower == "newindex" {
        return Some(json!(0));
    }
    let body = fixtures.body_for(parent);
    if lower.contains("name")
        && let Some(value) = body.and_then(|b| field_value(b, &["name", "Name", "title", "Title"]))
    {
        return Some(value);
    }
    body.and_then(|b| {
        field_value(
            b,
            &[param, "id", "Id", "ID", "ratingKey", "key", "Guid", "guid"],
        )
    })
    .or_else(|| {
        fixtures
            .values_for(parent)
            .and_then(|values| values.first().cloned())
    })
    .or_else(|| {
        fixture_parent_aliases(parent).iter().find_map(|alias| {
            fixtures
                .body_for(alias)
                .and_then(|b| {
                    field_value(
                        b,
                        &[param, "id", "Id", "ID", "ratingKey", "key", "Guid", "guid"],
                    )
                })
                .or_else(|| {
                    fixtures
                        .values_for(alias)
                        .and_then(|values| values.first().cloned())
                })
        })
    })
}

pub(super) fn field_value(value: &Value, keys: &[&str]) -> Option<Value> {
    let obj = value.as_object()?;
    keys.iter()
        .find_map(|key| obj.get(*key).filter(|v| is_scalar(v)).cloned())
}

pub(in crate::live) fn fixture_body_for_op<'a>(
    fixtures: &'a FixtureStore,
    op: &OperationSpec,
) -> Option<&'a Value> {
    let parent = fixture_parent_path(op.path);
    fixtures
        .body_for(op.path)
        .or_else(|| fixtures.body_for(parent))
        .or_else(|| {
            fixture_parent_aliases(parent)
                .iter()
                .find_map(|alias| fixtures.body_for(alias))
        })
        .or_else(|| {
            let leaf = parent.rsplit('/').next().unwrap_or(parent);
            fixtures
                .bodies
                .iter()
                .find(|(path, _)| path.ends_with(&format!("/{leaf}")))
                .and_then(|(_, bodies)| bodies.first())
        })
}

pub(super) fn fixture_parent_aliases(parent: &str) -> &'static [&'static str] {
    match parent {
        // Jellyfin exposes media operations through type-specific routes, but the
        // broad `/Items` collection is the reliable source of item ids in a small
        // fixture library.
        "/Videos" | "/Audio" | "/UserItems" | "/Shows" => &["/Items"],
        "/Artists" | "/Persons" | "/Studios" | "/MusicGenres" => &["/Items"],
        // Plex section and metadata ids are frequently nested under collection
        // endpoints rather than exposed by the exact templated parent.
        "/hubs/sections" => &["/library/sections/all"],
        "/hubs/metadata" => &["/library/metadata"],
        _ => &[],
    }
}
