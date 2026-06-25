use serde_json::Value;
use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
}

fn json(path: &str) -> Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|err| panic!("failed to parse {path}: {err}"))
}

#[test]
fn plugin_manifests_exist_for_all_supported_hosts() {
    for path in [
        "plugins/rustarr/.claude-plugin/plugin.json",
        "plugins/rustarr/.codex-plugin/plugin.json",
        "plugins/rustarr/gemini-extension.json",
        "plugins/rustarr/hooks/hooks.json",
        "plugins/rustarr/skills/rustarr/SKILL.md",
    ] {
        assert!(std::path::Path::new(path).exists(), "{path} should exist");
    }

    assert!(
        !std::path::Path::new("plugins/rustarr/.mcp.json").exists(),
        "rustarr plugin should stay on the no-MCP marketplace variant"
    );
}

#[test]
fn plugin_manifests_share_identity_and_connection_settings() {
    let claude = json("plugins/rustarr/.claude-plugin/plugin.json");
    let codex = json("plugins/rustarr/.codex-plugin/plugin.json");
    let gemini = json("plugins/rustarr/gemini-extension.json");

    assert_eq!(claude["name"], "rustarr");
    assert_eq!(codex["name"], "rustarr-mcp");
    assert_eq!(gemini["name"], "rustarr-mcp");

    assert!(
        claude["repository"]
            .as_str()
            .unwrap()
            .ends_with("rustarr-mcp")
    );
    assert!(
        codex["repository"]
            .as_str()
            .unwrap()
            .ends_with("rustarr-mcp")
    );
    assert!(
        gemini["repository"]
            .as_str()
            .unwrap()
            .ends_with("rustarr-mcp")
    );

    let user_config = claude["userConfig"].as_object().unwrap();
    for key in [
        "server_url",
        "api_token",
        "rustarr_services",
        "sonarr_url",
        "sonarr_api_key",
        "radarr_url",
        "radarr_api_key",
    ] {
        assert!(
            user_config.contains_key(key),
            "Claude userConfig missing {key}"
        );
    }

    let gemini_settings: Vec<&str> = gemini["settings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|setting| setting["name"].as_str().unwrap())
        .collect();
    for key in [
        "server_url",
        "api_token",
        "rustarr_services",
        "sonarr_url",
        "sonarr_api_key",
        "radarr_url",
        "radarr_api_key",
    ] {
        assert!(
            gemini_settings.contains(&key),
            "Gemini settings missing {key}"
        );
    }

    assert!(
        claude.get("mcpServers").is_none(),
        "Claude no-MCP plugin variant should not declare inline MCP servers"
    );
    assert!(
        gemini.get("mcpServers").is_none(),
        "Gemini no-MCP extension variant should not declare inline MCP servers"
    );
}

#[test]
fn claude_hooks_call_binary_owned_hook_command() {
    let hooks = json("plugins/rustarr/hooks/hooks.json");
    for hook_name in ["SessionStart", "ConfigChange"] {
        let command = hooks["hooks"][hook_name][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert_eq!(
            command,
            "${CLAUDE_PLUGIN_ROOT}/bin/rustarr setup plugin-hook"
        );
        // The hook calls the binary directly — no shell adapter owning
        // systemd or Docker orchestration.
        assert!(
            !command.contains("systemctl"),
            "plugin hook should not own systemd orchestration"
        );
        assert!(
            !command.contains("docker compose"),
            "plugin hook should not own Docker orchestration"
        );
    }
}

#[test]
fn plugin_hook_standard_is_documented() {
    let plugins = read("docs/PLUGINS.md");
    let patterns = read("docs/PATTERNS.md");
    for doc in [plugins, patterns] {
        assert!(doc.contains("<binary> setup plugin-hook"));
        assert!(doc.contains("<binary> setup plugin-hook --no-repair"));
        assert!(doc.contains("exit_policy"));
        assert!(doc.contains("blocking_failures"));
        assert!(doc.contains("advisory_failures"));
        assert!(doc.contains("ran_repair"));
    }
}

fn rustarr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rustarr")
}

fn setup_command(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(rustarr_bin());
    cmd.env_clear()
        .env("HOME", data_dir)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("RUSTARR_HOME", data_dir)
        .env("RUSTARR_SERVICES", "sonarr")
        .env("RUSTARR_SONARR_KIND", "sonarr")
        .env("RUSTARR_SONARR_URL", "https://api.rustarr.test")
        .env("RUSTARR_SONARR_API_KEY", "rustarr-secret")
        .env("RUSTARR_MCP_PORT", "0")
        .env("RUSTARR_MCP_TOKEN", "mcp-secret");
    cmd
}

#[test]
fn setup_plugin_hook_no_repair_emits_json_contract() {
    let dir = tempdir().unwrap();
    let mut cmd = setup_command(dir.path());
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["exit_policy"], "advisory_failure");
    assert_eq!(json["ran_repair"], false);
    assert_eq!(json["no_repair"], true);
    assert!(json["blocking_failures"].as_array().unwrap().is_empty());
    assert!(
        json["advisory_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["code"] == "env_file_missing")
    );
    assert!(!dir.path().join(".env").exists());
}

#[test]
fn setup_repair_creates_env_file_without_upstream_contact() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("appdata");
    let mut cmd = setup_command(&missing);
    let output = cmd.args(["setup", "repair"]).output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["exit_policy"], "success");
    assert_eq!(json["ran_repair"], true);
    assert_eq!(json["no_repair"], false);

    let env_file = std::fs::read_to_string(missing.join(".env")).unwrap();
    assert!(env_file.contains("RUSTARR_SONARR_URL=https://api.rustarr.test"));
    assert!(env_file.contains("RUSTARR_SONARR_API_KEY=rustarr-secret"));
    assert!(env_file.contains("RUSTARR_MCP_TOKEN=mcp-secret"));
    assert_env_file_mode(missing.join(".env").as_path());
}

