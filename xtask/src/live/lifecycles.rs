//! Confirmed stateful write lifecycles for the doc-based services whose curated
//! commands are not generated OpenAPI ops: SABnzbd / qBittorrent `download_*`,
//! Tautulli `stats_*` maintenance, and Bazarr / Tracearr seeded `api_delete`
//! cleanup. These drive the CURRENT surface — service-grouped CLI verbs
//! (`rustarr <svc> add|pause|resume|remove|...`) — and assert observable
//! before/after state, then clean up. (Logic recovered from the retired mcporter
//! suite; only the invocation moved from per-service MCP tools to the CLI.)
//!
//! Destructive by nature (queue removes, blacklist/session deletes), so the suite
//! is skipped under `--no-destructive`. It runs only against the disposable shart
//! stack. SABnzbd needs an NZB fixture URL reachable from shart — a tiny in-process
//! HTTP server on `RUSTARR_LIVE_FIXTURE_HOST` (default the dookie tailnet IP).
//! Bazarr/Tracearr seeding runs over `ssh shart docker exec`.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::{process, report};

const FIXTURE_PORT: u16 = 40175;

const SAB_FIXTURE_NZB: &str = r#"<?xml version="1.0" encoding="iso-8859-1" ?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="rustarr@example.invalid" date="1710000000" subject="rustarr-live-test">
    <groups><group>alt.binaries.test</group></groups>
    <segments><segment bytes="1" number="1">rustarr-live-test@example.invalid</segment></segments>
  </file>
</nzb>
"#;

pub(super) fn run(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
    no_destructive: bool,
) -> Result<()> {
    if no_destructive {
        return Ok(());
    }
    let fixture = start_fixture_server()?;
    run_sabnzbd_lifecycle(report, rustarr, &fixture)?;
    run_qbittorrent_lifecycle(report, rustarr)?;
    run_tautulli_maintenance_lifecycle(report, rustarr)?;
    run_bazarr_blacklist_delete(report, rustarr)?;
    run_tracearr_debug_delete(report, rustarr)?;
    Ok(())
}

/// Run `rustarr <args>` and parse stdout as JSON, bailing on a non-zero exit.
fn cli(rustarr: &process::RustarrProcess, args: &[&str]) -> Result<Value> {
    rustarr.json(args)
}

// ── SABnzbd download lifecycle ──────────────────────────────────────────────────

struct FixtureServer {
    base_url: String,
}

fn start_fixture_server() -> Result<FixtureServer> {
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

fn run_sabnzbd_lifecycle(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
    fixture: &FixtureServer,
) -> Result<()> {
    cleanup_sabnzbd_queue(rustarr)?;
    let url = format!("{}/rustarr-live-test.nzb", fixture.base_url);
    let added = cli(rustarr, &["sabnzbd", "add", "--url", &url, "--confirm"])?;
    if added.get("status").and_then(Value::as_bool) != Some(true) {
        bail!("SABnzbd add did not report status=true: {added}");
    }
    let id = added
        .get("nzo_ids")
        .and_then(Value::as_array)
        .and_then(|ids| ids.first())
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("SABnzbd add did not return nzo_ids: {added}"))?
        .to_string();
    wait_for_sab_job(rustarr, &id, true)?;

    for verb in ["pause", "resume"] {
        let value = cli(rustarr, &["sabnzbd", verb, "--id", &id, "--confirm"])?;
        assert_sab_status(&value, &id, verb)?;
    }

    let removed = cli(rustarr, &["sabnzbd", "remove", "--id", &id, "--confirm"])?;
    assert_sab_status(&removed, &id, "remove")?;
    wait_for_sab_job(rustarr, &id, false)?;

    report.pass(
        "lifecycle sabnzbd download",
        "add/pause/resume/remove changed observable queue state and cleaned up",
    );
    Ok(())
}

fn cleanup_sabnzbd_queue(rustarr: &process::RustarrProcess) -> Result<()> {
    let queue = sab_queue(rustarr)?;
    for id in sab_ids(&queue).collect::<Vec<_>>() {
        let _ = cli(rustarr, &["sabnzbd", "remove", "--id", &id, "--confirm"]);
    }
    Ok(())
}

