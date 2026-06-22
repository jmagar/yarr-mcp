//! Tests for the agent-facing Code Mode `.d.ts` generation.

use super::*;

#[test]
fn dts_declares_the_api_surface() {
    let d = codemode_dts();
    assert!(d.contains("declare function callTool"));
    assert!(d.contains("declare const tools:"));
    assert!(d.contains("declare const api:"));
    assert!(d.contains("declare const codemode:"));
    assert!(d.contains("declare function writeArtifact"));
    assert!(d.contains("declare const input:"));
    assert!(d.contains("interface CodeModeResult"));
    // delete is typed `never` (refused mid-script).
    assert!(d.contains("delete(path: string, body?: any): never"));
}

#[test]
fn dts_emits_per_service_response_namespaces() {
    let d = codemode_dts();
    for service in [
        "sonarr",
        "radarr",
        "prowlarr",
        "overseerr",
        "jellyfin",
        "plex",
        "tautulli",
        "sabnzbd",
        "qbittorrent",
        "bazarr",
        "tracearr",
    ] {
        assert!(
            d.contains(&format!("declare namespace {service}")),
            "missing namespace {service}"
        );
    }
    assert!(d.contains("export interface SeriesResource"));
    assert!(d.contains("export interface TorrentInfo"));
}

#[test]
fn dts_converts_optionals_enums_and_arrays() {
    let d = codemode_dts();
    // Optional fields (every model field is Option/defaulted).
    assert!(d.contains("?:"), "expected optional `?:` fields");
    // Enum → string union (QualitySource values).
    assert!(d.contains("\"television\""), "expected enum union members");
    // Numbers (int32 + f64) collapse to `number`.
    assert!(d.contains(": number"));
    // Arrays render as `T[]`.
    assert!(d.contains("[]"));
}
