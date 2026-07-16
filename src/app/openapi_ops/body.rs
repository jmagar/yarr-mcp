//! OpenAPI request/response representation selection and body encoding.

use anyhow::{Context, Result, anyhow, bail, ensure};
use base64::Engine as _;
use serde_json::Value;

use crate::openapi::{BodyEncoding, OperationSpec, ParameterStyle, RepresentationSpec};
use crate::yarr::{EncodedRequestBody, MultipartField};

use super::parameters::serialize_named;

const MAX_UPLOAD_BYTES: usize = 32 * 1024 * 1024;

pub(super) fn select_response<'a>(
    spec: &'a OperationSpec,
    args: &serde_json::Map<String, Value>,
) -> Result<&'a RepresentationSpec> {
    const DEFAULT: RepresentationSpec = RepresentationSpec {
        status: None,
        media_type: "application/json",
        encoding: BodyEncoding::Json,
        schema: "null",
        encoding_metadata: "null",
    };
    if let Some(requested) = args.get("responseContentType").and_then(Value::as_str) {
        return spec
            .responses
            .iter()
            .find(|response| response.media_type.eq_ignore_ascii_case(requested))
            .ok_or_else(|| anyhow!("operation `{}` has no `{requested}` response", spec.name));
    }
    Ok(spec
        .responses
        .iter()
        .find(|response| response.encoding == BodyEncoding::Json)
        .or_else(|| spec.responses.first())
        .unwrap_or(&DEFAULT))
}

pub(super) fn encode_request_body(
    spec: &OperationSpec,
    args: &serde_json::Map<String, Value>,
) -> Result<Option<EncodedRequestBody>> {
    let Some(body_spec) = spec.request_body else {
        ensure!(
            !args.contains_key("body")
                && !args.contains_key("bodyBase64")
                && !args.contains_key("multipartFileBase64"),
            "operation `{}` does not accept a request body",
            spec.name
        );
        return Ok(None);
    };
    let requested = args.get("contentType").and_then(Value::as_str);
    let representation = if let Some(requested) = requested {
        body_spec
            .representations
            .iter()
            .find(|representation| representation.media_type.eq_ignore_ascii_case(requested))
            .ok_or_else(|| {
                anyhow!(
                    "operation `{}` has no `{requested}` request body",
                    spec.name
                )
            })?
    } else {
        body_spec
            .representations
            .iter()
            .find(|representation| representation.encoding == BodyEncoding::Json)
            .or_else(|| body_spec.representations.first())
            .ok_or_else(|| anyhow!("operation `{}` has no supported request body", spec.name))?
    };
    let has_input = args.contains_key("body")
        || args.contains_key("bodyBase64")
        || args.contains_key("multipartFileBase64");
    if !has_input && !body_spec.required {
        return Ok(None);
    }
    match representation.encoding {
        BodyEncoding::Json => args
            .get("body")
            .cloned()
            .map(|value| EncodedRequestBody::Json {
                media_type: representation.media_type.to_string(),
                value,
            })
            .ok_or_else(|| anyhow!("operation `{}` requires `body`", spec.name))
            .map(Some),
        BodyEncoding::FormUrlEncoded => Ok(Some(EncodedRequestBody::Form(form_pairs(
            required_object_body(spec, args)?,
            representation.encoding_metadata,
        )?))),
        BodyEncoding::Multipart => Ok(Some(EncodedRequestBody::Multipart(multipart_fields(
            spec,
            args,
            representation,
        )?))),
        BodyEncoding::Text => Ok(Some(EncodedRequestBody::Text {
            media_type: representation.media_type.to_string(),
            value: args
                .get("body")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("operation `{}` requires string `body`", spec.name))?
                .to_string(),
        })),
        BodyEncoding::Binary => Ok(Some(EncodedRequestBody::Binary {
            media_type: representation.media_type.to_string(),
            bytes: decode_base64_arg(spec, args, "bodyBase64")?,
        })),
    }
}

