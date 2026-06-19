use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::expect_success;
use crate::live::mcporter::{FIXTURE_PORT, SAB_FIXTURE_NZB, call_tool, report};

pub(super) struct FixtureServer {
    base_url: String,
}

pub(super) fn start_fixture_server() -> Result<FixtureServer> {
    let host =
        std::env::var("RUSTARR_LIVE_FIXTURE_HOST").unwrap_or_else(|_| "100.88.16.79".to_string());
    let listener = TcpListener::bind(("0.0.0.0", FIXTURE_PORT)).with_context(|| {
        format!(
            "failed to bind live fixture server on 0.0.0.0:{FIXTURE_PORT}; \
             set RUSTARR_LIVE_FIXTURE_HOST to a host/IP reachable from shart"
        )
    })?;
    let body = Arc::new(SAB_FIXTURE_NZB.as_bytes().to_vec());
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let body = Arc::clone(&body);
            thread::spawn(move || {
                let _ = serve_fixture_request(stream, &body);
            });
        }
    });
    Ok(FixtureServer {
        base_url: format!("http://{host}:{FIXTURE_PORT}"),
    })
}

fn serve_fixture_request(mut stream: std::net::TcpStream, body: &[u8]) -> Result<()> {
    let mut buf = [0_u8; 1024];
    let _ = stream.read(&mut buf);
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/x-nzb\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    let _ = stream.flush();
    Ok(())
}

pub(super) fn run_sabnzbd_lifecycle(
    report: &mut report::Report,
    mcp_url: &str,
    fixture: &FixtureServer,
) -> Result<()> {
    cleanup_sabnzbd_queue(mcp_url)?;
    let url = format!("{}/rustarr-live-test.nzb", fixture.base_url);
    let added = expect_success(
        "sabnzbd",
        "download_add",
        call_tool(
            mcp_url,
            "sabnzbd",
            &json!({ "action": "download_add", "url": url, "confirm": true }),
        )?,
    )?;
    if added.get("status").and_then(Value::as_bool) != Some(true) {
        bail!("SABnzbd download_add did not report status=true: {added}");
    }
    let id = added
        .get("nzo_ids")
        .and_then(Value::as_array)
        .and_then(|ids| ids.first())
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("SABnzbd download_add did not return nzo_ids: {added}"))?
        .to_string();
    wait_for_sab_job(mcp_url, &id, true)?;

    for action in ["download_pause", "download_resume"] {
        let value = expect_success(
            "sabnzbd",
            action,
            call_tool(
                mcp_url,
                "sabnzbd",
                &json!({ "action": action, "id": id, "confirm": true }),
            )?,
        )?;
        assert_sab_status(&value, &id, action)?;
    }

    let removed = expect_success(
        "sabnzbd",
        "download_remove",
        call_tool(
            mcp_url,
            "sabnzbd",
            &json!({
                "action": "download_remove",
                "id": id,
                "delete_files": false,
                "confirm": true,
            }),
        )?,
    )?;
    assert_sab_status(&removed, &id, "download_remove")?;
    wait_for_sab_job(mcp_url, &id, false)?;

    report.pass(
        "mcporter confirmed write sabnzbd download lifecycle",
        "download_add/pause/resume/remove changed observable queue state and cleaned up",
    );
    Ok(())
}

fn cleanup_sabnzbd_queue(mcp_url: &str) -> Result<()> {
    let queue = sab_queue(mcp_url)?;
    for id in sab_ids(&queue).collect::<Vec<_>>() {
        let _ = call_tool(
            mcp_url,
            "sabnzbd",
            &json!({
                "action": "download_remove",
                "id": id,
                "delete_files": false,
                "confirm": true,
            }),
        )?;
    }
    Ok(())
}

fn wait_for_sab_job(mcp_url: &str, id: &str, should_exist: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut last_queue = Value::Null;
    while Instant::now() < deadline {
        let queue = sab_queue(mcp_url)?;
        let exists = sab_ids(&queue).any(|candidate| candidate == id);
        if exists == should_exist {
            return Ok(());
        }
        last_queue = queue;
        thread::sleep(Duration::from_millis(500));
    }
    let expected = if should_exist {
        "appear in"
    } else {
        "disappear from"
    };
    bail!("SABnzbd test job {id} did not {expected} queue: {last_queue}");
}

fn sab_queue(mcp_url: &str) -> Result<Value> {
    expect_success(
        "sabnzbd",
        "download_queue",
        call_tool(mcp_url, "sabnzbd", &json!({ "action": "download_queue" }))?,
    )
}

