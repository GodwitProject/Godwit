use crate::adapter::{Adapter, ProviderError, ProviderResponse, SseEvent, UsageReport};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::BoxStream;
use godwit_core::{
    AudioSttRequest, AudioTtsRequest, Capability, ChatCompletionChoice, ChatCompletionRequest,
    ChatCompletionResponse, ChatMessage, EmbeddingRequest, ImageGenerationRequest,
    VideoGenerationRequest,
};
use godwit_db::models::{Model, ProviderProfile};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};

pub struct GeminiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

pub type GeminiAdapter = GeminiProvider;

impl GeminiProvider {
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

    pub fn from_config(config: &godwit_core::ProviderConfig) -> Self {
        Self::new(&config.api_key, &config.base_url)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiChatRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<String>,
    contents: Vec<GeminiContent>,
    generation_config: GeminiGenerationConfig,
}

impl GeminiChatRequest {
    fn from_chat_request(request: ChatCompletionRequest) -> Self {
        let mut system_parts: Vec<String> = Vec::new();
        let mut contents: Vec<GeminiContent> = Vec::new();

        for msg in request.messages {
            if msg.role == "system" {
                system_parts.push(msg.content);
            } else {
                let role = if msg.role == "assistant" {
                    "model".to_string()
                } else {
                    msg.role
                };
                contents.push(GeminiContent {
                    role,
                    parts: vec![GeminiPart { text: msg.content }],
                });
            }
        }

        let system_instruction = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        Self {
            system_instruction,
            contents,
            generation_config: GeminiGenerationConfig {
                max_output_tokens: Some(request.max_tokens.unwrap_or(4096)),
                temperature: request.temperature,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiPartResponse {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiContentResponse {
    #[serde(default)]
    parts: Vec<GeminiPartResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: GeminiContentResponse,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    #[serde(default)]
    prompt_token_count: i32,
    #[serde(default)]
    candidates_token_count: i32,
    #[serde(default)]
    total_token_count: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiChatResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
}

fn gemini_response_to_chat_completion(
    response: GeminiChatResponse,
    model_id: &str,
) -> Result<ChatCompletionResponse, ProviderError> {
    let candidate = response
        .candidates
        .into_iter()
        .next()
        .ok_or_else(|| ProviderError::Provider("Gemini response contained no candidates".to_string()))?;

    let content = candidate
        .content
        .parts
        .into_iter()
        .filter_map(|part| part.text)
        .collect::<Vec<_>>()
        .join("");

    let usage = response.usage_metadata.map(|usage| godwit_core::Usage {
        prompt_tokens: usage.prompt_token_count,
        completion_tokens: usage.candidates_token_count,
        total_tokens: usage.total_token_count,
    });

    Ok(ChatCompletionResponse {
        id: uuid::Uuid::new_v4().to_string(),
        object: "chat.completion".to_string(),
        created: Utc::now().timestamp(),
        model: model_id.to_string(),
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content,
            },
            finish_reason: candidate.finish_reason,
        }],
        usage,
    })
}

#[async_trait]
impl Adapter for GeminiProvider {
    fn supported_capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat]
    }

    #[instrument(skip(self, _profile, model, request))]
    async fn chat(
        &self,
        _profile: &ProviderProfile,
        model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url, model.public_id, self.api_key
        );
        let gemini_request = GeminiChatRequest::from_chat_request(request);

        info!("sending gemini chat request to {}", url);
        debug!("gemini request body: {:?}", gemini_request);

