//! Live coverage for the **generated OpenAPI operation surface** via Code Mode.
//!
//! For each configured spec-backed service (sonarr/radarr/prowlarr/overseerr/
//! jellyfin/plex) this drives a sample of its generated, no-argument GET operations
//! through `rustarr codemode` against the shart stack — calling them the way an
//! agent would (`<service>.<op>()`), not via any per-service tool. It proves the
//! generated catalog + the per-service callable namespace + the `op` executor all
//! work end-to-end live. Individual upstream quirks (a 404 on an empty library) are
//! tolerated; a service fails the suite only if NONE of its sampled ops dispatch.

use anyhow::{Context, Result, bail};

use super::{matrix, process, report};

/// Kinds whose surface is generated from an OpenAPI spec.
const SPEC_BACKED: &[&str] = &[
    "sonarr",
    "radarr",
    "prowlarr",
    "overseerr",
    "jellyfin",
    "plex",
];

pub fn run(
    report: &mut report::Report,
    rustarr: &process::RustarrProcess,
    matrix: &matrix::Matrix,
) -> Result<()> {
    for service in &matrix.services {
        if !SPEC_BACKED.contains(&service.kind.as_str()) {
            continue;
        }
        let script = sample_get_ops_script(&service.name);
        let out = rustarr
            .json(&["codemode", "--code", &script])
            .with_context(|| format!("codemode generated-op smoke for {}", service.name))?;
        let result = &out["result"];
        let total = result["total"].as_u64().unwrap_or(0);
        let ok = result["ok"].as_u64().unwrap_or(0);

        if total == 0 {
            bail!(
                "no no-argument GET operations discovered for generated service {} \
                 (catalog/preamble may be wired wrong)",
                service.name
            );
        }
        if ok == 0 {
            bail!(
                "all {total} sampled generated GET ops failed for {} — the `<service>.<op>()` \
                 callable / `op` dispatch path is broken (first error: {})",
                service.name,
                result["firstError"].as_str().unwrap_or("<none captured>")
            );
        }
        report.pass(
            format!("codemode generated ops {}", service.name),
            format!("{ok}/{total} sampled generated GET ops dispatched ok via <service>.<op>()"),
        );
    }
    Ok(())
}

/// A Code Mode script that samples up to 8 no-argument GET operations for `service`
/// from the injected catalog and calls each through the per-service namespace,
/// returning `{ total, ok, firstError }`.
fn sample_get_ops_script(service: &str) -> String {
    format!(
        r#"async () => {{
            const svc = {service:?};
            const ops = globalThis.__codemodeCatalog
                .filter(e => e.service === svc && e.scope === "read"
                    && (e.required_params || []).length === 0)
                .slice(0, 8);
            let ok = 0;
            let firstError = null;
            for (const e of ops) {{
                try {{ await globalThis[svc][e.method](); ok++; }}
                catch (err) {{ if (firstError === null) firstError = String(err && err.message || err); }}
            }}
            return {{ total: ops.length, ok, firstError }};
        }}"#
    )
}
