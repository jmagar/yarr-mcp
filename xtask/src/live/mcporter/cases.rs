use anyhow::{Result, bail};

use super::{ActionCase, matrix};

mod arr;
mod download;
mod generic;
mod indexer;
mod media_server;
mod requests;
mod stats;

pub(super) fn action_cases(service: &matrix::ServiceCase, action: &str) -> Result<Vec<ActionCase>> {
    if let Some(cases) = generic::cases(service, action)
        .or_else(|| arr::cases(action))
        .or_else(|| indexer::cases(service, action))
        .or_else(|| download::cases(action))
        .or_else(|| media_server::cases(service, action))
        .or_else(|| requests::cases(action))
        .or_else(|| stats::cases(service, action))
    {
        return Ok(cases);
    }

    bail!(
        "action {action} is advertised for {} but xtask has no stateful mcporter test case",
        service.name
    )
}

pub(super) fn expect_type(value_type: &str) -> matrix::Expectation {
    matrix::Expectation {
        json_path: None,
        equals: None,
        equals_any: None,
        value_type: Some(value_type.to_owned()),
        contains: None,
        xml_root: None,
    }
}

pub(super) fn expect_path_contains(path: &str, needle: &str) -> matrix::Expectation {
    matrix::Expectation {
        json_path: Some(path.to_owned()),
        equals: None,
        equals_any: None,
        value_type: None,
        contains: Some(needle.to_owned()),
        xml_root: None,
    }
}
