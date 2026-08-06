use serde::Deserialize;
use std::time::Duration;
use tokio::time::timeout;
use axum::response::{IntoResponse, Response};
use godwit_db::models::ApiKey;
use std::sync::Arc;
use crate::state::AppState;
use crate::error::ApiError;

#[derive(Debug, Clone, Deserialize)]
pub struct RerankProviderConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RerankFallbackConfig {
    pub providers: Vec<RerankProviderConfig>,
    #[serde(default = "default_timeout_per_provider")]
    pub timeout_per_provider: Duration,
}

fn default_timeout_per_provider() -> Duration {
    Duration::from_secs(15)
}

impl Default for RerankFallbackConfig {
    fn default() -> Self {
        RerankFallbackConfig {
            providers: vec![
                RerankProviderConfig {
                    name: "cohere".to_string(),
                    base_url: "https://api.cohere.ai/v1".to_string(),
                    api_key: None,
                    model: "rerank-english-v3.0".to_string(),
                },
                RerankProviderConfig {
                    name: "azure".to_string(),
                    base_url: "https://your-resource.openai.azure.com/openai".to_string(),
                    api_key: None,
                    model: "rerank-model".to_string(),
                },
                RerankProviderConfig {
                    name: "self-hosted".to_string(),
                    base_url: "http://localhost:8000/v1".to_string(),
                    api_key: None,
                    model: "rerank-model".to_string(),
                },
            ],
            timeout_per_provider: Duration::from_secs(15),
        }
    }
}

pub struct RerankFallback {
    config: RerankFallbackConfig,
}

impl RerankFallback {
    pub fn new(config: RerankFallbackConfig) -> Self {
        RerankFallback { config }
    }

