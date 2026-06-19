use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::process::Command;

use super::expect_success;
use crate::live::mcporter::{call_tool, report};

pub(super) fn run_bazarr_blacklist_delete(
    report: &mut report::Report,
    mcp_url: &str,
) -> Result<()> {
    seed_bazarr_blacklist_fixture()?;
    let before = bazarr_blacklist_fixture_count()?;
    if before == 0 {
        bail!("bazarr fixture seeding did not create a blacklist row");
    }
    let deleted = expect_success(
        "bazarr",
        "api_delete movie blacklist cleanup",
        call_tool(
            mcp_url,
            "bazarr",
            &json!({
                "action": "api_delete",
                "path": "/api/movies/blacklist?all=true",
                "confirm": true,
            }),
        )?,
    )?;
    let accepted_empty_success = deleted.as_str() == Some("");
    if deleted.get("ok").and_then(Value::as_bool) != Some(true) && !accepted_empty_success {
        bail!("bazarr blacklist delete did not report ok=true or empty-string success: {deleted}");
    }
    let after = bazarr_blacklist_fixture_count()?;
    if after != 0 {
        bail!("bazarr blacklist fixture rows remained after delete: before={before} after={after}");
    }
    report.pass(
        "mcporter confirmed write bazarr blacklist delete",
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

pub(super) fn run_tracearr_debug_delete(report: &mut report::Report, mcp_url: &str) -> Result<()> {
    seed_tracearr_session_fixture()?;
    let before = tracearr_session_fixture_count()?;
    if before == 0 {
        bail!("tracearr fixture seeding did not create a session row");
    }
    let deleted = expect_success(
        "tracearr",
        "api_delete debug sessions",
        call_tool(
            mcp_url,
            "tracearr",
            &json!({
                "action": "api_delete",
                "path": "/api/v1/debug/sessions",
                "confirm": true,
            }),
        )?,
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
        "mcporter confirmed write tracearr debug sessions delete",
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
