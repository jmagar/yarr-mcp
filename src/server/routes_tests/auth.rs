pub(super) fn codemode_state() -> crate::server::AppState {
    crate::testing::loopback_state()
}

#[path = "auth_tests.rs"]
mod tests;
