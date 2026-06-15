use super::*;

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn empty_args_is_serve() {
    assert_eq!(RunMode::classify(&args(&[])), RunMode::Serve);
    assert!(RunMode::classify(&args(&[])).is_serve());
}

#[test]
fn serve_and_serve_mcp_are_serve() {
    assert_eq!(RunMode::classify(&args(&["serve"])), RunMode::Serve);
    assert_eq!(RunMode::classify(&args(&["serve", "mcp"])), RunMode::Serve);
}

#[test]
fn mcp_alone_is_stdio() {
    assert_eq!(RunMode::classify(&args(&["mcp"])), RunMode::Stdio);
    assert!(!RunMode::classify(&args(&["mcp"])).is_serve());
}

#[test]
fn other_commands_are_cli() {
    assert_eq!(RunMode::classify(&args(&["integrations"])), RunMode::Cli);
    assert_eq!(RunMode::classify(&args(&["sonarr", "list"])), RunMode::Cli);
    assert_eq!(RunMode::classify(&args(&["status"])), RunMode::Cli);
}

#[test]
fn extra_args_fall_through_to_cli() {
    // `mcp` is stdio only as a lone arg; with extras it's a CLI invocation.
    assert_eq!(RunMode::classify(&args(&["mcp", "extra"])), RunMode::Cli);
    // `serve` only pairs with `mcp`; any other second token is not serve.
    assert_eq!(RunMode::classify(&args(&["serve", "http"])), RunMode::Cli);
}
