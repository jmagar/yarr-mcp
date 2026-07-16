//! Tracearr models — public API (`/api/v1/public`).
//!
//! Source of truth is the zod-to-openapi schema (OpenAPI 3.0.0, "Tracearr
//! Public API") that drives both validation and docs. List endpoints
//! (`/users`, `/violations`, `/history`) wrap rows in a `{ data, meta }`
//! envelope; `/streams` wraps in `{ data, summary }`. `/health`, `/stats`,
//! `/stats/today`, and `/activity` are bare objects.
//!
//! Tracearr quirks captured here:
//! - The reserved word `type` on [`ServerStatus`] and [`ViolationRule`] is
//!   renamed to `kind` via `#[serde(rename = "type")]`.
//! - zod `.nullable()` keys are always present but may be `null` → `Option<T>`;
//!   zod `.optional()` keys (only the [`StreamDetails`] sub-objects) may be
//!   absent → `Option<T>` with `#[serde(default)]`.
//! - Timestamps are ISO-8601 datetime *strings* (`2024-01-15T12:00:00.000Z`),
//!   not epoch ints. Activity bucket dates are non-ISO `YYYY-MM-DD HH:MM:SS`
//!   strings. All modelled as `String`.
//! - `StreamsSummary.totalBitrate` is a pre-formatted string (`"22.5 Mbps"`),
//!   not numeric.
//! - The shared `ServerInfo` / `MediaInfo` / `DeviceInfo` / `StreamDetails` /
//!   `DisplayValues` shapes are spread inline (top-level JSON keys) in the
//!   source, so they are `#[serde(flatten)]` mixins here.
//! - `Violation.data` is an arbitrary JSON object (`z.record(string, unknown)`)
//!   → `serde_json::Value`.
//! - Float fields (`watchTimeHours`, `aspectRatio`, transcode `speed`, …) mean
//!   `Eq` is *not* derived anywhere in this module.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[path = "tracearr_core.rs"]
mod core;
pub use core::*;
#[path = "tracearr_activity.rs"]
mod activity;
pub use activity::*;
#[path = "tracearr_history.rs"]
mod history;
pub use history::*;
