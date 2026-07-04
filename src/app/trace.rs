//! Tracearr public API capability.

use anyhow::Result;
use serde_json::{Value, json};

use crate::app::RustarrService;
use crate::capability::Capability;
use crate::config::{ServiceConfig, destructive_allowed};
use crate::yarr::helpers::build_operation_url;

impl RustarrService {
    fn trace_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::Trace)
    }

    pub async fn trace_health(&self, service: &str) -> Result<Value> {
        self.trace_get(service, "/api/v1/public/health", Vec::new())
            .await
    }

    pub async fn trace_stats(&self, service: &str) -> Result<Value> {
        self.trace_get(service, "/api/v1/public/stats", Vec::new())
            .await
    }

    pub async fn trace_today(&self, service: &str, timezone: Option<&str>) -> Result<Value> {
        let query = optional_query("timezone", timezone);
        self.trace_get(service, "/api/v1/public/stats/today", query)
            .await
    }

    pub async fn trace_activity(&self, service: &str, period: Option<&str>) -> Result<Value> {
        let query = optional_query("period", period);
        self.trace_get(service, "/api/v1/public/activity", query)
            .await
    }

    pub async fn trace_streams(&self, service: &str, summary: bool) -> Result<Value> {
        let query = if summary {
            vec![("summary", "true".to_string())]
        } else {
            Vec::new()
        };
        self.trace_get(service, "/api/v1/public/streams", query)
            .await
    }

    pub async fn trace_users(
        &self,
        service: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Value> {
        self.trace_get(
            service,
            "/api/v1/public/users",
            paging_query(page, page_size),
        )
        .await
    }

    pub async fn trace_violations(
        &self,
        service: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Value> {
        self.trace_get(
            service,
            "/api/v1/public/violations",
            paging_query(page, page_size),
        )
        .await
    }

    pub async fn trace_history(
        &self,
        service: &str,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Value> {
        self.trace_get(
            service,
            "/api/v1/public/history",
            paging_query(page, page_size),
        )
        .await
    }

    pub async fn trace_terminate_stream(
        &self,
        service: &str,
        id: &str,
        reason: Option<&str>,
        confirm: bool,
    ) -> Result<Value> {
        if !confirm && !destructive_allowed() {
            anyhow::bail!(
                "trace terminate-stream is destructive and requires confirm=true (MCP: approve \
                 the elicitation prompt; CLI: pass --confirm; or set YARR_ALLOW_DESTRUCTIVE \
                 on a disposable test stack)"
            );
        }
        let config = self.trace_context(service)?;
        let url = build_operation_url(
            config,
            "/api/v1/public/streams/{id}/terminate",
            &[("id", id.to_string())],
            &[],
        )?;
        let body = match reason {
            Some(reason) => json!({ "reason": reason }),
            None => Value::Object(serde_json::Map::new()),
        };
        self.client_ref()
            .request_url(
                reqwest::Method::POST,
                config,
                url,
                Some(body),
                Some("application/json"),
            )
            .await
    }

    async fn trace_get(
        &self,
        service: &str,
        path: &str,
        query: Vec<(&str, String)>,
    ) -> Result<Value> {
        let config = self.trace_context(service)?;
        let url = build_operation_url(config, path, &[], &query)?;
        self.client_ref()
            .send_get(config, url, Some("application/json"))
            .await
    }
}

fn optional_query(name: &'static str, value: Option<&str>) -> Vec<(&'static str, String)> {
    value
        .filter(|v| !v.trim().is_empty())
        .map(|v| vec![(name, v.to_string())])
        .unwrap_or_default()
}

fn paging_query(page: Option<i64>, page_size: Option<i64>) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();
    if let Some(page) = page {
        query.push(("page", page.to_string()));
    }
    if let Some(page_size) = page_size {
        query.push(("pageSize", page_size.to_string()));
    }
    query
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
