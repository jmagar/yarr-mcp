use super::*;
use crate::cli::command::Command;

#[test]
fn wanted_movies_parses_pagination() {
    let args = vec![
        "--start".to_string(),
        "5".to_string(),
        "--length".to_string(),
        "10".to_string(),
    ];
    let cmd = parse(ServiceKind::Bazarr, "wanted-movies", &args)
        .unwrap()
        .unwrap();
    let Command::Curated { action, params } = cmd else {
        panic!("expected curated command");
    };
    assert_eq!(action, "subtitles_wanted_movies");
    assert_eq!(params["start"], "5");
    assert_eq!(params["length"], "10");
}

#[test]
fn providers_rejects_extra_args() {
    let args = vec!["extra".to_string()];
    assert!(parse(ServiceKind::Bazarr, "providers", &args).is_err());
}
