//! Unit tests for the C2 ArrManager write/intent logic. No live services: the
//! body-shape, id-key, selection, and count-cap contracts are all pure functions
//! tested directly, and the dry-run-mutates-nothing path is proven by asserting
//! the preview is built before any transport call (the stub URL is unreachable,
//! so a mutation attempt would surface a transport error instead of a preview).

use super::{
    MAX_BULK, Selection, command_body_plural, command_body_single, editor_id_key,
    editor_monitor_body, editor_quality_body, guard_count, kind_command_supports_plural_ids,
    refresh_command_name, search_command_name, select_all, select_by_ids, select_by_profile,
    select_by_titles, set_quality_preview,
};
use crate::config::ServiceKind;
use serde_json::json;

// ── editor body shape: seriesIds vs movieIds + qualityProfileId ──────────────────

#[test]
fn editor_id_key_is_series_for_sonarr_and_movie_for_radarr() {
    assert_eq!(editor_id_key(ServiceKind::Sonarr), "seriesIds");
    assert_eq!(editor_id_key(ServiceKind::Radarr), "movieIds");
}

#[test]
fn editor_id_key_is_artist_for_lidarr_and_author_for_readarr() {
    // C3: the v1 kinds derive their editor id key from `resource_noun` exactly
    // like the v3 kinds — `artist`→`artistIds`, `author`→`authorIds`. No special
    // casing: the same `{noun}Ids` rule drives all four ArrManager kinds.
    assert_eq!(editor_id_key(ServiceKind::Lidarr), "artistIds");
    assert_eq!(editor_id_key(ServiceKind::Readarr), "authorIds");
}

#[test]
fn quality_editor_body_uses_artist_ids_for_lidarr() {
    let body = editor_quality_body(ServiceKind::Lidarr, &[1, 2], 3);
    assert_eq!(body["artistIds"], json!([1, 2]));
    assert_eq!(body["qualityProfileId"], json!(3));
    assert!(
        body.get("seriesIds").is_none() && body.get("movieIds").is_none(),
        "lidarr must use artistIds only"
    );
}

#[test]
fn quality_editor_body_uses_author_ids_for_readarr() {
    let body = editor_quality_body(ServiceKind::Readarr, &[5], 6);
    assert_eq!(body["authorIds"], json!([5]));
    assert_eq!(body["qualityProfileId"], json!(6));
    assert!(
        body.get("artistIds").is_none(),
        "readarr must use authorIds only"
    );
}

// ── search/refresh command names are table-driven by resource noun (C3) ───────────

#[test]
fn command_names_follow_resource_noun_across_the_family() {
    // The Servarr command names are NOT uniform: sonarr `SeriesSearch` vs radarr
    // `MoviesSearch` (plural) vs lidarr `ArtistSearch` vs readarr `AuthorSearch`.
    // All four are resolved from the descriptor's resource noun — no hardcoded
    // movie/series-only branch leaks the v1 kinds into the wrong command name.
    assert_eq!(search_command_name(ServiceKind::Sonarr), "SeriesSearch");
    assert_eq!(search_command_name(ServiceKind::Radarr), "MoviesSearch");
    assert_eq!(search_command_name(ServiceKind::Lidarr), "ArtistSearch");
    assert_eq!(search_command_name(ServiceKind::Readarr), "AuthorSearch");

    assert_eq!(refresh_command_name(ServiceKind::Sonarr), "RefreshSeries");
    assert_eq!(refresh_command_name(ServiceKind::Radarr), "RefreshMovie");
    assert_eq!(refresh_command_name(ServiceKind::Lidarr), "RefreshArtist");
    assert_eq!(refresh_command_name(ServiceKind::Readarr), "RefreshAuthor");
}

#[test]
fn command_body_singular_key_for_lidarr_uses_artist_id() {
    // The per-item `/command` id key is also noun-driven: `artistId` for lidarr.
    // Lidarr has NO plural form, so a single id maps to the singular body.
    use super::editor_id_key_singular;
    let key = editor_id_key_singular(ServiceKind::Lidarr);
    assert_eq!(key, "artistId");
    let body = command_body_single("ArtistSearch", &key, Some(9));
    assert_eq!(body["name"], json!("ArtistSearch"));
    assert_eq!(body["artistId"], json!(9));
}

