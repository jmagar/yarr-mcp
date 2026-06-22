//! Discovery catalog tests.

use super::*;
use crate::actions::all_action_names;

#[test]
fn catalog_excludes_codemode_and_covers_the_registry() {
    let cat = build_catalog();
    let names: Vec<&str> = cat.iter().map(|e| e.name).collect();
    assert!(!names.contains(&"codemode"), "codemode must be excluded");
    // Representative generic + curated actions are present.
    assert!(names.contains(&"integrations"));
    assert!(names.contains(&"api_get"));
    assert!(names.contains(&"list"));
    // Covers exactly the deduped registry minus `codemode`.
    let mut expected = all_action_names();
    expected.sort_unstable();
    expected.dedup();
    let expected_len = expected.iter().filter(|n| **n != "codemode").count();
    assert_eq!(cat.len(), expected_len);
}

#[test]
fn catalog_entry_names_are_unique() {
    let cat = build_catalog();
    let mut names: Vec<&str> = cat.iter().map(|e| e.name).collect();
    names.sort_unstable();
    let mut deduped = names.clone();
    deduped.dedup();
    assert_eq!(names, deduped, "catalog entry names must be unique");
}

#[test]
fn catalog_flags_scope_and_destructive() {
    let cat = build_catalog();
    let get = cat.iter().find(|e| e.name == "api_get").unwrap();
    assert_eq!(get.scope, "write"); // api_get requires write scope
    assert!(!get.destructive);

    let del = cat.iter().find(|e| e.name == "api_delete").unwrap();
    assert!(del.destructive, "api_delete is destructive");

    let integrations = cat.iter().find(|e| e.name == "integrations").unwrap();
    assert_eq!(integrations.scope, "read");
    assert_eq!(integrations.kind, "generic");
    assert_eq!(integrations.capability, "infra");
}

#[test]
fn curated_entry_carries_capability_and_description() {
    let cat = build_catalog();
    let list = cat.iter().find(|e| e.name == "list").unwrap();
    assert_eq!(list.kind, "curated");
    assert_ne!(list.capability, "infra");
    assert!(!list.description.is_empty());
    // `list` targets ArrManager kinds.
    assert!(list.allowed_kinds.contains(&"sonarr"));
}

#[test]
fn catalog_json_is_valid_json_array() {
    let json = catalog_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), build_catalog().len());
}
