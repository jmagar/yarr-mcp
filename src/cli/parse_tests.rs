use super::{
    Selectors, parse_bool_flag, parse_passthrough_flags, parse_selectors, parse_watch_flags,
    reject_args,
};

#[test]
fn reject_args_rejects_extra() {
    assert!(reject_args(&[], "x").is_ok());
    let err = reject_args(&["--bad".into()], "x").unwrap_err();
    assert!(err.to_string().contains("--bad"));
}

#[test]
fn bool_flag_detects_and_dedups() {
    assert!(!parse_bool_flag(&[], "x", "--json").unwrap());
    assert!(parse_bool_flag(&["--json".into()], "x", "--json").unwrap());
    let err = parse_bool_flag(&["--json".into(), "--json".into()], "x", "--json").unwrap_err();
    assert!(err.to_string().contains("duplicate"));
}

#[test]
fn passthrough_requires_path() {
    let err = parse_passthrough_flags(&[], "get", false, false).unwrap_err();
    assert!(err.to_string().contains("requires --path"));
}

#[test]
fn passthrough_parses_body_and_confirm() {
    let flags = parse_passthrough_flags(
        &[
            "--path".into(),
            "/p".into(),
            "--body".into(),
            "{\"a\":1}".into(),
            "--confirm".into(),
        ],
        "post",
        true,
        true,
    )
    .unwrap();
    assert_eq!(flags.path, "/p");
    assert_eq!(flags.body, Some(serde_json::json!({"a": 1})));
    assert!(flags.confirm);
}

#[test]
fn passthrough_yes_aliases_confirm() {
    let flags = parse_passthrough_flags(
        &["--path".into(), "/p".into(), "--yes".into()],
        "post",
        false,
        true,
    )
    .unwrap();
    assert!(flags.confirm);
}

#[test]
fn passthrough_rejects_duplicate_path() {
    let err = parse_passthrough_flags(
        &["--path".into(), "/a".into(), "--path".into(), "/b".into()],
        "get",
        false,
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("duplicate --path"));
}

#[test]
fn passthrough_rejects_duplicate_body_and_confirm() {
    let err = parse_passthrough_flags(
        &[
            "--path".into(),
            "/p".into(),
            "--body".into(),
            "{}".into(),
            "--body".into(),
            "{}".into(),
        ],
        "post",
        true,
        true,
    )
    .unwrap_err();
    assert!(err.to_string().contains("duplicate --body"));

    let err = parse_passthrough_flags(
        &[
            "--path".into(),
            "/p".into(),
            "--confirm".into(),
            "--confirm".into(),
        ],
        "post",
        false,
        true,
    )
    .unwrap_err();
    assert!(err.to_string().contains("duplicate --confirm"));
}

#[test]
fn passthrough_rejects_flag_like_path_and_body_values() {
    let err = parse_passthrough_flags(&["--path".into(), "--body".into()], "get", false, false)
        .unwrap_err();
    assert!(err.to_string().contains("requires a value after --path"));

    let err = parse_passthrough_flags(
        &[
            "--path".into(),
            "/p".into(),
            "--body".into(),
            "--confirm".into(),
        ],
        "post",
        true,
        true,
    )
    .unwrap_err();
    assert!(err.to_string().contains("requires a value after --body"));
}

#[test]
fn passthrough_rejects_confirm_when_disallowed() {
    let err = parse_passthrough_flags(
        &["--path".into(), "/p".into(), "--confirm".into()],
        "get",
        false,
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("does not accept"));
}

#[test]
fn watch_flags_parsed() {
    let (url, interval) = parse_watch_flags(&[
        "--url".into(),
        "http://x".into(),
        "--interval".into(),
        "5".into(),
    ])
    .unwrap();
    assert_eq!(url.as_deref(), Some("http://x"));
    assert_eq!(interval.as_deref(), Some("5"));
}

#[test]
fn selectors_parse_all_four() {
    let s = parse_selectors(
        &[
            "--id".into(),
            "9".into(),
            "--title".into(),
            "Dune".into(),
            "--from".into(),
            "1".into(),
            "--to".into(),
            "5".into(),
        ],
        "x",
    )
    .unwrap();
    assert_eq!(
        s,
        Selectors {
            id: Some("9".into()),
            title: Some("Dune".into()),
            from: Some("1".into()),
            to: Some("5".into()),
        }
    );
}

#[test]
fn selectors_reject_unknown() {
    let err = parse_selectors(&["--bogus".into(), "v".into()], "x").unwrap_err();
    assert!(err.to_string().contains("--bogus"));
}
