use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::live::guard::{validate_env, SHART_HOME};

fn good_env() -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("RUSTARR_HOME".into(), SHART_HOME.into());
    env.insert("RUSTARR_SERVICES".into(), "sonarr,radarr,prowlarr,tautulli,overseerr,bazarr,tracearr,lidarr,readarr,sabnzbd,qbittorrent,wizarr,notifiarr,plex,jellyfin".into());
    for (name, kind, port) in [
        ("SONARR", "sonarr", "8989"),
        ("RADARR", "radarr", "7878"),
        ("PROWLARR", "prowlarr", "9696"),
        ("TAUTULLI", "tautulli", "8181"),
        ("OVERSEERR", "overseerr", "5055"),
        ("BAZARR", "bazarr", "6767"),
        ("TRACEARR", "tracearr", "8686"),
        ("LIDARR", "lidarr", "8687"),
        ("READARR", "readarr", "8787"),
        ("SABNZBD", "sabnzbd", "8080"),
        ("QBITTORRENT", "qbittorrent", "8081"),
        ("WIZARR", "wizarr", "5690"),
        ("NOTIFIARR", "notifiarr", "5454"),
        ("PLEX", "plex", "32400"),
        ("JELLYFIN", "jellyfin", "8096"),
    ] {
        env.insert(
            format!("RUSTARR_{name}_URL"),
            format!("http://shart.manatee-triceratops.ts.net:{port}"),
        );
        env.insert(format!("RUSTARR_{name}_KIND"), kind.into());
    }
    env
}

#[test]
fn guard_accepts_complete_shart_env() {
    let env = good_env();
    let result = validate_env(env, false).expect("complete shart env should pass");
    assert_eq!(result.services.len(), 15);
    assert_eq!(result.kinds["sonarr"], "sonarr");
}

#[test]
fn guard_rejects_live_home() {
    let mut env = good_env();
    env.insert("RUSTARR_HOME".into(), "/home/jmagar/.rustarr".into());
    let err = validate_env(env, false).unwrap_err().to_string();
    assert!(err.contains("RUSTARR_HOME must be /home/jmagar/.rustarr-shart"));
}

#[test]
fn guard_rejects_tootie_url_override() {
    let mut env = good_env();
    env.insert("RUSTARR_SONARR_URL".into(), "https://sonarr.tootie.tv".into());
    let err = validate_env(env, false).unwrap_err().to_string();
    assert!(err.contains("is not a shart URL"));
}

#[test]
fn guard_rejects_missing_required_kind() {
    let mut env = good_env();
    env.insert("RUSTARR_SERVICES".into(), "sonarr,radarr".into());
    let err = validate_env(env, false).unwrap_err().to_string();
    assert!(err.contains("missing required service kind"));
}

#[test]
fn guard_parses_env_file() {
    let path = Path::new("target/live-test-env");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "RUSTARR_SERVICES=sonarr\nRUSTARR_SONARR_URL=http://shart.manatee-triceratops.ts.net:8989\nRUSTARR_SONARR_KIND=sonarr\n").unwrap();
    let env = crate::live::guard::read_env_file(path).unwrap();
    assert_eq!(env["RUSTARR_SONARR_KIND"], "sonarr");
}

#[test]
fn matrix_covers_all_required_service_kinds() {
    let matrix_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/live/service_matrix.json");
    let matrix = crate::live::matrix::load(&matrix_path).unwrap();
    let kinds: std::collections::BTreeSet<_> =
        matrix.services.iter().map(|service| service.kind.as_str()).collect();
    assert_eq!(kinds, crate::live::guard::required_kinds());
    for service in &matrix.services {
        assert!(
            !service.get.is_empty(),
            "{} needs at least one GET case",
            service.name
        );
        assert!(
            !service.post_expected_error.error_contains_any.is_empty(),
            "{} needs expected-error tokens",
            service.name
        );
    }
}
