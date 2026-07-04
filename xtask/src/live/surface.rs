#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceCheck {
    pub name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceInventory {
    pub checks: Vec<SurfaceCheck>,
}

pub fn inventory() -> SurfaceInventory {
    SurfaceInventory {
        checks: [
            "cli setup repair",
            "cli setup install",
            "cli serve default lifecycle",
            "cli serve mcp lifecycle",
            "cli mcp stdio initialize",
            "cli unknown command error",
            "cli parser rejects invalid watch interval",
            "rest mcp auth rejects missing bearer",
            "rest mcp auth accepts bearer",
            "rest oauth authorization metadata",
            "rest oauth protected resource metadata",
            "mcp resources/read schema",
            "mcp unknown tool error",
            "mcp api_get validation error",
            "mcp api_post confirmed upstream error",
            "mcporter contract sonarr",
            "mcporter contract radarr",
            "mcporter contract prowlarr",
            "mcporter contract overseerr",
            "mcporter contract jellyfin",
            "mcporter contract plex",
        ]
        .into_iter()
        .map(|name| SurfaceCheck { name })
        .collect(),
    }
}

pub fn runtime_markers() -> Vec<&'static str> {
    inventory()
        .checks
        .into_iter()
        .map(|check| check.name)
        .collect()
}