fn required_object_body<'a>(
    spec: &OperationSpec,
    args: &'a serde_json::Map<String, Value>,
) -> Result<&'a serde_json::Map<String, Value>> {
    args.get("body")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("operation `{}` requires object `body`", spec.name))
}

fn form_pairs(
    body: &serde_json::Map<String, Value>,
    encoding_metadata: &str,
) -> Result<Vec<(String, String)>> {
    let metadata: Value = serde_json::from_str(encoding_metadata).unwrap_or(Value::Null);
    let mut output = Vec::new();
    for (name, value) in body {
        let encoding = metadata.get(name);
        let style = encoding
            .and_then(|value| value.get("style"))
            .and_then(Value::as_str)
            .map(style_from_str)
            .transpose()?
            .unwrap_or(ParameterStyle::Form);
        let explode = encoding
            .and_then(|value| value.get("explode"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        output.extend(serialize_named(name, style, explode, value)?);
    }
    Ok(output)
}

fn multipart_fields(
    spec: &OperationSpec,
    args: &serde_json::Map<String, Value>,
    representation: &RepresentationSpec,
) -> Result<Vec<MultipartField>> {
    let mut fields = Vec::new();
    if let Some(body) = args.get("body") {
        for (name, value) in body.as_object().ok_or_else(|| {
            anyhow!(
                "operation `{}` multipart `body` must be an object",
                spec.name
            )
        })? {
            fields.push(MultipartField::Text {
                name: name.clone(),
                value: crate::openapi::scalar_to_string(value).unwrap_or_else(|| value.to_string()),
            });
        }
    }
    if args.contains_key("multipartFileBase64") {
        let name = args
            .get("multipartField")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| binary_property_name(representation.schema))
            .unwrap_or_else(|| "file".to_string());
        fields.push(MultipartField::File {
            file_name: args
                .get("fileName")
                .and_then(Value::as_str)
                .unwrap_or("upload.bin")
                .to_string(),
            media_type: args
                .get("multipartMediaType")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| property_media_type(representation.encoding_metadata, &name))
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            name,
            bytes: decode_base64_arg(spec, args, "multipartFileBase64")?,
        });
    }
    ensure!(
        !fields.is_empty(),
        "operation `{}` requires multipart `body` or `multipartFileBase64`",
        spec.name
    );
    Ok(fields)
}

fn decode_base64_arg(
    spec: &OperationSpec,
    args: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<Vec<u8>> {
    let encoded = args
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("operation `{}` requires `{name}`", spec.name))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .with_context(|| format!("decode `{name}` for operation `{}`", spec.name))?;
    ensure!(
        bytes.len() <= MAX_UPLOAD_BYTES,
        "`{name}` exceeds upload limit"
    );
    Ok(bytes)
}

fn binary_property_name(schema: &str) -> Option<String> {
    let schema: Value = serde_json::from_str(schema).ok()?;
    schema
        .get("properties")?
        .as_object()?
        .iter()
        .find_map(|(name, schema)| {
            (schema.get("format").and_then(Value::as_str) == Some("binary")).then(|| name.clone())
        })
}

fn property_media_type(metadata: &str, field: &str) -> Option<String> {
    serde_json::from_str::<Value>(metadata)
        .ok()?
        .get(field)?
        .get("contentType")?
        .as_str()
        .map(str::to_string)
}

fn style_from_str(style: &str) -> Result<ParameterStyle> {
    Ok(match style {
        "simple" => ParameterStyle::Simple,
        "label" => ParameterStyle::Label,
        "matrix" => ParameterStyle::Matrix,
        "form" => ParameterStyle::Form,
        "spaceDelimited" => ParameterStyle::SpaceDelimited,
        "pipeDelimited" => ParameterStyle::PipeDelimited,
        "deepObject" => ParameterStyle::DeepObject,
        _ => bail!("unsupported parameter style `{style}`"),
    })
}

#[cfg(test)]
#[path = "body_tests.rs"]
mod tests;
