//! Deserialization fixtures for the Tracearr public-API models.

use super::*;
use serde_json::json;

#[path = "tracearr_history_tests.rs"]
mod history;
#[path = "tracearr_activity_tests.rs"]
mod responses;