#[test]
fn only_radarr_supports_plural_command_ids() {
    // Fix 6 (*arr FACT): ONLY Radarr's MoviesSearch/RefreshMovie take a plural
    // movieIds batch. Sonarr/Lidarr/Readarr have NO plural command form.
    assert!(kind_command_supports_plural_ids(ServiceKind::Radarr));
    assert!(!kind_command_supports_plural_ids(ServiceKind::Sonarr));
    assert!(!kind_command_supports_plural_ids(ServiceKind::Lidarr));
    assert!(!kind_command_supports_plural_ids(ServiceKind::Readarr));
}

#[test]
fn radarr_plural_command_body_uses_movie_ids_array() {
    // Radarr is the only kind that batches: {name, movieIds:[...]}.
    let body = command_body_plural("MoviesSearch", "movieId", &[1, 2, 3]);
    assert_eq!(body["name"], json!("MoviesSearch"));
    assert_eq!(body["movieIds"], json!([1, 2, 3]));
    assert!(
        body.get("movieId").is_none(),
        "no singular key in plural body"
    );
}

#[test]
fn quality_editor_body_uses_series_ids_for_sonarr() {
    let body = editor_quality_body(ServiceKind::Sonarr, &[1, 2, 3], 4);
    assert_eq!(body["seriesIds"], json!([1, 2, 3]));
    assert_eq!(body["qualityProfileId"], json!(4));
    assert!(
        body.get("movieIds").is_none(),
        "sonarr must not use movieIds"
    );
}

#[test]
fn quality_editor_body_uses_movie_ids_for_radarr() {
    let body = editor_quality_body(ServiceKind::Radarr, &[7], 9);
    assert_eq!(body["movieIds"], json!([7]));
    assert_eq!(body["qualityProfileId"], json!(9));
    assert!(
        body.get("seriesIds").is_none(),
        "radarr must not use seriesIds"
    );
}

#[test]
fn monitor_editor_body_carries_monitored_flag() {
    let on = editor_monitor_body(ServiceKind::Sonarr, &[1], true);
    assert_eq!(on["seriesIds"], json!([1]));
    assert_eq!(on["monitored"], json!(true));
    let off = editor_monitor_body(ServiceKind::Radarr, &[2], false);
    assert_eq!(off["movieIds"], json!([2]));
    assert_eq!(off["monitored"], json!(false));
}

// ── /command body (case-sensitive name + per-item id) ────────────────────────────

#[test]
fn command_body_single_id_uses_singular_key() {
    // Sonarr has no plural form: a single id is the singular {name, seriesId}.
    let body = command_body_single("SeriesSearch", "seriesId", Some(5));
    assert_eq!(body["name"], json!("SeriesSearch"));
    assert_eq!(body["seriesId"], json!(5));
}

#[test]
fn command_body_no_ids_is_name_only() {
    let body = command_body_single("RefreshSeries", "seriesId", None);
    assert_eq!(body["name"], json!("RefreshSeries"));
    assert!(body.get("seriesId").is_none());
    assert!(body.get("seriesIds").is_none());
}

// ── count cap (S3/AN-4) ──────────────────────────────────────────────────────────

#[test]
fn count_cap_allows_up_to_max() {
    assert!(guard_count(MAX_BULK, false).is_ok());
    assert!(guard_count(1, false).is_ok());
    assert!(guard_count(0, false).is_ok());
}

#[test]
fn count_cap_rejects_over_max_without_bulk_override() {
    let err = guard_count(MAX_BULK + 1, false).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("refusing to act on"), "{msg}");
    assert!(msg.contains("bulk=true"), "{msg}");
}

#[test]
fn count_cap_allows_over_max_with_bulk_override() {
    assert!(guard_count(MAX_BULK + 1000, true).is_ok());
}

// ── selection: name→id resolution drives the body via picked ids ─────────────────

fn rows() -> Vec<serde_json::Value> {
    vec![
        json!({ "id": 1, "title": "Alpha", "qualityProfileId": 6 }),
        json!({ "id": 2, "title": "Beta", "qualityProfileId": 6 }),
        json!({ "id": 3, "title": "Gamma", "qualityProfileId": 4 }),
    ]
}

#[test]
fn select_by_profile_picks_only_items_on_that_profile() {
    let sel = select_by_profile(&rows(), 6);
    assert_eq!(sel.ids, vec![1, 2]);
    // The picked ids are exactly what the editor body would carry.
    let body = editor_quality_body(ServiceKind::Sonarr, &sel.ids, 4);
    assert_eq!(body["seriesIds"], json!([1, 2]));
}

