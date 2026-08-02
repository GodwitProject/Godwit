use crate::adapter::{Adapter, ProviderError, ProviderResponse, SseEvent, UsageReport};
use crate::streaming::parse_sse_events;
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use godwit_core::{Capability, ChatCompletionRequest, ChatCompletionResponse, ProviderConfig};
use godwit_db::models::{Model, ProviderProfile};
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
impl Adapter for OpenAiProvider {
    fn supported_capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat]
    }

    async fn chat(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!("{}/chat/completions", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Http {
                status: 0,
                message: e.to_string(),
            })?;
        if !res.status().is_success() {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            return Err(ProviderError::Http {
                status,
                message: text,
            });
        }
        let body: ChatCompletionResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok((ProviderResponse::Chat(body), UsageReport::default()))
    }

    async fn chat_stream(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        mut request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        request.stream = Some(true);
        let url = format!("{}/chat/completions", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Http {
                status: 0,
                message: e.to_string(),
            })?;
        if !res.status().is_success() {
            return Err(ProviderError::Http {
                status: res.status().as_u16(),
                message: res.text().await.unwrap_or_default(),
            });
        }
        let byte_stream = res.bytes_stream();
        let event_stream = byte_stream.flat_map(|bytes| {
            let text = bytes
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            let events = parse_sse_events(&text);
            stream::iter(events.into_iter().map(Ok))
        });
        Ok(event_stream.boxed())
    }

    async fn image_generation(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: godwit_core::ImageGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported)
    }

    async fn video_generation(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: godwit_core::VideoGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported)
    }

    async fn audio_tts(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: godwit_core::AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported)
    }

    async fn audio_stt(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: godwit_core::AudioSttRequest,
        _file_bytes: Vec<u8>,
        _filename: String,
        _content_type: String,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported)
    }

    async fn embedding(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: godwit_core::EmbeddingRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use godwit_core::{ChatCompletionRequest, ChatMessage};
    use uuid::Uuid;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    fn dummy_profile() -> ProviderProfile {
        ProviderProfile {
            id: Uuid::nil(),
            organization_id: Uuid::nil(),
            name: "openai".to_string(),
            protocol: "openai".to_string(),
            base_url: None,
            auth: serde_json::json!({}),
            config: serde_json::json!({}),
            enabled: true,
            created_at: Utc::now(),
        }
    }

    fn dummy_model() -> Model {
        Model {
            id: Uuid::nil(),
            organization_id: Uuid::nil(),
            public_id: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            provider_profile_id: Uuid::nil(),
            provider_model_id: "gpt-4o".to_string(),
            capability: "chat".to_string(),
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn chat_returns_openai_shape() {
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
        let profile = dummy_profile();
        let model = dummy_model();
        let req = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        };
        let (ProviderResponse::Chat(resp), _) = client.chat(&profile, &model, req).await.unwrap()
        else {
            panic!("expected chat response");
        };
        assert_eq!(resp.choices[0].message.content, "Hello");
    }
}