fn sab_ids(value: &Value) -> impl Iterator<Item = String> + '_ {
    value
        .get("slots")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|slot| slot.get("nzo_id").and_then(Value::as_str))
        .map(str::to_owned)
}

fn assert_sab_status(value: &Value, id: &str, action: &str) -> Result<()> {
    if value.get("status").and_then(Value::as_bool) != Some(true) {
        bail!("SABnzbd {action} did not report status=true: {value}");
    }
    let has_id = value
        .get("nzo_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|candidate| candidate.as_str() == Some(id));
    if !has_id {
        bail!("SABnzbd {action} response did not include nzo id {id}: {value}");
    }
    Ok(())
}

pub(super) fn run_qbittorrent_lifecycle(report: &mut report::Report, mcp_url: &str) -> Result<()> {
    let hash = "1111111111111111111111111111111111111111";
    let magnet = format!("magnet:?xt=urn:btih:{hash}&dn=rustarr-live-mcporter-stateful");

    let _ = call_tool(
        mcp_url,
        "qbittorrent",
        &json!({
            "action": "download_remove",
            "id": hash,
            "delete_files": false,
            "confirm": true,
        }),
    )?;

    let add = expect_success(
        "qbittorrent",
        "download_add",
        call_tool(
            mcp_url,
            "qbittorrent",
            &json!({
                "action": "download_add",
                "url": magnet,
                "confirm": true,
            }),
        )?,
    )?;
    if !add.to_string().contains("Ok") {
        bail!("qBittorrent add did not return expected accepted response: {add}");
    }
    wait_for_torrent_presence(mcp_url, hash, true)?;

    for action in ["download_pause", "download_resume"] {
        let value = expect_success(
            "qbittorrent",
            action,
            call_tool(
                mcp_url,
                "qbittorrent",
                &json!({ "action": action, "id": hash, "confirm": true }),
            )?,
        )?;
        if value.get("submitted").and_then(Value::as_bool) != Some(true) {
            bail!("qBittorrent {action} did not report submitted=true: {value}");
        }
        wait_for_torrent_presence(mcp_url, hash, true)?;
    }

    let removed = expect_success(
        "qbittorrent",
        "download_remove",
        call_tool(
            mcp_url,
            "qbittorrent",
            &json!({
                "action": "download_remove",
                "id": hash,
                "delete_files": false,
                "confirm": true,
            }),
        )?,
    )?;
    if removed.get("submitted").and_then(Value::as_bool) != Some(true) {
        bail!("qBittorrent remove did not report submitted=true: {removed}");
    }
    wait_for_torrent_presence(mcp_url, hash, false)?;

    report.pass(
        "mcporter confirmed write lifecycle qbittorrent torrent",
        "download_add/pause/resume/remove changed observable queue state and cleaned up",
    );
    Ok(())
}

fn wait_for_torrent_presence(mcp_url: &str, hash: &str, should_exist: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut last_queue = Value::Null;
    while Instant::now() < deadline {
        let queue = expect_success(
            "qbittorrent",
            "download_queue",
            call_tool(
                mcp_url,
                "qbittorrent",
                &json!({ "action": "download_queue" }),
            )?,
        )?;
        let exists = torrent_hashes(&queue).any(|candidate| candidate.eq_ignore_ascii_case(hash));
        if exists == should_exist {
            return Ok(());
        }
        last_queue = queue;
        thread::sleep(Duration::from_millis(500));
    }
    let expected = if should_exist {
        "appear in"
    } else {
        "disappear from"
    };
    bail!("qBittorrent test torrent {hash} did not {expected} queue: {last_queue}");
}

fn torrent_hashes(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|torrent| torrent.get("hash").and_then(Value::as_str))
}

pub(super) fn run_tautulli_maintenance_lifecycle(
    report: &mut report::Report,
    mcp_url: &str,
) -> Result<()> {
    for (action, field) in [
        ("stats_refresh_libraries", "refreshed"),
        ("stats_refresh_users", "refreshed"),
        ("stats_delete_image_cache", "cleared"),
    ] {
        let value = expect_success(
            "tautulli",
            action,
            call_tool(
                mcp_url,
                "tautulli",
                &json!({ "action": action, "confirm": true }),
            )?,
        )?;
        if value.get("submitted").and_then(Value::as_bool) != Some(true) {
            bail!("tautulli {action} did not report submitted=true: {value}");
        }
        if !value.get(field).map(Value::is_boolean).unwrap_or(false) {
            bail!("tautulli {action} did not include boolean {field}: {value}");
        }
    }
    report.pass(
        "mcporter confirmed write tautulli maintenance lifecycle",
        "refresh_libraries/refresh_users/delete_image_cache all returned submitted maintenance state",
    );
    Ok(())
}
