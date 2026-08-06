use crate::adapter::{ProviderError, ResolvedProfile};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SearxngResult {
    pub title: String,
    pub url: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearxngResponse {
    #[serde(default)]
    pub results: Vec<SearxngResult>,
}

/// Minimal backend provider for a self-hosted [SearXNG](https://docs.searxng.org/) instance.
///
/// Not a chat `Adapter`; it is used as a web-search backend invoked by the tool-resolution path
/// when the target model does not support native web search.
pub struct SearxngProvider {
    client: Client,
}

impl SearxngProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("build reqwest client");
        Self { client }
    }

    /// Run a search against `{base_url}/search?q={query}&format=json` and return normalized results.
    pub async fn search(
        &self,
        profile: &ResolvedProfile,
        query: &str,
    ) -> Result<Vec<SearxngResult>, ProviderError> {
        let url = format!("{}/search", profile.base_url.trim_end_matches('/'));
        let params = [("q", query), ("format", "json")];
        let resp = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| ProviderError::Http {
                status: 0,
                message: e.to_string(),
            })?;
        if !resp.status().is_success() {
            return Err(ProviderError::Http {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        let body: SearxngResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok(body.results)
    }
}

impl Default for SearxngProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    fn profile(uri: String) -> ResolvedProfile {
        ResolvedProfile {
            base_url: uri,
            api_key: None,
        }
    }

    #[tokio::test]
    async fn searxng_search_parses_results() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "results": [
                {"title": "First", "url": "https://a.example", "content": "alpha"},
                {"title": "Second", "url": "https://b.example", "content": "beta"}
            ]
        });
        Mock::given(method("GET"))
            .and(path("/search"))
            .and(query_param("q", "hello world"))
            .and(query_param("format", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let provider = SearxngProvider::new();
        let results = provider.search(&profile(server.uri()), "hello world").await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "First");
        assert_eq!(results[0].url, "https://a.example");
        assert_eq!(results[0].content.as_deref(), Some("alpha"));
        assert_eq!(results[1].title, "Second");
    }

    #[tokio::test]
    async fn searxng_search_propagates_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({"error":"boom"})))
            .mount(&server)
            .await;

        let provider = SearxngProvider::new();
        let err = provider.search(&profile(server.uri()), "query").await.unwrap_err();
        match err {
            ProviderError::Http { status, .. } => assert_eq!(status, 500),
            other => panic!("expected http error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn searxng_search_tolerates_missing_content() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "results": [{"title": "NoContent", "url": "https://c.example"}]
        });
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let provider = SearxngProvider::new();
        let results = provider.search(&profile(server.uri()), "q").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, None);
    }

    #[tokio::test]
    async fn searxng_search_handles_trailing_slash_in_base_url() {
        let server = MockServer::start().await;
        let body = serde_json::json!({ "results": [] });
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let provider = SearxngProvider::new();
        let uri = format!("{}/", server.uri());
        let results = provider.search(&profile(uri), "q").await.unwrap();
        assert!(results.is_empty());
    }
}
