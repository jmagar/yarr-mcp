//! Deterministic operation and Rust identifier naming.

use std::collections::BTreeMap;

pub(super) fn derive_name(method: &str, path: &str, path_params: &[String]) -> String {
    let mut parts = vec![method.to_string()];
    for segment in path.split('/') {
        if segment.is_empty() || segment == "api" || segment.starts_with('{') {
            continue;
        }
        if segment.len() >= 2
            && segment.starts_with('v')
            && segment[1..]
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            continue;
        }
        parts.push(to_snake(segment));
    }
    if !path_params.is_empty() {
        parts.push("by".to_string());
        parts.extend(path_params.iter().map(|parameter| to_snake(parameter)));
    }
    let cleaned = sanitize_ident(&parts.join("_"));
    if cleaned.is_empty() {
        method.to_string()
    } else {
        cleaned
    }
}

pub(super) fn to_snake(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 4);
    let characters: Vec<char> = value.chars().collect();
    for (index, &character) in characters.iter().enumerate() {
        if character.is_ascii_uppercase() {
            let previous_lower = index > 0 && characters[index - 1].is_ascii_lowercase();
            let previous_digit = index > 0 && characters[index - 1].is_ascii_digit();
            let next_lower =
                index + 1 < characters.len() && characters[index + 1].is_ascii_lowercase();
            if (previous_lower || previous_digit || (index > 0 && next_lower))
                && !output.ends_with('_')
            {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else if character.is_ascii_alphanumeric() {
            output.push(character);
        } else if !output.ends_with('_') {
            output.push('_');
        }
    }
    output.trim_matches('_').to_string()
}

fn sanitize_ident(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
        } else if !output.ends_with('_') {
            output.push('_');
        }
    }
    output.trim_matches('_').to_string()
}

pub(super) fn unique(used: &mut BTreeMap<String, u32>, base: String) -> String {
    let base = if base.is_empty() {
        "op".to_string()
    } else {
        base
    };
    let count = used.entry(base.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        base
    } else {
        format!("{base}_{count}")
    }
}
