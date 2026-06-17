//! ArrManager name→id resolution helpers.
//!
//! Pre-created in C1 (read commands) so C2 (write/intent commands) can reuse the
//! quality-profile resolver without a mid-epic file split. Resolvers live in the
//! app layer (business logic) — never in a shim — and resolve a human-friendly
//! name to the numeric id the upstream *arr API expects.

use anyhow::{Result, anyhow};
use serde_json::Value;

use crate::app::RustarrService;
use crate::capability::Capability;

impl RustarrService {
    /// Resolve a quality-profile *name* to its numeric id for an ArrManager
    /// service (sonarr/radarr). Matching is case-insensitive and
    /// trimmed. Returns a teaching error listing the available profile names when
    /// no profile matches, so a caller can correct the name on the next try.
    ///
    /// Used by C2 write commands (`set_quality`, `add`) so the read and write
    /// beads share one resolver.
    pub(crate) async fn arr_resolve_quality_profile_id(
        &self,
        service: &str,
        name: &str,
    ) -> Result<i64> {
        let needle = name.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Err(anyhow!("quality profile name is required"));
        }
        let config = self.service_of_capability(service, Capability::ArrManager)?;
        let prefix = config.kind.descriptor().api_prefix;
        let path = format!("{prefix}/qualityprofile");
        let profiles = self.client_ref().get_json(config, &path).await?;
        match_quality_profile_id(&profiles, name)
    }
}

/// Pure name→id match over a quality-profile list payload. Case-insensitive,
/// trimmed. Errors with a teaching message listing the available names. Split out
/// (no `self`/network) so the matching contract is unit-testable.
pub(crate) fn match_quality_profile_id(profiles: &Value, name: &str) -> Result<i64> {
    let needle = name.trim().to_ascii_lowercase();
    // Borrow the array — no need to deep-clone every profile Value just to scan it
    // (this runs up to twice per `set-quality`, once for `to` and once for `from`).
    let items: &[Value] = profiles.as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut available: Vec<String> = Vec::new();
    for profile in items {
        let pname = profile.get("name").and_then(Value::as_str).unwrap_or("");
        available.push(pname.to_owned());
        if pname.trim().to_ascii_lowercase() == needle
            && let Some(id) = profile.get("id").and_then(Value::as_i64)
        {
            return Ok(id);
        }
    }
    Err(anyhow!(
        "no quality profile named `{name}`; available profiles: [{}]",
        available.join(", ")
    ))
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod tests;