        let res = self
            .client
            .post(&url)
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| ProviderError::Http {
                status: 0,
                message: e.to_string(),
            })?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            error!("gemini chat request failed with status {}: {}", status, text);
            return Err(ProviderError::Http { status, message: text });
        }

        let body: GeminiChatResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;

        debug!("gemini response body: {:?}", body);

        let chat_response = gemini_response_to_chat_completion(body, &model.public_id)?;
        let usage_report = if let Some(ref usage) = chat_response.usage {
            UsageReport {
                prompt_tokens: Some(usage.prompt_tokens),
                completion_tokens: Some(usage.completion_tokens),
                ..Default::default()
            }
        } else {
            UsageReport::default()
        };

        Ok((ProviderResponse::Chat(chat_response), usage_report))
    }

    async fn chat_stream(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "streaming is not supported for Gemini".to_string(),
        ))
    }

    async fn image_generation(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: ImageGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "image generation is not supported for Gemini".to_string(),
        ))
    }

    async fn video_generation(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: VideoGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "video generation is not supported for Gemini".to_string(),
        ))
    }

    async fn audio_tts(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "audio TTS is not supported for Gemini".to_string(),
        ))
    }

    async fn audio_stt(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: AudioSttRequest,
        _file_bytes: Vec<u8>,
        _filename: String,
        _content_type: String,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "audio STT is not supported for Gemini".to_string(),
        ))
    }

    async fn embedding(
        &self,
        _profile: &ProviderProfile,
        _model: &Model,
        _request: EmbeddingRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "embedding is not supported for Gemini".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use godwit_core::ChatCompletionRequest;
    use uuid::Uuid;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    fn dummy_profile() -> ProviderProfile {
        ProviderProfile {
            id: Uuid::nil(),
            organization_id: Uuid::nil(),
            name: "gemini".to_string(),
            protocol: "gemini".to_string(),
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
            public_id: "gemini-1.5-flash".to_string(),
            provider: "gemini".to_string(),
            provider_profile_id: Uuid::nil(),
            provider_model_id: "gemini-1.5-flash".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    fn chat_request_with_system() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are a helpful assistant.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                },
                ChatMessage {
                    role: "assistant".to_string(),
                    content: "Hi there".to_string(),
                },
            ],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        }
    }

    #[tokio::test]
    async fn chat_request_url_includes_model_and_key() {
        let server = MockServer::start().await;
        let captured_url = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let captured_clone = captured_url.clone();

        Mock::given(method("POST"))
            .respond_with(move |req: &wiremock::Request| {
                *captured_clone.lock().unwrap() = Some(req.url.to_string());
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "candidates": [{
                        "content": {
                            "role": "model",
                            "parts": [{"text": "Hi"}]
                        }
                    }]
                }))
            })
            .mount(&server)
            .await;

        let client = GeminiProvider::new("fake-key", &server.uri());
        let req = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        };

        let _ = client.chat(&dummy_profile(), &dummy_model(), req).await.unwrap();

        let url = captured_url.lock().unwrap().take().expect("request url captured");
        assert!(url.contains("/v1beta/models/gemini-1.5-flash:generateContent"), "url={}", url);
        assert!(url.contains("?key=fake-key"), "url={}", url);
    }

    #[tokio::test]
    async fn chat_request_body_serializes_system_contents_and_generation_config() {
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "candidates": [{
                        "content": {
                            "role": "model",
                            "parts": [{"text": "Hi"}]
                        }
                    }]
                }))
            })
            .mount(&server)
            .await;

        let client = GeminiProvider::new("fake-key", &server.uri());
        let _ = client
            .chat(&dummy_profile(), &dummy_model(), chat_request_with_system())
            .await
            .unwrap();

        let body = captured_body.lock().unwrap().take().expect("request body captured");
        assert_eq!(
            body["systemInstruction"],
            "You are a helpful assistant."
        );
        assert_eq!(body["contents"].as_array().unwrap().len(), 2);
        assert_eq!(body["contents"][0]["role"], "user");
        assert_eq!(body["contents"][0]["parts"][0]["text"], "Hello");
        assert_eq!(body["contents"][1]["role"], "model");
        assert_eq!(body["contents"][1]["parts"][0]["text"], "Hi there");
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 4096);
        assert!(body["generationConfig"]["temperature"].is_null());
    }

    #[tokio::test]
    async fn chat_parses_non_streaming_response_and_usage() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-1.5-flash:generateContent"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "candidates": [{
                    "content": {
                        "role": "model",
                        "parts": [
                            {"text": "Hello, "},
                            {"text": "world!"}
                        ]
                    },
                    "finishReason": "STOP"
                }],
                "usageMetadata": {
                    "promptTokenCount": 10,
                    "candidatesTokenCount": 5,
                    "totalTokenCount": 15
                }
            })))
            .mount(&server)
            .await;

        let client = GeminiProvider::new("fake-key", &server.uri());
        let req = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        };

        let (ProviderResponse::Chat(resp), usage_report) = client
            .chat(&dummy_profile(), &dummy_model(), req)
            .await
            .unwrap()
        else {
            panic!("expected chat response");
        };

        assert_eq!(resp.choices[0].message.content, "Hello, world!");
        assert_eq!(resp.choices[0].message.role, "assistant");
        assert_eq!(resp.object, "chat.completion");
        assert_eq!(resp.model, "gemini-1.5-flash");
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
        assert_eq!(usage_report.prompt_tokens, Some(10));
        assert_eq!(usage_report.completion_tokens, Some(5));
    }

    #[tokio::test]
    async fn chat_propagates_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-1.5-flash:generateContent"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({ "error": { "message": "bad request" } })),
            )
            .mount(&server)
            .await;

        let client = GeminiProvider::new("fake-key", &server.uri());
        let req = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        };

        let err = client
            .chat(&dummy_profile(), &dummy_model(), req)
            .await
            .unwrap_err();
        match err {
            ProviderError::Http { status, .. } => assert_eq!(status, 400),
            _ => panic!("expected http error, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn unsupported_capabilities_return_error() {
        let client = GeminiProvider::new("fake-key", "https://example.com");
        let profile = dummy_profile();
        let model = dummy_model();

        let image_req = ImageGenerationRequest {
            model: "gemini".to_string(),
            prompt: "a cat".to_string(),
            n: None,
            size: None,
            quality: None,
            style: None,
        };
        let err = client
            .image_generation(&profile, &model, image_req)
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::CapabilityNotSupported(_)));

        let audio_req = AudioTtsRequest {
            model: "gemini".to_string(),
            input: "hello".to_string(),
            voice: "default".to_string(),
            response_format: None,
        };
        let err = client.audio_tts(&profile, &model, audio_req).await.unwrap_err();
        assert!(matches!(err, ProviderError::CapabilityNotSupported(_)));

        let embedding_req = EmbeddingRequest {
            model: "gemini".to_string(),
            input: vec!["hello".to_string()],
        };
        let err = client.embedding(&profile, &model, embedding_req).await.unwrap_err();
        assert!(matches!(err, ProviderError::CapabilityNotSupported(_)));
    }
}
