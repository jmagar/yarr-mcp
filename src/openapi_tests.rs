//! Runtime tests for the generated-operation registry + scalar rendering.

use super::*;
use serde_json::json;

#[test]
fn scalar_rendering_covers_string_number_bool_only() {
    assert_eq!(scalar_to_string(&json!("x")).as_deref(), Some("x"));
    assert_eq!(scalar_to_string(&json!(42)).as_deref(), Some("42"));
    assert_eq!(scalar_to_string(&json!(true)).as_deref(), Some("true"));
    // Non-scalars are rejected so we never send `[object Object]`.
    assert!(scalar_to_string(&json!({"a": 1})).is_none());
    assert!(scalar_to_string(&json!([1, 2])).is_none());
    assert!(scalar_to_string(&json!(null)).is_none());
}

#[test]
fn find_operation_resolves_known_and_rejects_unknown() {
    // Sonarr is generated; a known op resolves and an unknown one does not.
    assert!(find_operation(ServiceKind::Sonarr, "get_system_status").is_some());
    assert!(find_operation(ServiceKind::Sonarr, "nope_not_real").is_none());
    // A doc-based kind has no generated operations.
    assert!(operations_for_kind(ServiceKind::Tautulli).is_empty());
    assert!(!is_generated(ServiceKind::Tautulli));
}

/// Table-invariant guard over EVERY generated operation across all 6 spec-backed
/// kinds. This enforces, at test time, the contracts the `OperationSpec` doc
/// comments describe but the (generated-data) types don't structurally guarantee —
/// so a regeneration that emits a bad method, a path/param mismatch, or a dangling
/// type reference fails CI instead of surfacing at request time.
#[test]
fn every_generated_operation_is_well_formed() {
    const KINDS: &[ServiceKind] = &[
        ServiceKind::Sonarr,
        ServiceKind::Radarr,
        ServiceKind::Prowlarr,
        ServiceKind::Overseerr,
        ServiceKind::Jellyfin,
        ServiceKind::Plex,
    ];
    for &kind in KINDS {
        let ops = operations_for_kind(kind);
        assert!(
            !ops.is_empty(),
            "{} should have generated ops",
            kind.as_str()
        );
        let type_names: std::collections::HashSet<&str> =
            types_for_kind(kind).iter().map(|t| t.name).collect();
        let mut seen = std::collections::HashSet::new();

        for op in ops {
            let where_ = format!("{}.{}", kind.as_str(), op.name);
            // 1. method is structurally typed and renders to the upstream verb.
            assert!(
                !op.method.as_str().is_empty(),
                "{where_}: method must render"
            );
            // 2. op names are unique per kind (callable dispatch keys).
            assert!(seen.insert(op.name), "{where_}: duplicate op name");
            // 3. path placeholders match declared path_params (both directions).
            //    Placeholders may be whole-segment (`/{id}/`) OR embedded
            //    (`stream.{container}`), so scan for every `{name}` substring — the
            //    same way build_operation_url substitutes them.
            let mut placeholders: Vec<&str> = Vec::new();
            let mut rest = op.path;
            while let Some(open) = rest.find('{') {
                let after = &rest[open + 1..];
                let close = after.find('}').expect("balanced braces in generated path");
                placeholders.push(&after[..close]);
                rest = &after[close + 1..];
            }
            for ph in &placeholders {
                assert!(
                    op.path_params.contains(ph),
                    "{where_}: path placeholder {{{ph}}} not in path_params"
                );
            }
            for pp in op.path_params {
                assert!(
                    placeholders.contains(pp),
                    "{where_}: path_param `{pp}` has no {{{pp}}} placeholder"
                );
            }
            // 4. request/response type references resolve within the kind's TYPES.
            for ty in [op.request_type, op.response_type].into_iter().flatten() {
                assert!(
                    type_names.contains(ty),
                    "{where_}: type `{ty}` not found in generated TYPES"
                );
            }
        }
    }
}
