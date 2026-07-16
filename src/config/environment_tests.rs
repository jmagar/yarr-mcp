use super::super::{Config, env_bool, env_list};
use super::*;
use crate::testing::TestEnv;

mod dotenv {
    use super::*;

    #[test]
    fn injectable_env_key_allowlist_is_namespace_scoped() {
        for key in [
            "YARR_SERVICES",
            "YARR_SONARR_API_KEY",
            "RUSTARR_SERVICES",
            "RUST_LOG",
        ] {
            assert!(is_injectable_env_key(key), "{key} should be allowed");
        }
        for key in [
            "PATH",
            "LD_PRELOAD",
            "SSL_CERT_FILE",
            "HOME",
            "RUST_BACKTRACE",
        ] {
            assert!(!is_injectable_env_key(key), "{key} must not be injectable");
        }
    }

    #[test]
    fn load_dotenv_returns_only_allowed_overlay_values_without_mutating_process_env() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "ZZINJECT_REVIEW_TEST=danger\nYARR_REVIEW_INJECT_OK=safe\n",
        )
        .unwrap();
        let mut env = TestEnv::new();
        env.set("YARR_HOME", dir.path());
        env.remove("ZZINJECT_REVIEW_TEST");
        env.remove("YARR_REVIEW_INJECT_OK");

        let overlay = load_dotenv_defaults().unwrap();
        assert!(std::env::var_os("ZZINJECT_REVIEW_TEST").is_none());
        assert!(std::env::var_os("YARR_REVIEW_INJECT_OK").is_none());
        assert_eq!(
            overlay.get("YARR_REVIEW_INJECT_OK").map(String::as_str),
            Some("safe")
        );
    }

    #[test]
    fn load_dotenv_migrates_legacy_keys_and_ignores_dangerous_keys() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "YARR_SERVICES=sonarr\nRUSTARR_NOAUTH=true\nPATH=/tmp/evil\nLD_PRELOAD=/tmp/evil.so\nRUST_LOG=debug\n",
        )
        .unwrap();
        let mut env = TestEnv::new();
        env.set("YARR_HOME", dir.path());
        for key in [
            "YARR_SERVICES",
            "RUSTARR_NOAUTH",
            "YARR_NOAUTH",
            "LD_PRELOAD",
            "RUST_LOG",
        ] {
            env.remove(key);
        }

        let overlay = load_dotenv_defaults().unwrap();
        for key in [
            "YARR_SERVICES",
            "RUSTARR_NOAUTH",
            "YARR_NOAUTH",
            "LD_PRELOAD",
            "RUST_LOG",
        ] {
            assert!(std::env::var_os(key).is_none(), "dotenv mutated {key}");
        }
        assert_eq!(
            overlay.get("YARR_SERVICES").map(String::as_str),
            Some("sonarr")
        );
        assert_eq!(overlay.get("YARR_NOAUTH").map(String::as_str), Some("true"));
        assert_eq!(overlay.get("RUST_LOG").map(String::as_str), Some("debug"));
    }

    #[test]
    fn load_dotenv_rejects_conflicting_legacy_and_current_keys() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "YARR_NOAUTH=false\nRUSTARR_NOAUTH=true\n",
        )
        .unwrap();
        let mut env = TestEnv::new();
        env.set("YARR_HOME", dir.path());
        env.remove("YARR_NOAUTH");
        let error = load_dotenv_defaults().expect_err("mixed keys must conflict");
        assert!(
            error
                .to_string()
                .contains("conflicting values for YARR_NOAUTH")
        );
    }

    #[cfg(unix)]
    #[test]
    fn legacy_dotenv_migration_writes_private_env_file() {
        use std::os::unix::fs::PermissionsExt;

        let home = tempfile::tempdir().unwrap();
        let data_dir = home.path().join(".yarr");
        let legacy_dir = home.path().join(".rustarr");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        std::fs::write(
            legacy_dir.join(".env"),
            "RUSTARR_SERVICES=sonarr\nRUSTARR_SONARR_API_KEY=secret\n",
        )
        .unwrap();
        let mut env = TestEnv::new();
        env.set("HOME", home.path());

        migrate_legacy_dotenv(&data_dir).unwrap();
        let path = data_dir.join(".env");
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let migrated = std::fs::read_to_string(path).unwrap();
        assert!(migrated.contains("YARR_SERVICES=sonarr"));
        assert!(migrated.contains("YARR_SONARR_API_KEY=secret"));
    }

    #[test]
    fn load_dotenv_rejects_null_bytes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".env"), b"YARR_REVIEW_NULL_TEST=ab\0cd\n").unwrap();
        let mut env = TestEnv::new();
        env.set("YARR_HOME", dir.path());
        env.remove("YARR_REVIEW_NULL_TEST");
        assert!(Config::load().is_err());
    }
}

mod env_helpers {
    use super::*;

    fn call_env_bool(key: &str, raw: &str) -> anyhow::Result<bool> {
        let mut env = TestEnv::new();
        env.set(key, raw);
        let mut target = false;
        env_bool(key, &mut target)?;
        Ok(target)
    }

    fn call_env_list(key: &str, raw: &str, initial: &[&str]) -> Vec<String> {
        let mut env = TestEnv::new();
        env.set(key, raw);
        let mut target = initial.iter().map(|value| (*value).to_owned()).collect();
        env_list(key, &mut target);
        target
    }

    #[test]
    fn env_bool_accepts_documented_boolean_spellings() {
        for (index, raw, expected) in [
            (0, "1", true),
            (1, "true", true),
            (2, "yes", true),
            (3, "0", false),
            (4, "false", false),
            (5, "no", false),
        ] {
            let key = format!("YARR_TEST_ENV_BOOL_{index}");
            assert_eq!(call_env_bool(&key, raw).unwrap(), expected, "raw={raw}");
        }
    }

