//! CLI parse module for Tracearr curated commands.

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::actions::curated_command;
use crate::capability::Capability;
use crate::cli::command::Command;
use crate::config::ServiceKind;

pub const VERBS: &[(&str, &str)] = &[
    ("health", "trace_health"),
    ("stats", "trace_stats"),
    ("today", "trace_today"),
    ("activity", "trace_activity"),
    ("streams", "trace_streams"),
    ("users", "trace_users"),
    ("violations", "trace_violations"),
    ("history", "trace_history"),
    ("terminate-stream", "trace_terminate_stream"),
];

pub fn parse(kind: ServiceKind, verb: &str, rest: &[String]) -> Result<Option<Command>> {
    let Some(action) = resolve(verb)? else {
        return Ok(None);
    };
    match verb {
        "today" => {
            parse_optional_string(kind, action, verb, rest, "--timezone", "timezone").map(Some)
        }
        "activity" => {
            parse_optional_string(kind, action, verb, rest, "--period", "period").map(Some)
        }
        "streams" => parse_streams(kind, action, rest).map(Some),
        "users" | "violations" | "history" => parse_paged(kind, action, verb, rest).map(Some),
        "terminate-stream" => parse_terminate(kind, action, rest).map(Some),
        _ => parse_simple(kind, action, verb, rest).map(Some),
    }
}

fn parse_simple(
    kind: ServiceKind,
    action: &'static str,
    verb: &str,
    rest: &[String],
) -> Result<Command> {
    if let Some(extra) = rest.first() {
        return Err(anyhow!("{verb} does not accept argument `{extra}`"));
    }
    Ok(Command::Curated {
        action,
        params: Value::Object(base_params(kind)),
    })
}

fn parse_optional_string(
    kind: ServiceKind,
    action: &'static str,
    verb: &str,
    rest: &[String],
    flag: &str,
    key: &str,
) -> Result<Command> {
    let mut params = base_params(kind);
    let mut i = 0;
    while i < rest.len() {
        if rest[i] == flag {
            params.insert(key.into(), json!(take_value(rest, &mut i, flag)?));
        } else {
            return Err(anyhow!("{verb} does not accept argument `{}`", rest[i]));
        }
        i += 1;
    }
    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

fn parse_streams(kind: ServiceKind, action: &'static str, rest: &[String]) -> Result<Command> {
    let mut params = base_params(kind);
    for arg in rest {
        match arg.as_str() {
            "--summary" => {
                params.insert("summary".into(), json!(true));
            }
            other => return Err(anyhow!("streams does not accept argument `{other}`")),
        }
    }
    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

fn parse_paged(
    kind: ServiceKind,
    action: &'static str,
    verb: &str,
    rest: &[String],
) -> Result<Command> {
    let mut params = base_params(kind);
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--page" => {
                params.insert("page".into(), json!(take_value(rest, &mut i, "--page")?));
            }
            "--page-size" => {
                params.insert(
                    "page_size".into(),
                    json!(take_value(rest, &mut i, "--page-size")?),
                );
            }
            other => return Err(anyhow!("{verb} does not accept argument `{other}`")),
        }
        i += 1;
    }
    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

fn parse_terminate(kind: ServiceKind, action: &'static str, rest: &[String]) -> Result<Command> {
    let mut params = base_params(kind);
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--id" => {
                params.insert("id".into(), json!(take_value(rest, &mut i, "--id")?));
            }
            "--reason" => {
                params.insert(
                    "reason".into(),
                    json!(take_value(rest, &mut i, "--reason")?),
                );
            }
            other => {
                return Err(anyhow!(
                    "terminate-stream does not accept argument `{other}`"
                ));
            }
        }
        i += 1;
    }
    if !params.contains_key("id") {
        return Err(anyhow!("terminate-stream requires --id"));
    }
    Ok(Command::Curated {
        action,
        params: Value::Object(params),
    })
}

fn base_params(kind: ServiceKind) -> Map<String, Value> {
    let mut params = Map::new();
    params.insert("service".into(), json!(kind.as_str()));
    params
}

fn take_value(rest: &[String], i: &mut usize, flag: &str) -> Result<String> {
    *i += 1;
    rest.get(*i)
        .filter(|v| !v.starts_with("--"))
        .cloned()
        .ok_or_else(|| anyhow!("{flag} requires a value"))
}

fn resolve(verb: &str) -> Result<Option<&'static str>> {
    let Some((_, action)) = VERBS.iter().find(|(cli_verb, _)| *cli_verb == verb) else {
        return Ok(None);
    };
    curated_command(action)
        .filter(|cmd| cmd.capability == Capability::Trace)
        .map(|cmd| Some(cmd.name))
        .ok_or_else(|| anyhow!("internal: verb `{verb}` has no Trace descriptor"))
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
