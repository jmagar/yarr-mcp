use super::*;
/// Run one op. Returns its classified result plus the successful response body (so
/// the caller can harvest resource ids for create-first seeding).
pub(super) fn run_op(
    yarr: &process::YarrProcess,
    svc: &str,
    kind: ServiceKind,
    spec: &Spec,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    no_destructive: bool,
) -> (OpResult, Option<Value>) {
    let mk = |outcome, detail: String| OpResult {
        name: op.name,
        method: op.method.as_str(),
        path: op.path,
        outcome,
        detail,
        args: None,
    };
    let args = match prepare_op_args(kind, spec, op, fixtures, no_destructive, false) {
        PreparedOp::Call(args) => args,
        PreparedOp::Skip(detail) => return (mk("skipped", detail), None),
    };
    match invoke(yarr, svc, op.name, &args) {
        Ok(Some(value)) => {
            let result = match op.response_type {
                Some(ty) => match spec.validate_response(ty, &value) {
                    Ok(()) => mk("ok", format!("2xx + matches {ty}")),
                    Err(e) => mk(
                        "schema_mismatch",
                        format!("{e}").chars().take(500).collect(),
                    ),
                },
                None => mk("ok", "2xx (no declared response type to validate)".into()),
            };
            (result, Some(value))
        }
        Ok(None) => (mk("ok", "2xx (empty/non-JSON body)".into()), None),
        Err(e) => (
            mk("rejected", format!("{e}").chars().take(500).collect()),
            None,
        ),
    }
}

pub(in crate::live) enum PreparedOp {
    Call(Map<String, Value>),
    Skip(String),
}

pub(in crate::live) fn prepare_op_args(
    kind: ServiceKind,
    spec: &Spec,
    op: &OperationSpec,
    fixtures: &FixtureStore,
    no_destructive: bool,
    allow_reset_required: bool,
) -> PreparedOp {
    if no_destructive && op.method.is_delete() {
        return PreparedOp::Skip("destructive (DELETE) skipped via --no-destructive".into());
    }
    // NEVER call self-destructive control endpoints: shutdown/restart stop the
    // service mid-run (which is exactly what took prowlarr down), and backup/restore
    // rewrites its whole config. Testing "every endpoint" cannot mean bricking the
    // stack — these are skipped by design.
    if op_requires_stack_reset(op) && !allow_reset_required {
        return PreparedOp::Skip(
            "requires stack reset/reseed (control endpoint or config/auth mutation)".into(),
        );
    }
    // Satisfy path params from discovered/seeded fixtures (parent collection =
    // path before `{`). No fallback IDs: a contract call that needs a resource but
    // has no resource fixture is not a meaningful live test.
    let mut path_args = Map::new();
    if !op.path_params.is_empty() {
        let parent = fixture_parent_path(op.path);
        for p in op.path_params {
            let Some(value) = fixture_path_value(fixtures, parent, p) else {
                return PreparedOp::Skip(format!(
                    "no live fixture for path param `{p}` under `{parent}`"
                ));
            };
            path_args.insert((*p).to_string(), value);
        }
    }
    let Some(mut args) = spec.build_args(op.method.as_str(), op.path, &path_args) else {
        return PreparedOp::Call(path_args);
    };
    apply_fixture_args(kind, op, fixtures, &mut args);
    if op.has_body
        && let Some(body) = live_fixture_body_for_op(kind, op, fixtures)
    {
        args.insert("body".into(), body);
        return PreparedOp::Call(args);
    }
    if op.has_body
        && can_reuse_fixture_body(op)
        && let Some(body) = fixture_body_for_op(fixtures, op)
    {
        args.insert("body".into(), body.clone());
    }
    PreparedOp::Call(args)
}

pub(in crate::live) fn op_requires_stack_reset(op: &OperationSpec) -> bool {
    let lp = op.path.to_ascii_lowercase();
    lp.contains("shutdown")
        || lp.contains("restart")
        || lp.contains("/backup/restore")
        || lp.ends_with("/system/backup")
        || (!op.method.is_read()
            && (lp.contains("/settings")
                || lp.contains("/auth")
                || lp.contains("/config")
                || lp.contains("/configuration")
                || lp.contains("/startup")
                || lp.contains("/prefs")
                || lp.contains("apikey")))
}

#[cfg(test)]
pub(super) fn is_known_non_contract_endpoint(path: &str) -> bool {
    let lp = path.to_ascii_lowercase();
    lp == "/login" || lp == "/logout" || lp.ends_with(".ics") || lp.ends_with("/system/routes")
}

#[cfg(test)]
pub(super) fn is_unseeded_optional_feature_endpoint(kind: ServiceKind, path: &str) -> bool {
    let lp = path.to_ascii_lowercase();
    match kind {
        ServiceKind::Jellyfin => {
            lp.starts_with("/livetv/")
                || lp.starts_with("/syncplay/")
                || lp.starts_with("/items/remotesearch/")
                || lp.starts_with("/quickconnect/")
        }
        ServiceKind::Plex => {
            lp.starts_with("/livetv/")
                || lp.starts_with("/media/subscriptions")
                || lp.starts_with("/media/grabbers")
                || lp.starts_with("/media/providers")
                || lp.starts_with("/downloadqueue")
        }
        _ => false,
    }
}
