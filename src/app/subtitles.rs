//! Bazarr subtitles capability.

use anyhow::Result;
use serde_json::Value;

use crate::app::YarrService;
use crate::capability::Capability;
use crate::config::ServiceConfig;
use crate::yarr::helpers::build_operation_url;

impl YarrService {
    fn subtitles_context<'a>(&'a self, service: &str) -> Result<&'a ServiceConfig> {
        self.service_of_capability(service, Capability::Subtitles)
    }

    pub async fn subtitles_status(&self, service: &str) -> Result<Value> {
        let config = self.subtitles_context(service)?;
        self.client_ref()
            .get_json(config, "/api/system/status")
            .await
    }

    pub async fn subtitles_movies(
        &self,
        service: &str,
        start: Option<i64>,
        length: Option<i64>,
    ) -> Result<Value> {
        self.subtitles_get(service, "/api/movies", paging_query(start, length))
            .await
    }

    pub async fn subtitles_episodes(
        &self,
        service: &str,
        start: Option<i64>,
        length: Option<i64>,
    ) -> Result<Value> {
        self.subtitles_get(service, "/api/episodes", paging_query(start, length))
            .await
    }

    pub async fn subtitles_wanted_episodes(
        &self,
        service: &str,
        start: Option<i64>,
        length: Option<i64>,
    ) -> Result<Value> {
        self.subtitles_get(service, "/api/episodes/wanted", paging_query(start, length))
            .await
    }

    pub async fn subtitles_wanted_movies(
        &self,
        service: &str,
        start: Option<i64>,
        length: Option<i64>,
    ) -> Result<Value> {
        self.subtitles_get(service, "/api/movies/wanted", paging_query(start, length))
            .await
    }

    pub async fn subtitles_providers(&self, service: &str) -> Result<Value> {
        self.subtitles_get(service, "/api/providers", Vec::new())
            .await
    }

    pub async fn subtitles_languages(&self, service: &str) -> Result<Value> {
        self.subtitles_get(service, "/api/system/languages", Vec::new())
            .await
    }

    async fn subtitles_get(
        &self,
        service: &str,
        path: &str,
        query: Vec<(&str, String)>,
    ) -> Result<Value> {
        let config = self.subtitles_context(service)?;
        let url = build_operation_url(config, path, &[], &query)?;
        self.client_ref().send_get(config, url, None).await
    }
}

fn paging_query(start: Option<i64>, length: Option<i64>) -> Vec<(&'static str, String)> {
    let mut query = Vec::new();
    if let Some(start) = start {
        query.push(("start", start.to_string()));
    }
    if let Some(length) = length {
        query.push(("length", length.to_string()));
    }
    query
}

#[cfg(test)]
#[path = "subtitles_tests.rs"]
mod tests;
