use super::*;

#[test]
fn web_assets_available_is_callable() {
    let _ = web_assets_available();
}

#[test]
fn normalize_asset_path_removes_leading_and_trailing_slashes() {
    assert_eq!(normalize_asset_path("/tools/"), "tools");
    assert_eq!(normalize_asset_path("/api"), "api");
    assert_eq!(normalize_asset_path("/"), "");
}

#[test]
fn asset_candidates_match_next_static_export_paths() {
    assert_eq!(
        asset_candidates("tools"),
        vec![
            "tools".to_string(),
            "tools.html".to_string(),
            "tools/index.html".to_string(),
        ],
    );
}

#[test]
fn trailing_slash_candidates_do_not_include_double_slashes() {
    let path = normalize_asset_path("/tools/");
    let candidates = asset_candidates(path);
    assert_eq!(candidates[2], "tools/index.html");
    assert!(!candidates.iter().any(|candidate| candidate.contains("//")));
}

#[test]
fn root_asset_candidates_do_not_include_empty_or_double_slash_paths() {
    assert_eq!(asset_candidates(""), vec!["index.html".to_string()]);
}

#[test]
fn cache_control_does_not_cache_html_shells() {
    assert_eq!(cache_control_for("index.html"), "no-store");
    assert_eq!(cache_control_for("tools.html"), "no-store");
    assert_eq!(cache_control_for("tools/index.html"), "no-store");
}

#[test]
fn cache_control_only_marks_next_static_assets_immutable() {
    assert_eq!(
        cache_control_for("_next/static/chunks/app-abc123.js"),
        "public, max-age=31536000, immutable"
    );
    assert_eq!(cache_control_for("tools/index.txt"), "public, max-age=3600");
    assert_eq!(cache_control_for("favicon.ico"), "public, max-age=3600");
}

#[test]
fn guess_mime_html() {
    assert_eq!(guess_mime("index.html"), "text/html; charset=utf-8");
}

#[test]
fn guess_mime_css() {
    assert_eq!(guess_mime("styles.css"), "text/css; charset=utf-8");
}

#[test]
fn guess_mime_js() {
    assert_eq!(
        guess_mime("app.js"),
        "application/javascript; charset=utf-8"
    );
}

#[test]
fn guess_mime_mjs() {
    assert_eq!(
        guess_mime("module.mjs"),
        "application/javascript; charset=utf-8"
    );
}

#[test]
fn guess_mime_json() {
    assert_eq!(guess_mime("data.json"), "application/json");
}

#[test]
fn guess_mime_svg() {
    assert_eq!(guess_mime("icon.svg"), "image/svg+xml");
}

#[test]
fn guess_mime_png() {
    assert_eq!(guess_mime("logo.png"), "image/png");
}

#[test]
fn guess_mime_ico() {
    assert_eq!(guess_mime("favicon.ico"), "image/x-icon");
}

#[test]
fn guess_mime_woff2() {
    assert_eq!(guess_mime("font.woff2"), "font/woff2");
}

#[test]
fn guess_mime_webmanifest() {
    assert_eq!(guess_mime("site.webmanifest"), "application/manifest+json");
}

#[test]
fn guess_mime_unknown_falls_back_to_octet_stream() {
    assert_eq!(guess_mime("archive.tar.bz2"), "application/octet-stream");
}

#[test]
fn guess_mime_no_extension_falls_back_to_octet_stream() {
    assert_eq!(guess_mime("Makefile"), "application/octet-stream");
}
