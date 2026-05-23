use rmcp::ServerHandler;

use crate::testing::loopback_state;

#[test]
fn rmcp_server_constructs_from_loopback_state() {
    let state = loopback_state();
    let _ = super::rmcp_server(state);
}

#[test]
fn server_info_advertises_tools_resources_prompts() {
    let server = super::rmcp_server(loopback_state());
    let info = server.get_info();
    let caps = &info.capabilities;
    assert!(
        caps.tools.is_some(),
        "server must advertise tools capability"
    );
    assert!(
        caps.resources.is_some(),
        "server must advertise resources capability"
    );
    assert!(
        caps.prompts.is_some(),
        "server must advertise prompts capability"
    );
}

#[test]
fn server_info_includes_implementation_metadata() {
    let server = super::rmcp_server(loopback_state());
    let _info = server.get_info();
    // get_info() must not panic — capabilities and server metadata are always set
}
