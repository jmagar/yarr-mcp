use anyhow::{bail, Result};
use serde_json::json;
use std::collections::BTreeMap;

use super::{
    assertions, configured_service_names, http, live_base_url, matrix, process, report,
    LIVE_AUTH_PORT, LIVE_OAUTH_PORT, LIVE_PORT,
};

pub(super) fn run_rest(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
) -> Result<()> {
    let base = live_base_url();
    let mut server = rustarr.start_server(LIVE_PORT)?;
    server.wait_healthy(&base)?;

    for (route, key) in [
        ("/health", "status"),
        ("/ready", "ready"),
        ("/status", "server"),
    ] {
        let (status, body) = http::get_text(&format!("{base}{route}"))?;
        if status != 200 || !body.contains(key) {
            bail!("GET {route} expected 200 and {key}, got {status}: {body}");
        }
        report.pass(format!("rest GET {route}"), format!("status={status}"));
    }

    let (status, _) = http::get_text(&format!("{base}/__rustarr_live_missing_route__"))?;
    if status != 404 {
        bail!("missing route expected 404, got {status}");
    }
    report.pass("rest GET unknown route", "status=404");

    let token = "rustarr-live-token";
    let auth_base = format!("http://127.0.0.1:{LIVE_AUTH_PORT}");
    let mut auth_env = BTreeMap::new();
    auth_env.insert("RUSTARR_MCP_NO_AUTH".into(), "false".into());
    auth_env.insert("RUSTARR_NOAUTH".into(), "false".into());
    auth_env.insert("RUSTARR_MCP_AUTH_MODE".into(), "bearer".into());
    auth_env.insert("RUSTARR_MCP_TOKEN".into(), token.into());
    let mut auth_server =
        rustarr.start_server_args(&["serve", "mcp"], "127.0.0.1", LIVE_AUTH_PORT, &auth_env)?;
    auth_server.wait_healthy(&auth_base)?;
    let unauthorized = http::mcp_status(&auth_base, "tools/list", None, None)?;
    if unauthorized != 401 {
        bail!("missing bearer expected 401, got {unauthorized}");
    }
    report.pass(
        "rest mcp auth rejects missing bearer",
        "missing bearer rejected with 401",
    );
    let authorized = http::mcp_with_auth(&auth_base, "tools/list", None, 88, Some(token))?;
    if !authorized.to_string().contains("\"rustarr\"") {
        bail!("authorized tools/list did not advertise rustarr: {authorized}");
    }
    report.pass(
        "rest mcp auth accepts bearer",
        "authorized tools/list succeeded",
    );

    let oauth_base = format!("http://127.0.0.1:{LIVE_OAUTH_PORT}");
    let mut oauth_env = BTreeMap::new();
    oauth_env.insert("RUSTARR_MCP_NO_AUTH".into(), "false".into());
    oauth_env.insert("RUSTARR_NOAUTH".into(), "false".into());
    oauth_env.insert("RUSTARR_MCP_AUTH_MODE".into(), "oauth".into());
    oauth_env.insert("RUSTARR_MCP_PUBLIC_URL".into(), oauth_base.clone());
    oauth_env.insert(
        "RUSTARR_MCP_GOOGLE_CLIENT_ID".into(),
        "rustarr-live-client".into(),
    );
    oauth_env.insert(
        "RUSTARR_MCP_GOOGLE_CLIENT_SECRET".into(),
        "rustarr-live-secret".into(),
    );
    oauth_env.insert(
        "RUSTARR_MCP_AUTH_ADMIN_EMAIL".into(),
        "rustarr-live@example.com".into(),
    );
    oauth_env.insert(
        "RUSTARR_MCP_AUTH_SQLITE_PATH".into(),
        "target/live-full/tmp/oauth/auth.sqlite".into(),
    );
    oauth_env.insert(
        "RUSTARR_MCP_AUTH_KEY_PATH".into(),
        "target/live-full/tmp/oauth/jwks.json".into(),
    );
    std::fs::create_dir_all("target/live-full/tmp/oauth")?;
    let mut oauth_server =
        rustarr.start_server_args(&["serve", "mcp"], "127.0.0.1", LIVE_OAUTH_PORT, &oauth_env)?;
    oauth_server.wait_healthy(&oauth_base)?;
    let (auth_meta_status, auth_meta) = http::get_text(&format!(
        "{oauth_base}/mcp/.well-known/oauth-authorization-server"
    ))?;
    if auth_meta_status != 200 || !auth_meta.contains("authorization_endpoint") {
        bail!("OAuth authorization metadata failed: {auth_meta_status} {auth_meta}");
    }
    report.pass(
        "rest oauth authorization metadata",
        format!("status={auth_meta_status}"),
    );
    let (resource_meta_status, resource_meta) = http::get_text(&format!(
        "{oauth_base}/mcp/.well-known/oauth-protected-resource"
    ))?;
    if resource_meta_status != 200 || !resource_meta.contains("resource") {
        bail!("OAuth protected resource metadata failed: {resource_meta_status} {resource_meta}");
    }
    report.pass(
        "rest oauth protected resource metadata",
        format!("status={resource_meta_status}"),
    );
    Ok(())
}

