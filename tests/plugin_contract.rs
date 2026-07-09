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
        "plugins/yarr/.claude-plugin/plugin.json",
        "plugins/yarr/.codex-plugin/plugin.json",
        "plugins/yarr/gemini-extension.json",
        "plugins/yarr/hooks/hooks.json",
        "plugins/yarr/skills/yarr/SKILL.md",
        "plugins/yarr/.mcp.json",
    ] {
        assert!(std::path::Path::new(path).exists(), "{path} should exist");
    }
}

#[test]
fn mcp_json_defaults_to_stdio_with_the_bundled_binary() {
    let mcp = json("plugins/yarr/.mcp.json");
    let server = &mcp["mcpServers"]["yarr"];

    assert_eq!(server["type"], "stdio");
    assert_eq!(server["command"], "${CLAUDE_PLUGIN_ROOT}/bin/yarr");
    assert_eq!(server["args"], serde_json::json!(["mcp"]));

    // Every service credential field the plugin exposes must have a matching
    // YARR_* env var wired from ${user_config.*} — stdio mode configures the
    // subprocess directly rather than depending on the SessionStart hook
    // having already run (or a remote server being reachable).
    let env = server["env"].as_object().unwrap();
    for (env_var, user_config_key) in [
        ("YARR_SERVICES", "yarr_services"),
        ("YARR_SONARR_URL", "sonarr_url"),
        ("YARR_SONARR_API_KEY", "sonarr_api_key"),
        ("YARR_RADARR_URL", "radarr_url"),
        ("YARR_RADARR_API_KEY", "radarr_api_key"),
        ("YARR_PROWLARR_URL", "prowlarr_url"),
        ("YARR_PROWLARR_API_KEY", "prowlarr_api_key"),
        ("YARR_OVERSEERR_URL", "overseerr_url"),
        ("YARR_OVERSEERR_API_KEY", "overseerr_api_key"),
        ("YARR_JELLYFIN_URL", "jellyfin_url"),
        ("YARR_JELLYFIN_API_KEY", "jellyfin_api_key"),
        ("YARR_PLEX_URL", "plex_url"),
        ("YARR_PLEX_TOKEN", "plex_token"),
        ("YARR_QBITTORRENT_URL", "qbittorrent_url"),
        ("YARR_QBITTORRENT_USERNAME", "qbittorrent_username"),
        ("YARR_QBITTORRENT_PASSWORD", "qbittorrent_password"),
        ("YARR_SABNZBD_URL", "sabnzbd_url"),
        ("YARR_SABNZBD_API_KEY", "sabnzbd_api_key"),
        ("YARR_TAUTULLI_URL", "tautulli_url"),
        ("YARR_TAUTULLI_API_KEY", "tautulli_api_key"),
        ("YARR_TRACEARR_URL", "tracearr_url"),
        ("YARR_BAZARR_URL", "bazarr_url"),
        ("YARR_BAZARR_API_KEY", "bazarr_api_key"),
    ] {
        let expected = format!("${{user_config.{user_config_key}}}");
        assert_eq!(
            env.get(env_var).and_then(Value::as_str),
            Some(expected.as_str()),
            "env[{env_var}] should substitute user_config.{user_config_key}"
        );
    }

    // Every env var this file wires must correspond to a real userConfig
    // field declared in plugin.json — catches typos/renames on either side.
    let claude = json("plugins/yarr/.claude-plugin/plugin.json");
    let user_config = claude["userConfig"].as_object().unwrap();
    for value in env.values() {
        let value = value.as_str().unwrap();
        let key = value
            .strip_prefix("${user_config.")
            .and_then(|rest| rest.strip_suffix('}'))
            .unwrap_or_else(|| panic!("unexpected env value shape: {value}"));
        assert!(
            user_config.contains_key(key),
            "plugins/yarr/.mcp.json references user_config.{key}, which is not in plugin.json's userConfig"
        );
    }
}

