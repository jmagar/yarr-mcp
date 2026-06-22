//! Tests for the per-type TypeScript catalog (surfaced via codemode.describe).

use super::*;

#[test]
fn type_entries_are_service_qualified_and_cover_services() {
    let entries = type_entries();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    // Service-qualified so sonarr.X and radarr.X don't collide.
    assert!(names.contains(&"sonarr.SeriesResource"));
    assert!(names.contains(&"qbittorrent.TorrentInfo"));
    assert!(names.contains(&"overseerr.MediaRequestPage"));
    // Nested $defs types are also discoverable (chain describe from a root).
    assert!(names.contains(&"sonarr.Quality"));
}

#[test]
fn each_entry_carries_a_ts_declaration() {
    for e in type_entries() {
        assert_eq!(e.name, format!("{}.{}", e.service, e.type_name));
        assert!(
            e.dts.starts_with("export interface ") || e.dts.starts_with("export type "),
            "{} dts: {}",
            e.name,
            e.dts
        );
    }
}

#[test]
fn converter_handles_optionals_enums_and_arrays() {
    let series = type_entries()
        .into_iter()
        .find(|e| e.name == "sonarr.SeriesResource")
        .unwrap();
    // Every model field is Option/defaulted -> optional in TS.
    assert!(series.dts.contains("?:"));

    // Sonarr's QualitySource is TV-specific (radarr's is movie-specific).
    let source = type_entries()
        .into_iter()
        .find(|e| e.name == "sonarr.QualitySource")
        .expect("sonarr.QualitySource enum present");
    // Enum -> string union.
    assert!(source.dts.contains("\"television\""));
    assert!(source.dts.starts_with("export type QualitySource ="));
}

#[test]
fn type_catalog_json_is_valid() {
    let json = type_catalog_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), type_entries().len());
}