#[test]
fn select_by_titles_resolves_ids_and_errors_on_miss() {
    let sel = select_by_titles(&rows(), &["beta".into(), "Alpha".into()]).unwrap();
    assert_eq!(sel.ids, vec![2, 1]);
    let err = select_by_titles(&rows(), &["Nope".into()]).unwrap_err();
    assert!(err.to_string().contains("Nope"), "{err}");
}

#[test]
fn select_all_takes_every_id() {
    let sel = select_all(&rows());
    assert_eq!(sel.ids, vec![1, 2, 3]);
}

#[test]
fn select_by_ids_preserves_requested_order() {
    let sel = select_by_ids(&rows(), &[3, 1]).unwrap();
    assert_eq!(sel.ids, vec![3, 1]);
    // Matched rows carry their real titles, never empty ghosts.
    assert_eq!(sel.titles, vec!["Gamma".to_string(), "Alpha".to_string()]);
}

#[test]
fn select_by_ids_errors_on_unknown_id_instead_of_ghosting() {
    // Fix 2: an id with no matching row must surface a teaching error, not push an
    // empty-title ghost row into the selection.
    let err = select_by_ids(&rows(), &[1, 999999]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no items found for ids"), "{msg}");
    assert!(msg.contains("999999"), "{msg}");
}

// ── dry-run mutates nothing: preview is a pure structure, no transport call ───────

#[test]
fn set_quality_preview_is_structured_and_mutates_nothing() {
    // The dry-run path (confirm absent) returns this pure preview WITHOUT issuing
    // any PUT — proven structurally: `set_quality_preview` takes no `self`/client,
    // so it cannot mutate. The preview carries the S3/AN-4 contract fields.
    let selection = Selection {
        ids: vec![1, 2],
        titles: vec!["Alpha".into(), "Beta".into()],
    };
    let preview = set_quality_preview(
        "sonarr",
        Some("Ultra-HD"),
        Some(6),
        "HD-1080p",
        4,
        &selection,
    );
    assert_eq!(preview["would_do"], json!("set_quality"));
    assert_eq!(
        preview["target_profile"],
        json!({ "name": "HD-1080p", "id": 4 })
    );
    assert_eq!(
        preview["from_profile"],
        json!({ "name": "Ultra-HD", "id": 6 })
    );
    assert_eq!(preview["count"], json!(2));
    assert_eq!(preview["sample_titles"], json!(["Alpha", "Beta"]));
    assert_eq!(preview["confirm_required"], json!(true));
    // A preview must never contain the apply summary keys.
    assert!(
        preview.get("changed").is_none(),
        "preview must not report changes"
    );
}

// ── editor apply summary: upstream-confirmed count (Fix 1) ───────────────────────

#[test]
fn editor_apply_summary_reports_upstream_count_when_array() {
    use super::editor_apply_summary;
    // The /editor endpoint echoes the updated resource array; `changed` reflects
    // its length, `attempted` the selection size, and they reconcile.
    let response = json!([{ "id": 1 }, { "id": 2 }]);
    let out = editor_apply_summary(&response, 3, json!({ "to": "HD-1080p" }));
    assert_eq!(out["changed"], json!(2));
    assert_eq!(out["attempted"], json!(3));
    assert_eq!(out["confirmed"], json!(true));
    assert_eq!(out["to"], json!("HD-1080p"));
}

#[test]
fn editor_apply_summary_falls_back_and_marks_unconfirmed_when_not_array() {
    use super::editor_apply_summary;
    // A non-array response cannot confirm the count: fall back to attempted and
    // mark confirmed=false.
    let response = json!({ "ok": true });
    let out = editor_apply_summary(&response, 5, json!({ "monitored": true }));
    assert_eq!(out["changed"], json!(5));
    assert_eq!(out["attempted"], json!(5));
    assert_eq!(out["confirmed"], json!(false));
    assert_eq!(out["monitored"], json!(true));
}

#[test]
fn value_shape_and_preview_describe_unexpected_shapes() {
    use super::{value_preview, value_shape};
    assert_eq!(value_shape(&json!({ "a": 1 })), "an object");
    assert_eq!(value_shape(&json!("hi")), "a string");
    assert_eq!(value_shape(&json!(null)), "null");
    let preview = value_preview(&json!({ "error": "boom" }));
    assert!(preview.contains("boom"), "{preview}");
}

#[test]
fn set_quality_preview_omits_from_when_absent() {
    let selection = Selection {
        ids: vec![1],
        titles: vec!["Alpha".into()],
    };
    let preview = set_quality_preview("radarr", None, None, "HD-1080p", 4, &selection);
    assert_eq!(preview["from_profile"], json!(null));
}