pub(super) fn run_mcp(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
    matrix: &matrix::Matrix,
) -> Result<()> {
    let base = live_base_url();
    let mut server = rustarr.start_server(LIVE_PORT)?;
    server.wait_healthy(&base)?;

    let init = http::mcp(
        &base,
        "initialize",
        Some(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {"name": "rustarr-live-test", "version": "1.0.0"}
        })),
        1,
    )?;
    assertions::assert_value(
        &init,
        &matrix::Expectation {
            json_path: Some("serverInfo.name".into()),
            equals: Some(json!("rustarr-mcp")),
            equals_any: None,
            value_type: None,
            contains: None,
            xml_root: None,
        },
    )?;
    report.pass("mcp initialize", "rustarr-mcp");

    let tools = http::mcp(&base, "tools/list", None, 2)?;
    if !tools.to_string().contains("\"rustarr\"") {
        bail!("tools/list did not advertise rustarr: {tools}");
    }
    report.pass("mcp tools/list", "rustarr tool advertised");

    let resources = http::mcp(&base, "resources/list", None, 3)?;
    report.pass(
        "mcp resources/list",
        format!("{} bytes", resources.to_string().len()),
    );

    let schema = http::mcp(
        &base,
        "resources/read",
        Some(json!({"uri":"rustarr://schema/mcp-tool"})),
        33,
    )?;
    if !schema.to_string().contains("inputSchema") {
        bail!("resources/read schema did not include inputSchema: {schema}");
    }
    report.pass("mcp resources/read schema", "schema resource returned");

    let prompts = http::mcp(&base, "prompts/list", None, 4)?;
    if !prompts.to_string().contains("quick_start") {
        bail!("prompts/list did not advertise quick_start: {prompts}");
    }
    report.pass("mcp prompts/list", "quick_start advertised");

    let quick_start = http::mcp(&base, "prompts/get", Some(json!({"name":"quick_start"})), 5)?;
    assertions::assert_value(
        &quick_start,
        &matrix::Expectation {
            json_path: Some("messages".into()),
            equals: None,
            equals_any: None,
            value_type: Some("array".into()),
            contains: None,
            xml_root: None,
        },
    )?;
    report.pass("mcp prompts/get quick_start", "prompt returned messages");

    let help = http::mcp_tool(&base, json!({"action":"help"}), 6)?;
    assertions::assert_value(
        &help,
        &matrix::Expectation {
            json_path: Some("help".into()),
            equals: None,
            equals_any: None,
            value_type: Some("string".into()),
            contains: None,
            xml_root: None,
        },
    )?;
    report.pass("mcp tool help", "structured help returned");

    let unknown_tool = http::mcp(
        &base,
        "tools/call",
        Some(json!({"name":"__rustarr_live_missing_tool__","arguments":{}})),
        66,
    );
    let unknown_error = unknown_tool.expect_err("unknown MCP tool should fail");
    if !unknown_error.to_string().contains("execution_error") {
        bail!("unknown MCP tool produced unexpected error: {unknown_error}");
    }
    report.pass("mcp unknown tool error", "unknown tool rejected");

    let invalid_api_get = http::mcp_tool(&base, json!({"action":"api_get","service":"sonarr"}), 67);
    let invalid_api_get_error = invalid_api_get.expect_err("api_get without path should fail");
    if !invalid_api_get_error.to_string().contains("path") {
        bail!("api_get validation error did not mention path: {invalid_api_get_error}");
    }
    report.pass("mcp api_get validation error", "missing path rejected");

    let integrations = http::mcp_tool(&base, json!({"action":"integrations"}), 7)?;
    let configured = configured_service_names(&integrations)?;
    for service in &matrix.services {
        if !configured.iter().any(|name| name == &service.name) {
            bail!(
                "MCP integrations missing configured service {}",
                service.name
            );
        }
    }
    report.pass(
        "mcp tool integrations",
        format!("{} configured services returned", configured.len()),
    );

    for (idx, service) in matrix.services.iter().enumerate() {
        let id = 100 + idx as u64;
        let status = http::mcp_tool(
            &base,
            json!({"action":"service_status","service":service.name}),
            id,
        )?;
        assertions::assert_value(&status, &service.status)?;
        report.pass(
            format!("mcp service_status {}", service.name),
            format!("semantic status matched ({})", service.kind),
        );

        for get_case in &service.get {
            let payload = http::mcp_tool(
                &base,
                json!({"action":"api_get","service":service.name,"path":get_case.path}),
                id + 1000,
            )?;
            assertions::assert_value(&payload, &get_case.expectation)?;
            report.pass(
                format!("mcp api_get {} {}", service.name, get_case.path),
                "semantic GET matched",
            );
        }

        let blocked = http::mcp_tool(
            &base,
            json!({
                "action":"api_post",
                "service":service.name,
                "path":service.post_blocked.path,
                "body":service.post_blocked.body,
                "confirm":false
            }),
            id + 2000,
        );
        let error = blocked.expect_err("api_post without confirm should fail");
        let mcp_error_tokens = vec!["execution_error".to_string(), "api_post".to_string()];
        assertions::assert_expected_error(&error.to_string(), &mcp_error_tokens)?;
        report.pass(
            format!("mcp api_post confirm guard {}", service.name),
            "blocked before upstream mutation",
        );

        let expected = http::mcp_tool(
            &base,
            json!({
                "action":"api_post",
                "service":service.name,
                "path":service.post_expected_error.path,
                "body":service.post_expected_error.body,
                "confirm":true
            }),
            id + 3000,
        );
        match expected {
            Ok(payload) => {
                assertions::assert_expected_error(
                    &payload.to_string(),
                    &service.post_expected_error.error_contains_any,
                )?;
            }
            Err(error) => {
                let mcp_error_tokens = vec!["execution_error".to_string(), "api_post".to_string()];
                assertions::assert_expected_error(&error.to_string(), &mcp_error_tokens)?;
            }
        }
        report.pass(
            format!("mcp api_post safe upstream error {}", service.name),
            "MCP execution error matched",
        );
    }
    report.pass(
        "mcp api_post safe upstream error",
        "all services returned sanitized MCP execution errors",
    );
    Ok(())
}
