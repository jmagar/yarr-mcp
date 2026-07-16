use anyhow::Result;
use std::path::Path;

use super::{reporter::PatternReporter, util::read_file};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceKind {
    CoreRust,
    OperationalRust,
    Web,
}

struct SurfaceFile {
    path: String,
    kind: SurfaceKind,
}

const CORE_RUST_FORBIDDEN: &[&str] = &[
    "reqwest::",
    "hyper::Client",
    "sqlx::",
    "rusqlite::",
    "tokio::fs",
    "std::fs",
    "std::process::Command",
    "Command::new",
    "YarrClient::new",
    "crate::yarr::YarrClient",
];

const WEB_FORBIDDEN: &[&str] = &[
    "child_process",
    "node:child_process",
    "fs.",
    "node:fs",
    "process.env.YARR_",
    "fetch(\"https://",
    "fetch('https://",
];

const BUSINESS_HELPER_TOKENS: &[&str] = &[
    "fn normalize_",
    "fn validate_",
    "fn transform_",
    "fn calculate_",
    "fn enrich_",
    "fn apply_policy",
];

const RUST_DELEGATION_TOKENS: &[&str] = &[
    "state.service",
    "service.",
    "execute_service_action",
    "api_dispatch",
    "streamable_http_service",
];

pub(super) fn thin_surfaces(reporter: &mut PatternReporter) -> Result<()> {
    let files = surface_files()?;
    let mut failures = Vec::new();
    let mut warnings = Vec::new();

    for file in &files {
        let text = read_file(&file.path);
        match file.kind {
            SurfaceKind::CoreRust => {
                let text = strip_inline_test_module(&text);
                check_core_rust_surface(file, text, &mut failures, &mut warnings)
            }
            SurfaceKind::OperationalRust => check_operational_surface(file, &text, &mut warnings),
            SurfaceKind::Web => check_web_surface(file, &text, &mut failures, &mut warnings),
        }
    }

    if !failures.is_empty() {
        reporter.fail(
            "surfaces",
            format!(
                "business/IO logic appears in surface files: {}. Hint: CLI/API/MCP/web surfaces should parse inputs, delegate to YarrService or API endpoints, and format responses only.",
                failures.join("; ")
            ),
        );
    }
    if !warnings.is_empty() {
        reporter.warn(
            "surfaces",
            format!(
                "suspicious surface logic found: {}. Hint: verify this is protocol/UI glue rather than business logic; move validation, normalization, and transformations into app.rs.",
                warnings.join("; ")
            ),
        );
    }
    if failures.is_empty() && warnings.is_empty() {
        reporter.ok(
            "surfaces",
            format!("{} CLI/API/MCP/web surface files look thin", files.len()),
        );
    }
    Ok(())
}

fn check_core_rust_surface(
    file: &SurfaceFile,
    text: &str,
    failures: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    for token in CORE_RUST_FORBIDDEN {
        if !core_token_applies(file, token) {
            continue;
        }
        if contains_forbidden_core_token(file, text, token) {
            failures.push(format!("{} contains `{token}`", file.path));
        }
    }

    for token in BUSINESS_HELPER_TOKENS {
        if text.contains(token) {
            warnings.push(format!("{} contains helper `{token}`", file.path));
        }
    }

    let is_dispatch_surface = matches!(
        file.path.as_str(),
        "src/api.rs" | "src/mcp/tools.rs" | "src/cli.rs"
    );
    if is_dispatch_surface
        && !RUST_DELEGATION_TOKENS
            .iter()
            .any(|token| text.contains(token))
    {
        warnings.push(format!(
            "{} has no obvious service delegation token",
            file.path
        ));
    }

    let json_macro_count = text.matches("json!({").count();
    if json_macro_count > 12 {
        warnings.push(format!(
            "{} has {json_macro_count} json! object literals",
            file.path
        ));
    }
}

fn contains_forbidden_core_token(file: &SurfaceFile, text: &str, token: &str) -> bool {
    if file.path == "src/mcp/rmcp_server.rs" && token == "reqwest::" {
        const TRANSPORT_ERROR_CLASSIFICATION: &str = "error.downcast_ref::<reqwest::Error>()";
        return text
            .replace(TRANSPORT_ERROR_CLASSIFICATION, "")
            .contains(token);
    }
    text.contains(token)
}

fn core_token_applies(file: &SurfaceFile, token: &str) -> bool {
    if file.path == "src/cli.rs" && matches!(token, "std::fs" | "YarrClient::new") {
        return false;
    }
    true
}

fn check_operational_surface(file: &SurfaceFile, text: &str, warnings: &mut Vec<String>) {
    // doctor/ and watch.rs use reqwest for health/connectivity pings — explicitly diagnostics-only.
    let is_diagnostics =
        file.path.starts_with("src/cli/doctor/") || file.path == "src/cli/watch.rs";
    for token in ["reqwest::", "sqlx::", "rusqlite::"] {
        if text.contains(token) && !(token == "reqwest::" && is_diagnostics) {
            warnings.push(format!(
                "{} operational surface contains `{token}`; confirm it is diagnostics-only",
                file.path
            ));
        }
    }
}

