use super::*;
use crate::cli::command::Command;

#[test]
fn terminate_stream_parses_id_reason_and_confirm() {
    let args = vec![
        "--id".to_string(),
        "stream-1".to_string(),
        "--reason".to_string(),
        "maintenance".to_string(),
        "--confirm".to_string(),
    ];
    let cmd = parse(ServiceKind::Tracearr, "terminate-stream", &args)
        .unwrap()
        .unwrap();
    let Command::Curated { action, params } = cmd else {
        panic!("expected curated command");
    };
    assert_eq!(action, "trace_terminate_stream");
    assert_eq!(params["id"], "stream-1");
    assert_eq!(params["reason"], "maintenance");
    assert_eq!(params["confirm"], true);
}

#[test]
fn streams_summary_flag_maps_to_boolean_param() {
    let args = vec!["--summary".to_string()];
    let cmd = parse(ServiceKind::Tracearr, "streams", &args)
        .unwrap()
        .unwrap();
    let Command::Curated { action, params } = cmd else {
        panic!("expected curated command");
    };
    assert_eq!(action, "trace_streams");
    assert_eq!(params["summary"], true);
}
