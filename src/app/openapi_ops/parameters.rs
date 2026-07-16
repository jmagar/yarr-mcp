//! OpenAPI path/query/header/cookie parameter serialization.

use anyhow::{Result, anyhow, bail};
use serde_json::Value;

use crate::openapi::{OperationSpec, ParameterLocation, ParameterSpec, ParameterStyle};

pub(super) struct EncodedParameters {
    pub(super) path: Vec<(String, String)>,
    pub(super) query: Vec<(String, String)>,
    pub(super) headers: Vec<(String, String)>,
}

pub(super) fn prepare_parameters(
    spec: &OperationSpec,
    args: &serde_json::Map<String, Value>,
) -> Result<EncodedParameters> {
    let mut output = EncodedParameters {
        path: Vec::new(),
        query: Vec::new(),
        headers: Vec::new(),
    };
    let mut cookies = Vec::new();
    for parameter in spec.parameters {
        let value = args.get(parameter.name).filter(|value| !value.is_null());
        if parameter.required && value.is_none() {
            bail!(
                "operation `{}` requires {} parameter `{}`",
                spec.name,
                location_name(parameter.location),
                parameter.name
            );
        }
        let Some(value) = value else {
            continue;
        };
        let encoded = serialize_parameter(parameter, value)?;
        match parameter.location {
            ParameterLocation::Path => output.path.extend(encoded),
            ParameterLocation::Query => output.query.extend(encoded),
            ParameterLocation::Header => output.headers.push((
                parameter.name.to_string(),
                encoded
                    .iter()
                    .map(|(_, value)| value.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            )),
            ParameterLocation::Cookie => cookies.extend(encoded),
        }
    }
    if !cookies.is_empty() {
        output.headers.push((
            reqwest::header::COOKIE.as_str().to_string(),
            cookies
                .iter()
                .map(|(name, value)| cookie_pair(name, value))
                .collect::<Vec<_>>()
                .join("; "),
        ));
    }
    Ok(output)
}

pub(super) fn serialize_parameter(
    parameter: &ParameterSpec,
    value: &Value,
) -> Result<Vec<(String, String)>> {
    serialize_named(parameter.name, parameter.style, parameter.explode, value)
}

pub(super) fn serialize_named(
    name: &str,
    style: ParameterStyle,
    explode: bool,
    value: &Value,
) -> Result<Vec<(String, String)>> {
    let scalars = scalar_members(name, value)?;
    let rendered = match (style, value) {
        (ParameterStyle::DeepObject, Value::Object(values)) => values
            .iter()
            .map(|(key, value)| Ok((format!("{name}[{key}]"), scalar(name, value)?)))
            .collect::<Result<Vec<_>>>()?,
        (ParameterStyle::Form, Value::Array(_)) if explode => scalars
            .into_iter()
            .map(|value| (name.to_string(), value))
            .collect(),
        (ParameterStyle::Form, Value::Object(values)) if explode => values
            .iter()
            .map(|(key, value)| Ok((key.clone(), scalar(name, value)?)))
            .collect::<Result<Vec<_>>>()?,
        (ParameterStyle::SpaceDelimited, Value::Array(_)) => {
            vec![(name.to_string(), scalars.join(" "))]
        }
        (ParameterStyle::PipeDelimited, Value::Array(_)) => {
            vec![(name.to_string(), scalars.join("|"))]
        }
        (ParameterStyle::Simple, Value::Object(values)) if explode => {
            vec![(name.to_string(), object_equals(name, values, ",")?)]
        }
        (ParameterStyle::Label, Value::Object(values)) if explode => vec![(
            name.to_string(),
            format!(".{}", object_equals(name, values, ".")?),
        )],
        (ParameterStyle::Label, _) => vec![(
            name.to_string(),
            format!(
                ".{}",
                scalars.join(if explode && value.is_array() {
                    "."
                } else {
                    ","
                })
            ),
        )],
        (ParameterStyle::Matrix, Value::Array(_)) if explode => vec![(
            name.to_string(),
            scalars
                .iter()
                .map(|value| format!(";{name}={value}"))
                .collect::<String>(),
        )],
        (ParameterStyle::Matrix, Value::Object(values)) if explode => vec![(
            name.to_string(),
            format!(";{}", object_equals(name, values, ";")?),
        )],
        (ParameterStyle::Matrix, _) => {
            vec![(name.to_string(), format!(";{name}={}", scalars.join(",")))]
        }
        (ParameterStyle::Simple | ParameterStyle::Form, _) => {
            vec![(name.to_string(), scalars.join(","))]
        }
        (ParameterStyle::DeepObject, _) => {
            bail!("parameter `{name}` deepObject value must be an object")
        }
        (ParameterStyle::SpaceDelimited | ParameterStyle::PipeDelimited, _) => {
            bail!("parameter `{name}` delimited value must be an array")
        }
    };
    Ok(rendered)
}

fn object_equals(
    name: &str,
    values: &serde_json::Map<String, Value>,
    delimiter: &str,
) -> Result<String> {
    values
        .iter()
        .map(|(key, value)| Ok(format!("{key}={}", scalar(name, value)?)))
        .collect::<Result<Vec<_>>>()
        .map(|values| values.join(delimiter))
}

fn scalar_members(name: &str, value: &Value) -> Result<Vec<String>> {
    match value {
        Value::Array(values) => values.iter().map(|value| scalar(name, value)).collect(),
        Value::Object(values) => values
            .iter()
            .flat_map(|(key, value)| [Ok(key.clone()), scalar(name, value)])
            .collect(),
        other => Ok(vec![scalar(name, other)?]),
    }
}

fn scalar(name: &str, value: &Value) -> Result<String> {
    crate::openapi::scalar_to_string(value)
        .ok_or_else(|| anyhow!("parameter `{name}` values must be strings/numbers/bools"))
}

fn cookie_pair(name: &str, value: &str) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    serializer.append_pair(name, value);
    serializer.finish()
}

fn location_name(location: ParameterLocation) -> &'static str {
    match location {
        ParameterLocation::Path => "path",
        ParameterLocation::Query => "query",
        ParameterLocation::Header => "header",
        ParameterLocation::Cookie => "cookie",
    }
}

#[cfg(test)]
#[path = "parameters_tests.rs"]
mod tests;
