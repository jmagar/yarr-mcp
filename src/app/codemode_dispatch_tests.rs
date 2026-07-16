#[tokio::test]
async fn dispatch_rejects_recursive_codemode_and_non_object_params() {
    let service = crate::testing::loopback_state().service;
    let recursive = service
        .codemode_dispatch("codemode", "{}", false, None)
        .await
        .unwrap_err();
    assert!(recursive.contains("cannot invoke codemode"));

    let non_object = service
        .codemode_dispatch("help", "[]", false, None)
        .await
        .unwrap_err();
    assert!(non_object.contains("must be a JSON object"));
}

#[tokio::test]
async fn snippet_dispatch_rejects_nested_snippet_run() {
    let service = crate::testing::loopback_state().service;
    let error = service
        .codemode_dispatch(
            "snippet_run",
            r#"{"name":"nested","input":null}"#,
            true,
            None,
        )
        .await
        .unwrap_err();
    assert!(error.contains("cannot run another snippet"));
}
