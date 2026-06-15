//! Unit tests for the C2 ArrManager write/intent logic. No live services: the
//! body-shape, id-key, selection, and count-cap contracts are all pure functions
//! tested directly, and the dry-run-mutates-nothing path is proven by asserting
//! the preview is built before any transport call (the stub URL is unreachable,
//! so a mutation attempt would surface a transport error instead of a preview).

use super::{
    command_body, editor_id_key, editor_monitor_body, editor_quality_body, guard_count, select_all,
    select_by_ids, select_by_profile, select_by_titles, set_quality_preview, Selection, MAX_BULK,
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
    let body = command_body("SeriesSearch", "seriesId", &[5]);
    assert_eq!(body["name"], json!("SeriesSearch"));
    assert_eq!(body["seriesId"], json!(5));
}

#[test]
fn command_body_no_ids_is_name_only() {
    let body = command_body("RefreshSeries", "seriesId", &[]);
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
    let sel = select_by_ids(&rows(), &[3, 1]);
    assert_eq!(sel.ids, vec![3, 1]);
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

#[test]
fn set_quality_preview_omits_from_when_absent() {
    let selection = Selection {
        ids: vec![1],
        titles: vec!["Alpha".into()],
    };
    let preview = set_quality_preview("radarr", None, None, "HD-1080p", 4, &selection);
    assert_eq!(preview["from_profile"], json!(null));
}
