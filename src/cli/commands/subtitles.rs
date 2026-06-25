//! CLI parse module for Bazarr subtitles curated commands.

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::actions::curated_command;
use crate::capability::Capability;
use crate::cli::command::Command;
use crate::config::ServiceKind;

pub const VERBS: &[(&str, &str)] = &[
    ("status-info", "subtitles_status"),
    ("movies", "subtitles_movies"),
    ("episodes", "subtitles_episodes"),
    ("wanted-episodes", "subtitles_wanted_episodes"),
    ("wanted-movies", "subtitles_wanted_movies"),
    ("providers", "subtitles_providers"),
    ("languages", "subtitles_languages"),
];

pub fn parse(kind: ServiceKind, verb: &str, rest: &[String]) -> Result<Option<Command>> {
    let Some(action) = resolve(verb)? else {
        return Ok(None);
    };
    match verb {
        "movies" | "episodes" | "wanted-episodes" | "wanted-movies" => {
            parse_paged(kind, action, verb, rest).map(Some)
        }
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
            flag @ ("--start" | "--length") => {
                let value = take_value(rest, &mut i, flag)?;
                params.insert(flag[2..].into(), json!(value));
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
        .filter(|cmd| cmd.capability == Capability::Subtitles)
        .map(|cmd| Some(cmd.name))
        .ok_or_else(|| anyhow!("internal: verb `{verb}` has no Subtitles descriptor"))
}

#[cfg(test)]
#[path = "subtitles_tests.rs"]
mod tests;
