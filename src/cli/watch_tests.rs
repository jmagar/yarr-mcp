//! Unit tests for src/cli/watch.rs

use std::time::Duration;

use super::*;

// ── ServerState::Display ──────────────────────────────────────────────────────

#[test]
fn server_state_display_up() {
    assert_eq!(ServerState::Up.to_string(), "UP");
}

#[test]
fn server_state_display_down() {
    assert_eq!(ServerState::Down.to_string(), "DOWN");
}

#[test]
fn server_state_display_degraded() {
    assert_eq!(ServerState::Degraded(503).to_string(), "DEGRADED(HTTP 503)");
}

#[test]
fn server_state_display_degraded_404() {
    assert_eq!(ServerState::Degraded(404).to_string(), "DEGRADED(HTTP 404)");
}

// ── format_event ──────────────────────────────────────────────────────────────

#[test]
fn health_url_for_appends_health_to_base_url() {
    assert_eq!(
        health_url_for("http://localhost:40070"),
        "http://localhost:40070/health"
    );
}

#[test]
fn health_url_for_accepts_direct_health_url() {
    assert_eq!(
        health_url_for("http://localhost:40070/health/"),
        "http://localhost:40070/health"
    );
}

#[test]
fn format_event_initial_up() {
    let line = format_event(
        "http://localhost:40070",
        &ServerState::Up,
        None,
        Duration::from_secs(0),
        10,
    );
    assert!(line.contains("UP"), "initial UP line should mention UP");
    assert!(
        line.contains("http://localhost:40070"),
        "line should include the base URL"
    );
}

#[test]
fn format_event_down() {
    let line = format_event(
        "http://localhost:40070",
        &ServerState::Down,
        Some(ServerState::Up),
        Duration::from_secs(30),
        10,
    );
    assert!(line.contains("DOWN"), "down line should mention DOWN");
    assert!(
        line.contains("10"),
        "down line should include retry interval"
    );
}

#[test]
fn format_event_recovery_from_down() {
    let line = format_event(
        "http://localhost:40070",
        &ServerState::Up,
        Some(ServerState::Down),
        Duration::from_secs(120),
        10,
    );
    assert!(line.contains("UP"), "recovery line should mention UP");
    assert!(
        line.contains("120"),
        "recovery line should include previous duration"
    );
    assert!(
        line.contains("DOWN"),
        "recovery line should name the previous state"
    );
}

#[test]
fn format_event_recovery_from_degraded() {
    let line = format_event(
        "http://localhost:40070",
        &ServerState::Up,
        Some(ServerState::Degraded(503)),
        Duration::from_secs(60),
        10,
    );
    assert!(
        line.contains("UP"),
        "recovery from degraded should mention UP"
    );
    assert!(
        line.contains("DEGRADED"),
        "recovery from degraded should name the previous state"
    );
}

#[test]
fn format_event_degraded() {
    let line = format_event(
        "http://localhost:40070",
        &ServerState::Degraded(500),
        Some(ServerState::Up),
        Duration::from_secs(5),
        10,
    );
    assert!(
        line.contains("DEGRADED"),
        "degraded line should mention DEGRADED"
    );
    assert!(
        line.contains("500"),
        "degraded line should include status code"
    );
}
