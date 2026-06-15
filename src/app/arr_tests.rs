//! Tests for the ArrManager module declaration: exercise a re-exported pure
//! helper from the `editor` submodule to prove the module wiring resolves.

use super::*;
use crate::config::ServiceKind;

#[test]
fn editor_id_key_differs_by_kind() {
    // Sonarr and Radarr use different id keys for bulk-edit bodies.
    let sonarr = editor::editor_id_key(ServiceKind::Sonarr);
    let radarr = editor::editor_id_key(ServiceKind::Radarr);
    assert_ne!(sonarr, radarr);
}