fn wait_for_sab_job(rustarr: &process::RustarrProcess, id: &str, should_exist: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut last_queue = Value::Null;
    while Instant::now() < deadline {
        let queue = sab_queue(rustarr)?;
        if sab_ids(&queue).any(|candidate| candidate == id) == should_exist {
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

fn sab_queue(rustarr: &process::RustarrProcess) -> Result<Value> {
    cli(rustarr, &["sabnzbd", "queue"])
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

fn assert_sab_status(value: &Value, id: &str, verb: &str) -> Result<()> {
    if value.get("status").and_then(Value::as_bool) != Some(true) {
        bail!("SABnzbd {verb} did not report status=true: {value}");
    }
    let has_id = value
        .get("nzo_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|candidate| candidate.as_str() == Some(id));
    if !has_id {
        bail!("SABnzbd {verb} response did not include nzo id {id}: {value}");
    }
    Ok(())
}

// ── qBittorrent download lifecycle ──────────────────────────────────────────────

fn run_qbittorrent_lifecycle(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
) -> Result<()> {
    let hash = "1111111111111111111111111111111111111111";
    let magnet = format!("magnet:?xt=urn:btih:{hash}&dn=rustarr-live-stateful");

    // Best-effort pre-clean (ignore errors if the torrent is not present).
    let _ = cli(
        rustarr,
        &["qbittorrent", "remove", "--hash", hash, "--confirm"],
    );

    let add = cli(
        rustarr,
        &["qbittorrent", "add", "--magnet", &magnet, "--confirm"],
    )?;
    if !add.to_string().contains("Ok") {
        bail!("qBittorrent add did not return expected accepted response: {add}");
    }
    wait_for_torrent_presence(rustarr, hash, true)?;

    for verb in ["pause", "resume"] {
        let value = cli(rustarr, &["qbittorrent", verb, "--hash", hash, "--confirm"])?;
        if value.get("submitted").and_then(Value::as_bool) != Some(true) {
            bail!("qBittorrent {verb} did not report submitted=true: {value}");
        }
        wait_for_torrent_presence(rustarr, hash, true)?;
    }

    let removed = cli(
        rustarr,
        &["qbittorrent", "remove", "--hash", hash, "--confirm"],
    )?;
    if removed.get("submitted").and_then(Value::as_bool) != Some(true) {
        bail!("qBittorrent remove did not report submitted=true: {removed}");
    }
    wait_for_torrent_presence(rustarr, hash, false)?;

    report.pass(
        "lifecycle qbittorrent download",
        "add/pause/resume/remove changed observable queue state and cleaned up",
    );
    Ok(())
}

fn wait_for_torrent_presence(
    rustarr: &process::RustarrProcess,
    hash: &str,
    should_exist: bool,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut last_queue = Value::Null;
    while Instant::now() < deadline {
        let queue = cli(rustarr, &["qbittorrent", "queue"])?;
        if torrent_hashes(&queue).any(|candidate| candidate.eq_ignore_ascii_case(hash))
            == should_exist
        {
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

// ── Tautulli maintenance lifecycle ──────────────────────────────────────────────

fn run_tautulli_maintenance_lifecycle(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
) -> Result<()> {
    // refresh-* are non-destructive (run immediately); delete-image-cache is
    // destructive and needs --confirm.
    for (verb, field, confirm) in [
        ("refresh-libraries", "refreshed", false),
        ("refresh-users", "refreshed", false),
        ("delete-image-cache", "cleared", true),
    ] {
        let mut args = vec!["tautulli", verb];
        if confirm {
            args.push("--confirm");
        }
        let value = cli(rustarr, &args)?;
        if value.get("submitted").and_then(Value::as_bool) != Some(true) {
            bail!("tautulli {verb} did not report submitted=true: {value}");
        }
        if !value.get(field).map(Value::is_boolean).unwrap_or(false) {
            bail!("tautulli {verb} did not include boolean {field}: {value}");
        }
    }
    report.pass(
        "lifecycle tautulli maintenance",
        "refresh-libraries/refresh-users/delete-image-cache returned submitted maintenance state",
    );
    Ok(())
}

// ── Bazarr / Tracearr seeded api_delete cleanup ─────────────────────────────────

fn run_bazarr_blacklist_delete(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
) -> Result<()> {
    seed_bazarr_blacklist_fixture()?;
    let before = bazarr_blacklist_fixture_count()?;
    if before == 0 {
        bail!("bazarr fixture seeding did not create a blacklist row");
    }
    let deleted = cli(
        rustarr,
        &[
            "bazarr",
            "delete",
            "--path",
            "/api/movies/blacklist?all=true",
            "--confirm",
        ],
    )?;
    let accepted_empty = deleted.as_str() == Some("");
    if deleted.get("ok").and_then(Value::as_bool) != Some(true) && !accepted_empty {
        bail!("bazarr blacklist delete did not report ok=true or empty-string success: {deleted}");
    }
    let after = bazarr_blacklist_fixture_count()?;
    if after != 0 {
        bail!("bazarr blacklist fixture rows remained after delete: before={before} after={after}");
    }
    report.pass(
        "lifecycle bazarr blacklist delete",
        format!("api_delete removed {before} seeded movie blacklist row(s)"),
    );
    Ok(())
}

fn seed_bazarr_blacklist_fixture() -> Result<()> {
    run_ssh_shart(
        r#"docker exec -i bazarr python3 <<'PY'
import datetime
import sqlite3

con = sqlite3.connect("/config/db/bazarr.db")
con.execute("delete from table_blacklist_movie where provider=? or subs_id=?", ("rustarr-live", "rustarr-live-sub"))
con.execute(
    "insert into table_blacklist_movie(language, provider, radarr_id, subs_id, timestamp) values (?,?,?,?,?)",
    ("en", "rustarr-live", 999001, "rustarr-live-sub", datetime.datetime.now(datetime.UTC).isoformat()),
)
con.commit()
PY"#,
    )?;
    Ok(())
}

fn bazarr_blacklist_fixture_count() -> Result<i64> {
    let output = run_ssh_shart(
        r#"docker exec -i bazarr python3 <<'PY'
import sqlite3
con = sqlite3.connect("/config/db/bazarr.db")
print(con.execute("select count(*) from table_blacklist_movie where provider=? or subs_id=?", ("rustarr-live", "rustarr-live-sub")).fetchone()[0])
PY"#,
    )?;
    parse_i64_output(&output, "bazarr blacklist fixture count")
}

fn run_tracearr_debug_delete(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
) -> Result<()> {
    seed_tracearr_session_fixture()?;
    let before = tracearr_session_fixture_count()?;
    if before == 0 {
        bail!("tracearr fixture seeding did not create a session row");
    }
    let deleted = cli(
        rustarr,
        &[
            "tracearr",
            "delete",
            "--path",
            "/api/v1/debug/sessions",
            "--confirm",
        ],
    )?;
    let deleted_sessions = deleted
        .get("deleted")
        .and_then(|deleted| deleted.get("sessions"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if deleted_sessions < before {
        bail!(
            "tracearr debug delete removed {deleted_sessions}, expected at least {before}: {deleted}"
        );
    }
    let after = tracearr_session_fixture_count()?;
    if after != 0 {
        bail!("tracearr session fixture rows remained after delete: before={before} after={after}");
    }
    report.pass(
        "lifecycle tracearr debug delete",
        format!("api_delete removed {deleted_sessions} seeded session row(s)"),
    );
    Ok(())
}

fn seed_tracearr_session_fixture() -> Result<()> {
    run_ssh_shart(
        r#"docker exec -i tracearr-db psql -U tracearr -d tracearr -v ON_ERROR_STOP=1 <<'SQL'
insert into users (id, username, name, email, role) values ('00000000-0000-4000-8000-000000000001','rustarr-fixture','Rustarr Fixture','rustarr-fixture@example.invalid','member') on conflict (email) do update set username=excluded.username;
insert into servers (id, name, type, url, token) values ('00000000-0000-4000-8000-000000000002','Rustarr Fixture Server','plex','http://example.invalid','fixture-token') on conflict (id) do update set name=excluded.name;
insert into server_users (id, user_id, server_id, external_id, username) values ('00000000-0000-4000-8000-000000000003','00000000-0000-4000-8000-000000000001','00000000-0000-4000-8000-000000000002','rustarr-fixture-user','rustarr-fixture') on conflict (id) do update set username=excluded.username;
delete from sessions where id='00000000-0000-4000-8000-000000000004';
insert into sessions (id, server_id, server_user_id, session_key, state, media_type, media_title, ip_address, last_seen_at, started_at) values ('00000000-0000-4000-8000-000000000004','00000000-0000-4000-8000-000000000002','00000000-0000-4000-8000-000000000003','rustarr-fixture-session','playing','movie','Rustarr Fixture Movie','127.0.0.1', now(), now());
SQL"#,
    )?;
    Ok(())
}

fn tracearr_session_fixture_count() -> Result<i64> {
    let output = run_ssh_shart(
        r#"docker exec tracearr-db psql -U tracearr -d tracearr -tAc "select count(*) from sessions where id='00000000-0000-4000-8000-000000000004'""#,
    )?;
    parse_i64_output(&output, "tracearr session fixture count")
}

fn run_ssh_shart(command: &str) -> Result<String> {
    let output = Command::new("ssh")
        .arg("shart")
        .arg(command)
        .output()
        .with_context(|| format!("failed to run ssh shart {command}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !output.status.success() {
        bail!("ssh shart {command} failed: {stdout}{stderr}");
    }
    Ok(stdout)
}

fn parse_i64_output(output: &str, label: &str) -> Result<i64> {
    output
        .lines()
        .rev()
        .find_map(|line| line.trim().parse::<i64>().ok())
        .ok_or_else(|| anyhow::anyhow!("{label} did not return an integer: {output}"))
}

#[cfg(test)]
#[path = "lifecycles_tests.rs"]
mod tests;
