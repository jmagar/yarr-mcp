//! `cargo xtask gen-openapi` generates lossless operation/type tables for the six
//! spec-backed services from the vendored documents under `specs/`.
//!
//! Generated files contain data only. Supported operations preserve parameter
//! serialization and request/response representations; operations that cannot be
//! encoded safely are emitted into an explicit omission table.

use anyhow::{Context, Result};
use serde_json::Value;

mod emit;
mod extract;
mod naming;
mod types;

use emit::emit_rust;
use extract::extract_operations;
#[cfg(test)]
use extract::server_base_path;
use types::extract_types;

/// (service module name, spec path) for each generated service.
const SPECS: &[(&str, &str)] = &[
    ("sonarr", "specs/sonarr.openapi.json"),
    ("radarr", "specs/radarr.openapi.json"),
    ("prowlarr", "specs/prowlarr.openapi.json"),
    ("overseerr", "specs/overseerr.openapi.yml"),
    ("jellyfin", "specs/jellyfin.openapi.json"),
    ("plex", "specs/plex.openapi.yml"),
];

#[derive(Debug)]
struct ParameterOut {
    name: String,
    location: String,
    required: bool,
    schema: String,
    style: String,
    explode: bool,
}

#[derive(Debug)]
struct RepresentationOut {
    status: Option<String>,
    media_type: String,
    encoding: String,
    schema: String,
    encoding_metadata: String,
}

#[derive(Debug)]
struct RequestBodyOut {
    required: bool,
    representations: Vec<RepresentationOut>,
}

#[derive(Debug)]
struct OperationOut {
    name: String,
    method: String,
    path: String,
    parameters: Vec<ParameterOut>,
    request_body: Option<RequestBodyOut>,
    responses: Vec<RepresentationOut>,
    request_type: Option<String>,
    response_type: Option<String>,
    tag: String,
    summary: String,
    omission_reason: Option<String>,
}

#[derive(Debug)]
struct TypeOut {
    name: String,
    ts: String,
}

pub fn run(_args: &[String]) -> Result<()> {
    for (service, spec_path) in SPECS {
        let root = load_spec(spec_path).with_context(|| format!("loading {spec_path}"))?;
        let operations = extract_operations(&root)
            .with_context(|| format!("extracting operations from {spec_path}"))?;
        let types = extract_types(&root);
        let supported = operations
            .iter()
            .filter(|operation| operation.omission_reason.is_none())
            .count();
        let omitted = operations.len() - supported;
        let code = emit_rust(service, &operations, &types);
        let output = format!("src/openapi/generated/{service}.rs");
        std::fs::write(&output, code).with_context(|| format!("writing {output}"))?;
        println!(
            "  {service:9} -> {output}  ({supported} supported, {omitted} omitted, {} types)",
            types.len()
        );
    }
    println!("gen-openapi: done. Run `cargo fmt` + `cargo build` to verify.");
    Ok(())
}

fn load_spec(path: &str) -> Result<Value> {
    let text = std::fs::read_to_string(path)?;
    if path.ends_with(".json") {
        Ok(serde_json::from_str(&text)?)
    } else {
        Ok(noyalib::from_str_strict(&text)?)
    }
}

#[cfg(test)]
#[path = "gen_openapi_tests.rs"]
mod tests;
