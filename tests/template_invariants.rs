use serde_json::Value;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
}

fn json(path: &str) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|err| panic!("failed to parse {path}: {err}"))
}

#[test]
fn agent_memory_files_are_claude_symlinks() {
    for path in ["AGENTS.md", "GEMINI.md"] {
        let target = fs::read_link(path).unwrap_or_else(|err| panic!("{path}: {err}"));
        assert_eq!(
            target,
            Path::new("CLAUDE.md"),
            "{path} should link to CLAUDE.md"
        );
    }
    for path in ["plugins/rustarr/AGENTS.md", "plugins/rustarr/GEMINI.md"] {
        let target = fs::read_link(path).unwrap_or_else(|err| panic!("{path}: {err}"));
        assert_eq!(
            target,
            Path::new("CLAUDE.md"),
            "{path} should link to CLAUDE.md"
        );
    }
    for path in ["docs/AGENTS.md", "docs/GEMINI.md"] {
        let target = fs::read_link(path).unwrap_or_else(|err| panic!("{path}: {err}"));
        assert_eq!(
            target,
            Path::new("CLAUDE.md"),
            "{path} should link to CLAUDE.md"
        );
    }
}

#[test]
fn portable_scripts_are_executable_and_documented() {
    let docs = read("scripts/README.md");
    for path in [
        "scripts/check-dependency-updates.sh",
        "scripts/check-file-size.sh",
        "scripts/asciicheck.py",
        "scripts/check-blob-size.py",
        "scripts/check-runtime-current.sh",
        "scripts/validate-plugin-layout.sh",
        "scripts/test-mcp-auth.sh",
        "scripts/pre-release-check.sh",
        "scripts/test-template-features.sh",
        "scripts/check-schema-docs.py",
        "scripts/check-coupled-files.sh",
        "scripts/live-read-smoke.sh",
    ] {
        let metadata = fs::metadata(path).unwrap_or_else(|err| panic!("{path}: {err}"));
        assert!(
            metadata.permissions().mode() & 0o111 != 0,
            "{path} should be executable"
        );
        let basename = Path::new(path).file_name().unwrap().to_string_lossy();
        assert!(
            docs.contains(basename.as_ref()),
            "scripts/README.md should document {basename}"
        );
    }
}

#[test]
fn justfile_exposes_ported_automation_recipes() {
    let justfile = read("Justfile");
    for recipe in [
        "install-tools:",
        "bootstrap:",
        "install-hooks:",
        "uninstall-hooks:",
        "deps-check:",
        "blob-size-check:",
        "coupled-files-check:",
        "ascii-check:",
        "ascii-fix:",
        "file-size-check:",
        "schema-docs:",
        "schema-docs-check:",
        "template-features:",
        "template-check:",
        "test-cov:",
        "watch:",
        "runtime-current:",
        "auth-smoke:",
        "pre-release:",
        "up:",
        "down:",
        "release:",
    ] {
        assert!(justfile.contains(recipe), "Justfile missing {recipe}");
    }
}

#[test]
fn plugin_manifests_do_not_have_version_fields() {
    for path in [
        "plugins/rustarr/.claude-plugin/plugin.json",
        "plugins/rustarr/.codex-plugin/plugin.json",
        "plugins/rustarr/gemini-extension.json",
    ] {
        let manifest = json(path);
        assert!(
            !manifest.as_object().unwrap().contains_key("version"),
            "{path} must not contain a version field"
        );
    }
}

#[test]
fn registry_and_deploy_metadata_are_rustarr_specific() {
    let server = json("server.json");
    assert_eq!(server["name"], "tv.tootie/rustarr-mcp");
    assert_eq!(
        server["description"],
        "MCP server for querying and automating a configured media automation fleet."
    );

    for path in [
        "server.json",
        "install.sh",
        "docker-compose.prod.yml",
        ".env.example",
        "config.example.toml",
    ] {
        let text = read(path);
        for placeholder in [
            "TEMPLATE: Replace",
            "your-org",
            "yourdomain.com",
            "myservice",
            "RUSTARR_API_URL",
            "RUSTARR_API_KEY",
        ] {
            assert!(
                !text.contains(placeholder),
                "{path} still contains placeholder `{placeholder}`"
            );
        }
    }
}

#[test]
fn schema_contract_doc_tracks_known_actions() {
    let doc = read("docs/MCP_SCHEMA.md");
    let actions = read("src/actions.rs");
    let schemas = read("src/mcp/schemas.rs");
    for action in [
        "integrations",
        "service_status",
        "api_get",
        "api_post",
        "help",
    ] {
        assert!(actions.contains(action), "actions.rs missing {action}");
        assert!(
            doc.contains(&format!("`{action}`")),
            "schema doc missing {action}"
        );
    }
    assert!(
        schemas.contains("action_names()"),
        "schemas.rs should derive action enum from action metadata"
    );
}
