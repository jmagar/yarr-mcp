//! Facade-level checks for the hand-modeled (doc-based) service contracts. The 6
//! spec-backed services are generated (see `crate::openapi`); only the 5 doc-based
//! services remain here, each with its own colocated `*_tests.rs`.

use super::*;

#[test]
fn doc_based_service_modules_are_wired() {
    // The facade exposes exactly the 5 doc-based services, each with a headline
    // type that implements the contract derives (Deserialize + JsonSchema). This
    // is a wiring check — per-type wire-shape coverage lives in each `*_tests.rs`.
    fn assert_contract<T: serde::de::DeserializeOwned + schemars::JsonSchema>() -> &'static str {
        std::any::type_name::<T>()
    }
    let wired = [
        assert_contract::<bazarr::SystemStatus>(),
        assert_contract::<qbittorrent::TorrentInfo>(),
        assert_contract::<sabnzbd::QueueResponse>(),
        assert_contract::<tautulli::GetHistoryData>(),
        assert_contract::<tracearr::ServerInfo>(),
    ];
    assert_eq!(wired.len(), 5);
}
