//! Typed upstream response models for every supported `ServiceKind`.
//!
//! rustarr forwards upstream payloads as [`serde_json::Value`] and slims them to
//! the fields agents need (see `crate::rustarr::slim`). That keeps the
//! transport generic, but it leaves the *shape* of each service's API implicit —
//! scattered across `.get("field").and_then(Value::as_…)` probes in the app
//! layer. This module makes those shapes explicit: one typed, `Deserialize`-able
//! struct per response the app layer actually reads, organised by the same
//! `Capability` grouping as `src/app/`.
//!
//! Design rules (so the models stay faithful and low-risk):
//!   * **Every field is optional or defaulted.** These upstream APIs are loose,
//!     version-dependent, and routinely add/drop fields; a missing field must
//!     never fail deserialization. `Option<T>` fields default to `None` and
//!     `Vec<T>` fields carry `#[serde(default)]`.
//!   * **Unknown fields are ignored** (serde's default), so a real payload with
//!     dozens of extra keys deserializes cleanly into the slim model.
//!   * **Field selection mirrors the proven `slim()` lists** in `src/app/`, plus
//!     the stable identity/version fields each endpoint is documented to return.
//!   * Models derive [`schemars::JsonSchema`] so a machine-readable JSON Schema
//!     can be emitted for each — directly answering "is there a schema for this
//!     service?" with "yes, here".
//!
//! These models are an *available, tested* typing layer; they do not replace the
//! `Value`+`slim()` forwarding path (which must keep passing arbitrary fields
//! through). Colocated `*_tests.rs` deserialize representative fixtures to prove
//! each model matches the real wire shape.

pub mod arr;
pub mod download;
pub mod indexer;
pub mod media_server;
pub mod requests;
pub mod stats;
pub mod system;