    pub fn from_config_value(config: Option<&serde_json::Value>) -> Self {
        let providers = config
            .and_then(|c| c.get("rerank_providers"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let name = v.get("name")?.as_str()?.to_string();
                        let base_url = v.get("base_url")?.as_str()?.to_string();
                        let api_key = v.get("api_key").and_then(|k| k.as_str()).map(|s| s.to_string());
                        let model = v.get("model")?.as_str()?.to_string();
                        Some(RerankProviderConfig {
                            name,
                            base_url,
                            api_key,
                            model,
                        })
                    })
                    .collect::<Vec<_>>()
            });

        let timeout_secs = config
            .and_then(|c| c.get("rerank_timeout_per_provider_secs"))
            .and_then(|v| v.as_u64());

        RerankFallback {
            config: RerankFallbackConfig {
                providers: providers.unwrap_or_else(|| RerankFallbackConfig::default().providers),
                timeout_per_provider: Duration::from_secs(timeout_secs.unwrap_or(15)),
            },
        }
    }

    pub async fn execute(
        &self,
        state: &Arc<AppState>,
        api_key: &ApiKey,
        body: serde_json::Value,
    ) -> Result<Response, ApiError> {
        let mut last_error: Option<ApiError> = None;

        for (idx, provider) in self.config.providers.iter().enumerate() {
            tracing::info!(
                "Attempting rerank with provider {} ({}/{})",
                provider.name,
                idx + 1,
                self.config.providers.len()
            );

            match timeout(
                self.config.timeout_per_provider,
                self.call_provider(state, api_key, provider, &body),
            )
            .await
            {
                Ok(Ok(response)) => {
                    tracing::info!("Rerank succeeded with provider: {}", provider.name);
                    return Ok(response);
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        "Rerank failed with provider {}: {:?}",
                        provider.name,
                        e
                    );
                    last_error = Some(e);
                }
                Err(_) => {
                    tracing::warn!(
                        "Rerank timed out with provider {} after {:?}",
                        provider.name,
                        self.config.timeout_per_provider
                    );
                    last_error = Some(ApiError::Core(
                        godwit_core::PasteurError::Provider(format!(
                            "rerank timeout after {:?}",
                            self.config.timeout_per_provider
                        ))
                    ));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ApiError::Core(godwit_core::PasteurError::Provider(
                "All rerank providers failed".to_string(),
            ))
        }))
    }

    async fn call_provider(
        &self,
        _state: &Arc<AppState>,
        _api_key: &ApiKey,
        provider: &RerankProviderConfig,
        body: &serde_json::Value,
    ) -> Result<Response, ApiError> {
        let mut body_mut = body.clone();
        body_mut["model"] = serde_json::Value::String(provider.model.clone());

        let url = format!(
            "{}/rerank",
            provider.base_url.trim_end_matches('/')
        );

        let client = reqwest::Client::new();
        let mut req = client
            .post(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_TYPE, "application/json");

        if let Some(key) = &provider.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        req = req.json(&body_mut);

        let res = req.send().await.map_err(|e| {
            ApiError::Core(godwit_core::PasteurError::Provider(e.to_string()))
        })?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            return Err(ApiError::Core(godwit_core::PasteurError::Provider(
                format!("HTTP {}: {}", status, text)
            )));
        }

        let value: serde_json::Value = res.json().await.map_err(|e| {
            ApiError::Core(godwit_core::PasteurError::Provider(format!(
                "failed to deserialize rerank response: {}",
                e
            )))
        })?;

        let normalized = self.normalize_response(value, &provider.name);

        Ok(axum::Json(normalized).into_response())
    }

    fn normalize_response(
        &self,
        response: serde_json::Value,
        _provider_name: &str,
    ) -> serde_json::Value {
        if let Some(obj) = response.as_object() {
            let id = obj
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("rerank-unknown")
                .to_string();

            let model = obj
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let results = obj
                .get("results")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));

            serde_json::json!({
                "id": id,
                "model": model,
                "results": results
            })
        } else {
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_config_default() {
        let config = RerankFallbackConfig::default();
        assert_eq!(config.providers.len(), 3);
        assert_eq!(config.providers[0].name, "cohere");
        assert_eq!(config.providers[1].name, "azure");
        assert_eq!(config.providers[2].name, "self-hosted");
        assert_eq!(config.timeout_per_provider.as_secs(), 15);
    }

    #[test]
    fn test_config_from_custom_json() {
        let config = json!({
            "rerank_providers": [
                {
                    "name": "custom-cohere",
                    "base_url": "https://api.cohere.ai/v1",
                    "api_key": "test-key",
                    "model": "rerank-multilingual-v3.0"
                },
                {
                    "name": "custom-azure",
                    "base_url": "https://custom.openai.azure.com/openai",
                    "model": "azure-rerank-v1"
                }
            ],
            "rerank_timeout_per_provider_secs": 20
        });
        let fallback = RerankFallback::from_config_value(Some(&config));
        assert_eq!(fallback.config.providers.len(), 2);
        assert_eq!(fallback.config.providers[0].name, "custom-cohere");
        assert_eq!(fallback.config.providers[1].name, "custom-azure");
        assert_eq!(fallback.config.timeout_per_provider.as_secs(), 20);
    }

    #[test]
    fn test_config_from_empty_json() {
        let config = json!({});
        let fallback = RerankFallback::from_config_value(Some(&config));
        assert_eq!(fallback.config.providers.len(), 3);
        assert_eq!(fallback.config.timeout_per_provider.as_secs(), 15);
    }

    #[test]
    fn test_normalize_response() {
        let config = RerankFallbackConfig::default();
        let fallback = RerankFallback::new(config);

        let input = json!({
            "id": "rerank-123",
            "model": "rerank-english-v3.0",
            "results": [
                {
                    "index": 0,
                    "relevance_score": 0.95
                },
                {
                    "index": 1,
                    "relevance_score": 0.82
                }
            ]
        });

        let normalized = fallback.normalize_response(input, "cohere");

        assert_eq!(normalized["id"], "rerank-123");
        assert_eq!(normalized["model"], "rerank-english-v3.0");
        assert!(normalized["results"].is_array());
        assert_eq!(normalized["results"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_normalize_response_missing_fields() {
        let config = RerankFallbackConfig::default();
        let fallback = RerankFallback::new(config);

        let input = json!({
            "results": []
        });

        let normalized = fallback.normalize_response(input, "azure");

        assert_eq!(normalized["id"], "rerank-unknown");
        assert_eq!(normalized["model"], "unknown");
        assert!(normalized["results"].is_array());
    }

    #[test]
    fn test_normalize_response_cohere_format() {
        let config = RerankFallbackConfig::default();
        let fallback = RerankFallback::new(config);

        let input = json!({
            "id": "cohere-rerank-456",
            "model": "rerank-english-v3.0",
            "results": [
                {
                    "index": 2,
                    "relevance_score": 0.98
                }
            ]
        });

        let normalized = fallback.normalize_response(input, "cohere");

        assert_eq!(normalized["id"], "cohere-rerank-456");
        assert_eq!(normalized["model"], "rerank-english-v3.0");
        assert!(normalized["results"].is_array());
        assert_eq!(normalized["results"].as_array().unwrap()[0]["index"], 2);
        assert_eq!(normalized["results"].as_array().unwrap()[0]["relevance_score"], 0.98);
    }

    #[test]
    fn test_timeout_config_respected() {
        let config = json!({
            "rerank_providers": [
                {
                    "name": "fast-provider",
                    "base_url": "https://api.example.com/v1",
                    "model": "rerank-v1"
                }
            ],
            "rerank_timeout_per_provider_secs": 5
        });
        let fallback = RerankFallback::from_config_value(Some(&config));
        assert_eq!(fallback.config.timeout_per_provider.as_secs(), 5);
    }

    #[test]
    fn test_fallback_chain_order() {
        let config = RerankFallbackConfig::default();
        let fallback = RerankFallback::new(config);
        
        assert_eq!(fallback.config.providers.len(), 3);
        assert_eq!(fallback.config.providers[0].name, "cohere");
        assert_eq!(fallback.config.providers[1].name, "azure");
        assert_eq!(fallback.config.providers[2].name, "self-hosted");
    }
}