    #[test]
    fn env_bool_rejects_invalid_values() {
        assert!(call_env_bool("YARR_TEST_ENV_BOOL_INVALID", "maybe").is_err());
    }

    #[test]
    fn env_list_splits_and_trims_comma_separated_values() {
        assert_eq!(
            call_env_list("YARR_TEST_ENV_LIST_CSV", "a,b,c", &[]),
            ["a", "b", "c"]
        );
        assert_eq!(
            call_env_list("YARR_TEST_ENV_LIST_SPACES", "foo , bar , baz", &[]),
            ["foo", "bar", "baz"]
        );
    }

    #[test]
    fn env_list_empty_string_preserves_the_existing_target() {
        assert_eq!(
            call_env_list("YARR_TEST_ENV_LIST_EMPTY", "", &["existing"]),
            ["existing"]
        );
    }
}

mod loading {
    use super::*;

    #[test]
    fn load_reads_dotenv_without_overriding_process_env() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "YARR_SERVICES=sonarr\nYARR_SONARR_URL=https://sonarr.local\nYARR_SONARR_API_KEY=from-file\nYARR_MCP_TOKEN=from-file\n",
        )
        .unwrap();
        let mut env = TestEnv::new();
        env.set("YARR_HOME", dir.path());
        env.remove("YARR_SERVICES");
        env.remove("YARR_SONARR_URL");
        env.set("YARR_SONARR_API_KEY", "from-env");
        env.remove("YARR_MCP_TOKEN");

        let loaded = Config::load().unwrap();
        assert!(std::env::var_os("YARR_MCP_TOKEN").is_none());
        assert_eq!(loaded.yarr.services.len(), 1);
        assert_eq!(loaded.yarr.services[0].base_url, "https://sonarr.local");
        assert_eq!(loaded.yarr.services[0].api_key.as_deref(), Some("from-env"));
        assert_eq!(loaded.mcp.api_token.as_deref(), Some("from-file"));
    }

    #[test]
    fn load_falls_back_to_legacy_rustarr_config() {
        let home = tempfile::tempdir().unwrap();
        let legacy = home.path().join(".rustarr");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(
            legacy.join("config.toml"),
            "[mcp]\nport = 40123\nserver_name = \"legacy-yarr\"\n\n[[rustarr.services]]\nname = \"sonarr\"\nkind = \"sonarr\"\nbase_url = \"https://sonarr.legacy\"\napi_key = \"legacy-key\"\n",
        )
        .unwrap();
        let mut env = TestEnv::new();
        env.set("HOME", home.path());
        env.remove("YARR_HOME");
        env.remove("YARR_CONFIG");
        env.remove("YARR_MCP_PORT");

        let loaded = Config::load().unwrap();
        assert_eq!(loaded.mcp.port, 40123);
        assert_eq!(loaded.mcp.server_name, "legacy-yarr");
        assert_eq!(loaded.yarr.services[0].base_url, "https://sonarr.legacy");
        assert_eq!(
            loaded.yarr.services[0].api_key.as_deref(),
            Some("legacy-key")
        );
    }

    #[test]
    fn load_rejects_conflicting_legacy_and_current_config_sections() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.toml");
        std::fs::write(
            &config,
            "[rustarr]\nservices = []\n\n[yarr]\nservices = []\n",
        )
        .unwrap();
        let mut env = TestEnv::new();
        env.set("YARR_CONFIG", &config);
        env.remove("YARR_HOME");
        assert!(Config::load().is_err());
    }

    #[test]
    fn load_accepts_current_service_environment() {
        let dir = tempfile::tempdir().unwrap();
        let mut env = TestEnv::new();
        env.set("YARR_HOME", dir.path());
        env.set("YARR_SERVICES", "sonarr");
        env.set("YARR_SONARR_KIND", "sonarr");
        env.set("YARR_SONARR_URL", "https://api.yarr.test");
        env.set("YARR_SONARR_API_KEY", "secret");

        let loaded = Config::load().unwrap();
        assert_eq!(loaded.yarr.services.len(), 1);
        assert_eq!(loaded.yarr.services[0].name, "sonarr");
        assert_eq!(loaded.yarr.services[0].base_url, "https://api.yarr.test");
        assert_eq!(loaded.yarr.services[0].api_key.as_deref(), Some("secret"));
    }

    #[test]
    fn load_migrates_legacy_environment_when_current_keys_are_absent() {
        let dir = tempfile::tempdir().unwrap();
        let mut env = TestEnv::new();
        env.set("YARR_HOME", dir.path());
        env.set("RUSTARR_SERVICES", "sonarr");
        env.set("RUSTARR_SONARR_KIND", "sonarr");
        env.set("RUSTARR_SONARR_URL", "https://legacy.yarr.test");
        env.set("RUSTARR_SONARR_API_KEY", "legacy-secret");
        for key in [
            "YARR_SERVICES",
            "YARR_SONARR_KIND",
            "YARR_SONARR_URL",
            "YARR_SONARR_API_KEY",
        ] {
            env.remove(key);
        }

        let loaded = Config::load().unwrap();
        assert_eq!(loaded.yarr.services[0].base_url, "https://legacy.yarr.test");
        assert_eq!(
            loaded.yarr.services[0].api_key.as_deref(),
            Some("legacy-secret")
        );
    }

    #[test]
    fn load_rejects_conflicting_legacy_and_current_environment() {
        let mut env = TestEnv::new();
        env.set("RUSTARR_SERVICES", "sonarr");
        env.set("YARR_SERVICES", "radarr");
        let error = Config::load().expect_err("conflicting namespaces must fail");
        assert!(error.to_string().contains("conflicting legacy env"));
    }
}