fn check_web_surface(
    file: &SurfaceFile,
    text: &str,
    failures: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    for token in WEB_FORBIDDEN {
        if text.contains(token) {
            failures.push(format!("{} contains `{token}`", file.path));
        }
    }

    // template.ts is the canonical action metadata definition for the web UI — it is
    // expected to list all action names and is explicitly excluded from this check.
    let is_definition_file = file.path == "apps/web/lib/template.ts";
    if !is_definition_file {
        let hardcoded_action_count = ["integrations", "service_status", "api_get", "api_post"]
            .iter()
            .filter(|action| text.contains(&format!("\"{action}\"")))
            .count();
        if hardcoded_action_count > 3 {
            warnings.push(format!(
                "{} hardcodes {hardcoded_action_count} action names",
                file.path
            ));
        }
    }
}

fn strip_inline_test_module(text: &str) -> &str {
    let lines = text.lines().collect::<Vec<_>>();
    for index in 0..lines.len().saturating_sub(1) {
        if lines[index].trim() != "#[cfg(test)]" {
            continue;
        }
        let next = lines[index + 1].trim_start();
        if next.starts_with("mod ") || next.starts_with("pub mod ") {
            let byte_index = lines[..index].iter().map(|line| line.len() + 1).sum();
            return &text[..byte_index];
        }
    }
    text
}

fn surface_files() -> Result<Vec<SurfaceFile>> {
    let output = crate::run_cmd_output("git", &["ls-files"])?;
    Ok(output.lines().filter_map(surface_file).collect::<Vec<_>>())
}

fn surface_file(path: &str) -> Option<SurfaceFile> {
    let kind = if matches!(
        path,
        "src/api.rs"
            | "src/server/routes.rs"
            | "src/mcp/tools.rs"
            | "src/mcp/rmcp_server.rs"
            | "src/mcp/schemas.rs"
            | "src/mcp/prompts.rs"
            | "src/cli.rs"
    ) {
        SurfaceKind::CoreRust
    } else if path.starts_with("src/cli/") && path.ends_with(".rs") {
        SurfaceKind::OperationalRust
    } else if is_web_surface(path) {
        SurfaceKind::Web
    } else {
        return None;
    };

    Some(SurfaceFile {
        path: path.to_owned(),
        kind,
    })
}

fn is_web_surface(path: &str) -> bool {
    let p = Path::new(path);
    if path.ends_with(".test.ts") || path.ends_with(".test.tsx") {
        return false;
    }
    let is_tsx = p.extension().and_then(|ext| ext.to_str()) == Some("tsx");
    let is_ts = p.extension().and_then(|ext| ext.to_str()) == Some("ts");
    (path.starts_with("apps/web/app/") && is_tsx) || (path.starts_with("apps/web/lib/") && is_ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_core_rust_surfaces() {
        let file = surface_file("src/mcp/tools.rs").expect("tools should be a surface");
        assert_eq!(file.kind, SurfaceKind::CoreRust);
    }

    #[test]
    fn classifies_operational_cli_surfaces() {
        let file = surface_file("src/cli/doctor.rs").expect("doctor should be a surface");
        assert_eq!(file.kind, SurfaceKind::OperationalRust);
    }

    #[test]
    fn classifies_web_surfaces() {
        let file = surface_file("apps/web/app/page.tsx").expect("web page should be a surface");
        assert_eq!(file.kind, SurfaceKind::Web);
    }

    #[test]
    fn ignores_web_tests() {
        assert!(surface_file("apps/web/lib/template.test.ts").is_none());
    }

    #[test]
    fn ignores_service_layer() {
        assert!(surface_file("src/app.rs").is_none());
    }

    #[test]
    fn rmcp_transport_error_classification_is_a_precise_reqwest_exception() {
        let file = surface_file("src/mcp/rmcp_server.rs").expect("rmcp server is a surface");
        let mut failures = Vec::new();
        let mut warnings = Vec::new();

        check_core_rust_surface(
            &file,
            "error.downcast_ref::<reqwest::Error>().is_some()",
            &mut failures,
            &mut warnings,
        );

        assert!(
            failures.is_empty(),
            "precise error classification is protocol glue: {failures:?}"
        );
    }

    #[test]
    fn rmcp_reqwest_client_use_is_still_rejected() {
        let file = surface_file("src/mcp/rmcp_server.rs").expect("rmcp server is a surface");
        let mut failures = Vec::new();
        let mut warnings = Vec::new();

        check_core_rust_surface(
            &file,
            "let client = reqwest::Client::new();",
            &mut failures,
            &mut warnings,
        );

        assert_eq!(failures, ["src/mcp/rmcp_server.rs contains `reqwest::`"]);
    }
}
