pub(super) fn metrics_state() -> crate::server::AppState {
    crate::testing::loopback_state()
}

#[path = "metrics_tests.rs"]
mod tests;