#[test]
fn plugin_manifests_share_identity_and_connection_settings() {
    let claude = json("plugins/yarr/.claude-plugin/plugin.json");
    let codex = json("plugins/yarr/.codex-plugin/plugin.json");
    let gemini = json("plugins/yarr/gemini-extension.json");

    assert_eq!(claude["name"], "yarr");
    assert_eq!(codex["name"], "yarr-mcp");
    assert_eq!(gemini["name"], "yarr-mcp");

    assert!(claude["repository"].as_str().unwrap().ends_with("yarr"));
    assert!(codex["repository"].as_str().unwrap().ends_with("yarr"));
    assert!(gemini["repository"].as_str().unwrap().ends_with("yarr"));

    let user_config = claude["userConfig"].as_object().unwrap();
    for key in [
        "server_url",
        "api_token",
        "yarr_services",
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
        "yarr_services",
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
    let hooks = json("plugins/yarr/hooks/hooks.json");
    let wrapper = std::path::Path::new("plugins/yarr/bin/yarr");
    assert!(wrapper.is_file(), "plugin hook wrapper must be tracked");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(wrapper).unwrap().permissions().mode();
        assert_ne!(mode & 0o111, 0, "plugin hook wrapper must be executable");
    }
    for hook_name in ["SessionStart", "ConfigChange"] {
        let command = hooks["hooks"][hook_name][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert_eq!(command, "${CLAUDE_PLUGIN_ROOT}/bin/yarr setup plugin-hook");
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

fn yarr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_yarr")
}

fn setup_command(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(yarr_bin());
    cmd.env_clear()
        .env("HOME", data_dir)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("YARR_HOME", data_dir)
        .env("YARR_SERVICES", "sonarr")
        .env("YARR_SONARR_KIND", "sonarr")
        .env("YARR_SONARR_URL", "https://api.yarr.test")
        .env("YARR_SONARR_API_KEY", "yarr-secret")
        .env("YARR_MCP_PORT", "0")
        .env("YARR_MCP_TOKEN", "mcp-secret");
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
    assert!(env_file.contains("YARR_SONARR_URL=https://api.yarr.test"));
    assert!(env_file.contains("YARR_SONARR_API_KEY=yarr-secret"));
    assert!(env_file.contains("YARR_MCP_TOKEN=mcp-secret"));
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
    assert!(env_file.contains("YARR_SONARR_URL=https://api.yarr.test"));
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
//   - `setup_command` sets YARR_MCP_TOKEN, which normally selects bearer
//     mode.  We override that by adding YARR_MCP_AUTH_MODE=oauth.
//   - We omit YARR_MCP_TOKEN here so the setup logic enters the OAuth
//     credential-check branch (token takes precedence in bearer mode).
//   - Port is kept at 0 (from setup_command) to avoid mcp_port_in_use noise.

fn oauth_setup_command(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(yarr_bin());
    cmd.env_clear()
        .env("HOME", data_dir)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("YARR_HOME", data_dir)
        .env("YARR_SERVICES", "sonarr")
        .env("YARR_SONARR_KIND", "sonarr")
        .env("YARR_SONARR_URL", "https://api.yarr.test")
        .env("YARR_SONARR_API_KEY", "yarr-secret")
        .env("YARR_MCP_PORT", "0")
        .env("YARR_MCP_AUTH_MODE", "oauth")
        .env("YARR_MCP_PUBLIC_URL", "https://mcp.yarr.test")
        .env("YARR_MCP_GOOGLE_CLIENT_ID", "test-client-id")
        .env("YARR_MCP_GOOGLE_CLIENT_SECRET", "test-client-secret")
        .env("YARR_MCP_AUTH_ADMIN_EMAIL", "admin@yarr.test");
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
    cmd.env_remove("YARR_MCP_PUBLIC_URL");
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
    cmd.env_remove("YARR_MCP_GOOGLE_CLIENT_ID");
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
    cmd.env_remove("YARR_MCP_GOOGLE_CLIENT_SECRET");
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
    cmd.env_remove("YARR_MCP_AUTH_ADMIN_EMAIL");
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
// write a .env that includes YARR_MCP_AUTH_MODE=oauth and all four OAuth
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
        env_file.contains("YARR_MCP_AUTH_MODE=oauth"),
        ".env should contain YARR_MCP_AUTH_MODE=oauth"
    );
    assert!(
        env_file.contains("YARR_MCP_PUBLIC_URL=https://mcp.yarr.test"),
        ".env should contain YARR_MCP_PUBLIC_URL"
    );
    assert!(
        env_file.contains("YARR_MCP_GOOGLE_CLIENT_ID=test-client-id"),
        ".env should contain YARR_MCP_GOOGLE_CLIENT_ID"
    );
    assert!(
        env_file.contains("YARR_MCP_GOOGLE_CLIENT_SECRET=test-client-secret"),
        ".env should contain YARR_MCP_GOOGLE_CLIENT_SECRET"
    );
    assert!(
        env_file.contains("YARR_MCP_AUTH_ADMIN_EMAIL=admin@yarr.test"),
        ".env should contain YARR_MCP_AUTH_ADMIN_EMAIL"
    );
    assert_env_file_mode(&data_dir.join(".env"));
}
