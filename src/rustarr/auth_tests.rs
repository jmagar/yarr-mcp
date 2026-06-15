use super::*;
use crate::config::{ServiceConfig, ServiceKind};

fn svc(kind: ServiceKind) -> ServiceConfig {
    ServiceConfig {
        name: kind.as_str().into(),
        kind,
        base_url: "http://localhost:8989".into(),
        api_key: Some("key".into()),
        token: Some("tok".into()),
        ..ServiceConfig::default()
    }
}

/// Build a request through `apply_auth` and return its header map for inspection.
fn headers_for(service: &ServiceConfig) -> reqwest::header::HeaderMap {
    let client = reqwest::Client::new();
    let builder = client.get("http://localhost:8989/x");
    let request = apply_auth(builder, service)
        .build()
        .expect("request should build");
    request.headers().clone()
}

#[test]
fn arr_uses_x_api_key_header() {
    let h = headers_for(&svc(ServiceKind::Sonarr));
    assert_eq!(h.get("X-Api-Key").unwrap(), "key");
}

#[test]
fn api_key_header_kind_never_gets_bearer() {
    // S4: an ApiKeyHeader service with BOTH api_key and token must NOT leak an
    // Authorization: Bearer header alongside the X-Api-Key credential.
    let h = headers_for(&svc(ServiceKind::Sonarr));
    assert_eq!(h.get("X-Api-Key").unwrap(), "key");
    assert!(
        h.get(reqwest::header::AUTHORIZATION).is_none(),
        "ApiKeyHeader kind must not send an Authorization header even when token is set"
    );
}

#[test]
fn plex_has_no_authorization_bearer() {
    // S4: Plex token travels in the query string only — never as a bearer.
    let h = headers_for(&svc(ServiceKind::Plex));
    assert!(
        h.get(reqwest::header::AUTHORIZATION).is_none(),
        "plex must not send an Authorization header"
    );
    assert!(h.get("X-Api-Key").is_none());
}

#[test]
fn jellyfin_uses_mediabrowser_not_bearer() {
    // S4: Jellyfin must use the quoted MediaBrowser form, not Authorization:Bearer.
    let h = headers_for(&svc(ServiceKind::Jellyfin));
    let authz = h
        .get(reqwest::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap();
    assert_eq!(authz, r#"MediaBrowser Token="key""#);
    assert!(!authz.starts_with("Bearer"), "must not be a bearer token");
    // X-Emby-Token fallback is still present.
    assert_eq!(h.get("X-Emby-Token").unwrap(), "key");
}

#[test]
fn accepts_qbittorrent_login_success_variants() {
    use reqwest::StatusCode;
    assert!(qbittorrent_login_accepted(StatusCode::OK, "Ok."));
    assert!(qbittorrent_login_accepted(StatusCode::OK, " Ok.\n"));
    assert!(qbittorrent_login_accepted(StatusCode::NO_CONTENT, ""));
    assert!(!qbittorrent_login_accepted(StatusCode::OK, "Fails."));
    assert!(!qbittorrent_login_accepted(StatusCode::UNAUTHORIZED, "Ok."));
}

/// S1: after a qBittorrent login sets a SID cookie, a request to a *different*
/// service on the *same host* must NOT carry that cookie. The shared client has
/// no cookie jar; only the dedicated qbit client does.
#[tokio::test]
async fn qbit_cookie_does_not_bleed_to_other_service() {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel::<Vec<String>>();

    // Serve: (1) qbit login -> Set-Cookie SID, (2) qbit app/version,
    // (3) sonarr request. Record each request's header lines.
    let handle = std::thread::spawn(move || {
        for i in 0..3 {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut header_lines = Vec::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 {
                    break;
                }
                if line == "\r\n" || line == "\n" {
                    break;
                }
                header_lines.push(line.trim_end().to_string());
            }
            // Drain a possible request body so the socket closes cleanly.
            let mut sink = [0_u8; 256];
            let _ = reader
                .get_mut()
                .set_read_timeout(Some(std::time::Duration::from_millis(20)));
            let _ = reader.get_mut().read(&mut sink);
            tx.send(header_lines).unwrap();

            let (status, extra, body) = if i == 0 {
                ("200 OK", "Set-Cookie: SID=secret-sid; path=/\r\n", "Ok.")
            } else {
                ("200 OK", "", "{\"ok\":true}")
            };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{extra}Content-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            let _ = stream.flush();
        }
    });

    let base = format!("http://{addr}");
    let qbit = ServiceConfig {
        name: "qbittorrent".into(),
        kind: ServiceKind::Qbittorrent,
        base_url: base.clone(),
        username: Some("u".into()),
        password: Some("p".into()),
        ..ServiceConfig::default()
    };
    let sonarr = ServiceConfig {
        name: "sonarr".into(),
        kind: ServiceKind::Sonarr,
        base_url: base,
        api_key: Some("key".into()),
        ..ServiceConfig::default()
    };
    let config = crate::config::RustarrConfig {
        services: vec![qbit.clone(), sonarr.clone()],
    };
    let client = crate::rustarr::RustarrClient::new(&config).unwrap();

    // qbit request: triggers login (req 0) + app/version (req 1).
    let _ = client.get_json(&qbit, "/api/v2/app/version").await;
    // non-qbit request on the same host (req 2).
    let _ = client.get_json(&sonarr, "/api/v3/system/status").await;

    let login = rx.recv().unwrap();
    let qbit_call = rx.recv().unwrap();
    let sonarr_call = rx.recv().unwrap();
    handle.join().unwrap();

    let has_sid = |hdrs: &[String]| {
        hdrs.iter()
            .any(|h| h.to_ascii_lowercase().starts_with("cookie:") && h.contains("SID=secret-sid"))
    };
    assert!(!has_sid(&login), "login itself sends no prior cookie");
    assert!(
        has_sid(&qbit_call),
        "qbit follow-up SHOULD carry its own SID via the dedicated cookie client"
    );
    assert!(
        !has_sid(&sonarr_call),
        "sonarr request must NOT carry the qbit SID cookie (S1)"
    );
}