#[test]
fn setup_repair_replaces_existing_env_file_with_private_mode() {
    let dir = tempdir().unwrap();
    let env_path = dir.path().join(".env");
    fs::write(&env_path, "OLD_VALUE=1\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&env_path, fs::Permissions::from_mode(0o644)).unwrap();

    let mut cmd = setup_command(dir.path());
    let output = cmd.args(["setup", "repair"]).output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let env_file = fs::read_to_string(&env_path).unwrap();
    assert!(!env_file.contains("OLD_VALUE"));
    assert!(env_file.contains("RUSTARR_SONARR_URL=https://api.rustarr.test"));
    assert_env_file_mode(&env_path);
}

fn assert_env_file_mode(path: &std::path::Path) {
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

// ── OAuth setup validation (H12) ─────────────────────────────────────────────
//
// These helpers build a Command with OAuth mode enabled and all four OAuth
// credentials present, then selectively omit one field per test to confirm
// the expected blocking-failure code is reported by `setup plugin-hook
// --no-repair`.
//
// Notes:
//   - `setup_command` sets RUSTARR_MCP_TOKEN, which normally selects bearer
//     mode.  We override that by adding RUSTARR_MCP_AUTH_MODE=oauth.
//   - We omit RUSTARR_MCP_TOKEN here so the setup logic enters the OAuth
//     credential-check branch (token takes precedence in bearer mode).
//   - Port is kept at 0 (from setup_command) to avoid mcp_port_in_use noise.

fn oauth_setup_command(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(rustarr_bin());
    cmd.env_clear()
        .env("HOME", data_dir)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("RUSTARR_HOME", data_dir)
        .env("RUSTARR_SERVICES", "sonarr")
        .env("RUSTARR_SONARR_KIND", "sonarr")
        .env("RUSTARR_SONARR_URL", "https://api.rustarr.test")
        .env("RUSTARR_SONARR_API_KEY", "rustarr-secret")
        .env("RUSTARR_MCP_PORT", "0")
        .env("RUSTARR_MCP_AUTH_MODE", "oauth")
        .env("RUSTARR_MCP_PUBLIC_URL", "https://mcp.rustarr.test")
        .env("RUSTARR_MCP_GOOGLE_CLIENT_ID", "test-client-id")
        .env("RUSTARR_MCP_GOOGLE_CLIENT_SECRET", "test-client-secret")
        .env("RUSTARR_MCP_AUTH_ADMIN_EMAIL", "admin@rustarr.test");
    cmd
}

fn blocking_failure_codes(output: &std::process::Output) -> Vec<String> {
    let json: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout not JSON: {e}\nstdout: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    json["blocking_failures"]
        .as_array()
        .expect("blocking_failures should be an array")
        .iter()
        .map(|f| f["code"].as_str().unwrap_or("").to_string())
        .collect()
}

#[test]
fn oauth_missing_public_url_produces_blocking_failure() {
    let dir = tempdir().unwrap();
    let mut cmd = oauth_setup_command(dir.path());
    // Remove the public URL so the check fires.
    cmd.env_remove("RUSTARR_MCP_PUBLIC_URL");
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    // setup exits non-zero when there are blocking failures.
    assert!(
        !output.status.success(),
        "expected non-zero exit for blocking failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let codes = blocking_failure_codes(&output);
    assert!(
        codes.contains(&"missing_oauth_public_url".to_string()),
        "expected missing_oauth_public_url in blocking_failures, got: {codes:?}"
    );
}

#[test]
fn oauth_missing_client_id_produces_blocking_failure() {
    let dir = tempdir().unwrap();
    let mut cmd = oauth_setup_command(dir.path());
    cmd.env_remove("RUSTARR_MCP_GOOGLE_CLIENT_ID");
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero exit for blocking failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let codes = blocking_failure_codes(&output);
    assert!(
        codes.contains(&"missing_oauth_client_id".to_string()),
        "expected missing_oauth_client_id in blocking_failures, got: {codes:?}"
    );
}

#[test]
fn oauth_missing_client_secret_produces_blocking_failure() {
    let dir = tempdir().unwrap();
    let mut cmd = oauth_setup_command(dir.path());
    cmd.env_remove("RUSTARR_MCP_GOOGLE_CLIENT_SECRET");
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero exit for blocking failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let codes = blocking_failure_codes(&output);
    assert!(
        codes.contains(&"missing_oauth_client_secret".to_string()),
        "expected missing_oauth_client_secret in blocking_failures, got: {codes:?}"
    );
}

#[test]
fn oauth_missing_admin_email_produces_blocking_failure() {
    let dir = tempdir().unwrap();
    let mut cmd = oauth_setup_command(dir.path());
    cmd.env_remove("RUSTARR_MCP_AUTH_ADMIN_EMAIL");
    let output = cmd
        .args(["setup", "plugin-hook", "--no-repair"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected non-zero exit for blocking failure; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let codes = blocking_failure_codes(&output);
    assert!(
        codes.contains(&"missing_oauth_admin_email".to_string()),
        "expected missing_oauth_admin_email in blocking_failures, got: {codes:?}"
    );
}

// ── write_env OAuth branch (L28) ──────────────────────────────────────────────
//
// When `auth_mode = OAuth` with all OAuth fields set, `setup repair` must
// write a .env that includes RUSTARR_MCP_AUTH_MODE=oauth and all four OAuth
// credential lines.

#[test]
fn setup_repair_oauth_writes_oauth_env_lines() {
    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("appdata");
    let mut cmd = oauth_setup_command(&data_dir);
    let output = cmd.args(["setup", "repair"]).output().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["exit_policy"], "success");
    assert_eq!(json["ran_repair"], true);

    let env_file = fs::read_to_string(data_dir.join(".env")).unwrap();
    assert!(
        env_file.contains("RUSTARR_MCP_AUTH_MODE=oauth"),
        ".env should contain RUSTARR_MCP_AUTH_MODE=oauth"
    );
    assert!(
        env_file.contains("RUSTARR_MCP_PUBLIC_URL=https://mcp.rustarr.test"),
        ".env should contain RUSTARR_MCP_PUBLIC_URL"
    );
    assert!(
        env_file.contains("RUSTARR_MCP_GOOGLE_CLIENT_ID=test-client-id"),
        ".env should contain RUSTARR_MCP_GOOGLE_CLIENT_ID"
    );
    assert!(
        env_file.contains("RUSTARR_MCP_GOOGLE_CLIENT_SECRET=test-client-secret"),
        ".env should contain RUSTARR_MCP_GOOGLE_CLIENT_SECRET"
    );
    assert!(
        env_file.contains("RUSTARR_MCP_AUTH_ADMIN_EMAIL=admin@rustarr.test"),
        ".env should contain RUSTARR_MCP_AUTH_ADMIN_EMAIL"
    );
    assert_env_file_mode(&data_dir.join(".env"));
}
