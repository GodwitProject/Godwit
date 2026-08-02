use crate::{Provider, ProviderError, ProviderResponse, SseEvent};
use async_trait::async_trait;
use futures::stream::BoxStream;
use pasteurllm_core::{ChatCompletionRequest, ChatCompletionResponse, ProviderConfig};
use reqwest::Client;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self {
            client,
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
        }
    }

    pub fn from_config(config: &ProviderConfig) -> Self {
        Self::new(&config.api_key, &config.base_url)
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ProviderResponse, ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(ProviderError::Provider(text));
        }
        let body: ChatCompletionResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok(ProviderResponse::Json(body))
    }

    async fn stream_chat_completion(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        Err(ProviderError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pasteurllm_core::{ChatCompletionRequest, ChatMessage};
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn chat_completion_returns_openai_shape() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4o",
            "choices": [{"index":0,"message":{"role":"assistant","content":"Hello"},"finish_reason":"stop"}],
            "usage": {"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
        });
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = OpenAiProvider::new("fake-key", &server.uri());
        let req = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage { role: "user".to_string(), content: "Hi".to_string() }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        };
        let ProviderResponse::Json(resp) = client.chat_completion(req).await.unwrap();
        assert_eq!(resp.choices[0].message.content, "Hello");
    }
}
