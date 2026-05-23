use anyhow::Result;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct ScaffoldIntent {
    pub display_name: String,
    pub crate_name: String,
    pub binary_name: String,
    pub server_category: String,
    pub env_prefix: String,
    pub auth_kind: String,
    pub host: String,
    pub port: u16,
    pub mcp_transport: String,
    pub mcp_primitives: String,
    pub deployment: String,
    pub plugins: String,
    pub publish_mcp: bool,
    pub crawl_urls: String,
    pub crawl_repos: String,
    pub crawl_search_topics: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldIntentValidationError {
    message: String,
}

impl ScaffoldIntentValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ScaffoldIntentValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ScaffoldIntentValidationError {}

pub fn build_scaffold_intent(input: ScaffoldIntent) -> Result<Value> {
    validate_scaffold_intent(&input)?;
    let category = normalize_category(&input.server_category);
    let required_surfaces = if category == "application-platform" {
        vec!["api", "cli", "mcp", "web"]
    } else {
        vec!["mcp", "cli"]
    };
    let service_name = input.binary_name.trim().replace('-', "_");
    let env_prefix = input.env_prefix.trim().to_ascii_uppercase();

    Ok(json!({
        "kind": "rustarr_scaffold_intent",
        "schema_version": 1,
        "server_category": category,
        "required_surfaces": required_surfaces,
        "project": {
            "display_name": input.display_name.trim(),
            "crate_name": input.crate_name.trim(),
            "binary_name": input.binary_name.trim(),
            "service_name": service_name,
            "env_prefix": env_prefix,
        },
        "upstream": {
            "base_url_env": format!("{env_prefix}_API_URL"),
            "auth_kind": normalize_auth_kind(&input.auth_kind),
        },
        "runtime": {
            "host": normalize_host(&input.host),
            "port": input.port,
            "mcp_transport": normalize_transport(&input.mcp_transport),
        },
        "mcp_primitives": normalize_primitives(&input.mcp_primitives),
        "deployment": normalize_deployment(&input.deployment),
        "plugins": normalize_plugins(&input.plugins),
        "publish_mcp": input.publish_mcp,
        "crawl_docs": {
            "urls": split_csv(&input.crawl_urls),
            "repos": split_csv(&input.crawl_repos),
            "search_topics": split_csv(&input.crawl_search_topics),
        },
        "handoff": {
            "recommended_skill": "scaffold-project",
            "instructions": "Create an approval-first scaffold plan from this JSON. Do not mutate files until the user approves the plan.",
        },
        "policy": {
            "business_action_minimum_surfaces": ["mcp", "cli"],
            "upstream_client_surfaces": ["mcp", "cli"],
            "application_platform_surfaces": ["api", "cli", "mcp", "web"],
        }
    }))
}

fn validate_scaffold_intent(input: &ScaffoldIntent) -> Result<()> {
    validate_non_empty("display_name", &input.display_name)?;
    validate_kebab_identifier("crate_name", &input.crate_name)?;
    validate_kebab_identifier("binary_name", &input.binary_name)?;
    validate_env_prefix(&input.env_prefix)?;
    if input.port == 0 {
        return Err(ScaffoldIntentValidationError::new("port must be between 1 and 65535").into());
    }
    validate_urls("crawl_urls", &input.crawl_urls)?;
    validate_urls("crawl_repos", &input.crawl_repos)?;
    Ok(())
}

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ScaffoldIntentValidationError::new(format!(
            "`{field}` is required and must not be empty"
        ))
        .into());
    }
    Ok(())
}

fn validate_kebab_identifier(field: &str, value: &str) -> Result<()> {
    let value = value.trim();
    validate_non_empty(field, value)?;
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(ScaffoldIntentValidationError::new(format!(
            "`{field}` is required and must not be empty"
        ))
        .into());
    };
    if !first.is_ascii_lowercase()
        || !chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(ScaffoldIntentValidationError::new(format!(
            "`{field}` must match ^[a-z][a-z0-9-]*$"
        ))
        .into());
    }
    Ok(())
}

fn validate_env_prefix(value: &str) -> Result<()> {
    let value = value.trim().to_ascii_uppercase();
    validate_non_empty("env_prefix", &value)?;
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(ScaffoldIntentValidationError::new(
            "`env_prefix` is required and must not be empty",
        )
        .into());
    };
    if !first.is_ascii_uppercase()
        || !chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(ScaffoldIntentValidationError::new(
            "`env_prefix` must match ^[A-Z][A-Z0-9_]*$",
        )
        .into());
    }
    Ok(())
}

fn validate_urls(field: &str, value: &str) -> Result<()> {
    for item in split_csv(value) {
        url::Url::parse(&item).map_err(|_| {
            ScaffoldIntentValidationError::new(format!("`{field}` contains invalid URL: {item}"))
        })?;
    }
    Ok(())
}

fn normalize_category(category: &str) -> &'static str {
    let normalized = category.trim().to_ascii_lowercase();
    if normalized.contains("application") || normalized.contains("platform") {
        "application-platform"
    } else {
        "upstream-client"
    }
}

fn normalize_auth_kind(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => "none",
        "api-key" | "apikey" | "api_key" | "api key" | "key" => "api-key",
        "bearer" | "token" => "bearer",
        "oauth" => "oauth",
        "both" => "both",
        _ => "other",
    }
}

fn normalize_host(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "127.0.0.1".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn normalize_transport(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "stdio" => "stdio",
        "http" | "streamable-http" | "streamable_http" => "http",
        _ => "dual",
    }
}

fn normalize_deployment(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "systemd" => "systemd",
        "docker" | "container" | "containers" => "docker",
        _ => "none",
    }
}

fn normalize_primitives(value: &str) -> Vec<String> {
    let mut primitives = Vec::new();
    for item in split_csv(value) {
        let primitive = match item.to_ascii_lowercase().as_str() {
            "tools" | "tool" => Some("tools"),
            "resources" | "resource" => Some("resources"),
            "prompts" | "prompt" => Some("prompts"),
            "elicitation" | "elicit" => Some("elicitation"),
            _ => None,
        };
        if let Some(primitive) = primitive {
            let primitive = primitive.to_owned();
            if !primitives.contains(&primitive) {
                primitives.push(primitive);
            }
        }
    }
    if primitives.is_empty() {
        primitives.push("tools".to_owned());
    }
    primitives
}

fn normalize_plugins(value: &str) -> Vec<String> {
    let mut plugins = Vec::new();
    for item in split_csv(value) {
        let plugin = match item.to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claude_code" => Some("claude"),
            "codex" => Some("codex"),
            "gemini" => Some("gemini"),
            "none" => None,
            _ => None,
        };
        if let Some(plugin) = plugin {
            let plugin = plugin.to_owned();
            if !plugins.contains(&plugin) {
                plugins.push(plugin);
            }
        }
    }
    plugins
}

fn split_csv(value: &str) -> Vec<String> {
    let mut items = Vec::new();
    for item in value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let item = item.to_owned();
        if !items.contains(&item) {
            items.push(item);
        }
    }
    items
}
