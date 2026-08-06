use crate::adapter::{
    Adapter, ProviderError, ProviderResponse, ResolvedProfile, SseEvent, UsageReport,
};
use crate::streaming::{normalize_openai_sse_event, parse_sse_events};
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use godwit_core::{
    AudioSttRequest, AudioSttResponse, AudioTtsRequest, Capability, ChatCompletionRequest,
    ChatCompletionResponse, ChatContent, ChatContentPart, EmbeddingRequest, EmbeddingResponse,
    ImageGenerationRequest, ImageGenerationResponse,
};
use godwit_db::models::Model;
use reqwest::Client;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct OpenAiImageUrl {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenAiContentPart {
    Text { text: String },
    ImageUrl { image_url: OpenAiImageUrl },
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: Option<Vec<OpenAiContentPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

impl OpenAiMessage {
    fn from_chat_message(msg: &godwit_core::ChatMessage) -> Self {
        let content = msg.content.as_ref().map(|contents| {
            contents
                .iter()
                .flat_map(|c| match c {
                    ChatContent::Text(text) => vec![OpenAiContentPart::Text { text: text.clone() }],
                    ChatContent::Parts(parts) => parts
                        .iter()
                        .map(|p| match p {
                            ChatContentPart::Text { text } => {
                                OpenAiContentPart::Text { text: text.clone() }
                            }
                            ChatContentPart::ImageUrl { image_url } => {
                                OpenAiContentPart::ImageUrl {
                                    image_url: OpenAiImageUrl {
                                        url: image_url.url.clone(),
                                        detail: image_url.detail.clone(),
                                    },
                                }
                            }
                        })
                        .collect(),
                })
                .collect()
        });

        Self {
            role: msg.role.clone(),
            content,
            name: msg.name.clone(),
            tool_calls: msg.tool_calls.as_ref().map(|tc| serde_json::to_value(tc).unwrap()),
            tool_call_id: msg.tool_call_id.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<godwit_core::Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<godwit_core::ToolChoice>,
}

pub struct OpenAiProvider {
    client: Client,
}

pub type OpenAiAdapter = OpenAiProvider;

impl OpenAiProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("build reqwest client");
        Self { client }
    }
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for OpenAiProvider {
    fn supported_capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Chat,
            Capability::ImageGeneration,
            Capability::AudioTts,
            Capability::AudioStt,
            Capability::Embedding,
        ]
    }

    async fn chat(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        // Translate the catalog/wildcard-resolved public id into the upstream model id.
        request.model = model.provider_model_id.clone();
        let openai_messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(OpenAiMessage::from_chat_message)
            .collect();
        let openai_request = OpenAiChatRequest {
            model: request.model.clone(),
            messages: openai_messages,
            stream: request.stream,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: request.tools.clone(),
            tool_choice: request.tool_choice.clone(),
        };
        let url = format!("{}/chat/completions", profile.base_url);
        let mut req = self.client.post(&url).json(&openai_request);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
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
        let usage = crate::usage::chat_usage_report(&body.usage);
        Ok((ProviderResponse::Chat(body), usage))
    }

    async fn chat_stream(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        request.stream = Some(true);
        // Translate the catalog/wildcard-resolved public id into the upstream model id.
        request.model = model.provider_model_id.clone();
        let openai_messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(OpenAiMessage::from_chat_message)
            .collect();
        let openai_request = OpenAiChatRequest {
            model: request.model.clone(),
            messages: openai_messages,
            stream: request.stream,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: request.tools.clone(),
            tool_choice: request.tool_choice.clone(),
        };
        let url = format!("{}/chat/completions", profile.base_url);
        let mut req = self.client.post(&url).json(&openai_request);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
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
            let raw_events = parse_sse_events(&text);
            let normalized: Vec<Result<SseEvent, ProviderError>> = raw_events
                .into_iter()
                .flat_map(normalize_openai_sse_event)
                .collect();
            stream::iter(normalized)
        });
        Ok(event_stream.boxed())
    }

    async fn image_generation(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: ImageGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        // Translate the catalog/wildcard-resolved public id into the upstream model id.
        request.model = model.provider_model_id.clone();
        let url = format!("{}/images/generations", profile.base_url);
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
            status: 0,
            message: e.to_string(),
        })?;
        if !res.status().is_success() {
            return Err(ProviderError::Http {
                status: res.status().as_u16(),
                message: res.text().await.unwrap_or_default(),
            });
        }
        let body: ImageGenerationResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        let usage = UsageReport {
            image_count: Some(body.data.len() as i64),
            ..Default::default()
        };
        Ok((ProviderResponse::Image(body), usage))
    }

    async fn image_edit(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: godwit_core::ImageEditRequest,
        image_bytes: Vec<u8>,
        image_filename: String,
        mask_bytes: Option<Vec<u8>>,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        // Translate the catalog/wildcard-resolved public id into the upstream model id.
        request.model = model.provider_model_id.clone();
        let url = format!("{}/images/edits", profile.base_url);
        let image_part = reqwest::multipart::Part::bytes(image_bytes)
            .file_name(image_filename)
            .mime_str("image/png")
            .map_err(|e| ProviderError::Provider(e.to_string()))?;
        let mut form = reqwest::multipart::Form::new()
            .part("image", image_part)
            .text("model", request.model)
            .text("prompt", request.prompt);
        if let Some(mask) = mask_bytes {
            let mask_part = reqwest::multipart::Part::bytes(mask)
                .file_name("mask.png")
                .mime_str("image/png")
                .map_err(|e| ProviderError::Provider(e.to_string()))?;
            form = form.part("mask", mask_part);
        }
        if let Some(n) = request.n {
            form = form.text("n", n.to_string());
        }
        if let Some(size) = request.size {
            form = form.text("size", size);
        }
        if let Some(response_format) = request.response_format {
            form = form.text("response_format", response_format);
        }
        let mut req = self.client.post(&url).multipart(form);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
            status: 0,
            message: e.to_string(),
        })?;
        if !res.status().is_success() {
            return Err(ProviderError::Http {
                status: res.status().as_u16(),
                message: res.text().await.unwrap_or_default(),
            });
        }
        let body: godwit_core::ImageGenerationResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        let usage = UsageReport {
            image_count: Some(body.data.len() as i64),
            ..Default::default()
        };
        Ok((ProviderResponse::Image(body), usage))
    }

    async fn video_generation(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: godwit_core::VideoGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "video generation is not supported for OpenAI".to_string(),
        ))
    }

    async fn audio_tts(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        // Translate the catalog/wildcard-resolved public id into the upstream model id.
        request.model = model.provider_model_id.clone();
        let url = format!("{}/audio/speech", profile.base_url);
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
            status: 0,
            message: e.to_string(),
        })?;
        if !res.status().is_success() {
            return Err(ProviderError::Http {
                status: res.status().as_u16(),
                message: res.text().await.unwrap_or_default(),
            });
        }
        let content_type = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("audio/mpeg")
            .to_string();
        let bytes = res
            .bytes()
            .await
            .map_err(|e| ProviderError::Http {
                status: 0,
                message: e.to_string(),
            })?
            .to_vec();
        let usage = UsageReport {
            tts_characters: Some(request.input.chars().count() as i64),
            ..Default::default()
        };
        Ok((
            ProviderResponse::Bytes(bytes, content_type),
            usage,
        ))
    }

    async fn audio_stt(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: AudioSttRequest,
        file_bytes: Vec<u8>,
        filename: String,
        content_type: String,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        // Translate the catalog/wildcard-resolved public id into the upstream model id.
        request.model = model.provider_model_id.clone();
        let url = format!("{}/audio/transcriptions", profile.base_url);
        let file_part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(filename)
            .mime_str(&content_type)
            .map_err(|e| ProviderError::Provider(e.to_string()))?;
        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", request.model);
        if let Some(language) = request.language {
            form = form.text("language", language);
        }
        if let Some(response_format) = request.response_format {
            form = form.text("response_format", response_format);
        }
        let mut req = self.client.post(&url).multipart(form);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
            status: 0,
            message: e.to_string(),
        })?;
        if !res.status().is_success() {
            return Err(ProviderError::Http {
                status: res.status().as_u16(),
                message: res.text().await.unwrap_or_default(),
            });
        }
        let body: AudioSttResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        let usage = UsageReport {
            audio_seconds: Some(0.0),
            ..Default::default()
        };
        Ok((ProviderResponse::AudioStt(body), usage))
    }

    async fn embedding(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: EmbeddingRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        // Translate the catalog/wildcard-resolved public id into the upstream model id.
        request.model = model.provider_model_id.clone();
        let url = format!("{}/embeddings", profile.base_url);
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
            status: 0,
            message: e.to_string(),
        })?;
        if !res.status().is_success() {
            return Err(ProviderError::Http {
                status: res.status().as_u16(),
                message: res.text().await.unwrap_or_default(),
            });
        }
        let body: EmbeddingResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok((
            ProviderResponse::Embedding(body.clone()),
            UsageReport {
                embedding_tokens: Some(body.usage.total_tokens as i64),
                ..Default::default()
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use godwit_core::{
        AudioSttRequest, AudioTtsRequest, ChatCompletionRequest, ChatContent, ChatMessage,
        EmbeddingRequest, ImageGenerationRequest,
    };
    use uuid::Uuid;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    fn dummy_model() -> Model {
        Model {
            id: Uuid::nil(),
            public_id: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            provider_profile_id: Uuid::nil(),
            provider_model_id: "gpt-4o".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    /// A catalog row whose friendly `public_id` differs from the real upstream
    /// `provider_model_id`, so tests can prove which of the two is actually sent upstream.
    fn mapped_model(public_id: &str, provider_model_id: &str) -> Model {
        Model {
            public_id: public_id.to_string(),
            provider_model_id: provider_model_id.to_string(),
            ..dummy_model()
        }
    }

    /// Regression guard for the whole point of `models.provider_model_id`: the client's
    /// `request.model` (a `public_id`, or a full `<profile>/<suffix>` wildcard ref) must be
    /// replaced by the upstream id before the request leaves the adapter.
    #[tokio::test]
    async fn chat_sends_provider_model_id_not_public_id() {
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "chatcmpl-123",
                    "object": "chat.completion",
                    "created": 1,
                    "model": "gpt-4o-2024-08-06",
                    "choices": [{"index":0,"message":{"role":"assistant","content":"Hello"},"finish_reason":"stop"}],
                    "usage": {"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
                }))
            })
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        // The client asked for the friendly catalog id...
        let req = ChatCompletionRequest {
            model: "my-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Hi")]),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };
        let model = mapped_model("my-4o", "gpt-4o-2024-08-06");
        let _ = client.chat(&profile, &model, req).await.unwrap();

        let body = captured_body
            .lock()
            .unwrap()
            .take()
            .expect("request body captured");
        // ...but the upstream must see the mapped provider_model_id.
        assert_eq!(body["model"], "gpt-4o-2024-08-06");
        assert_ne!(body["model"], "my-4o");
    }

    #[tokio::test]
    async fn chat_forwards_native_web_search_tools() {
        use godwit_core::{FunctionDefinition, Tool};
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "chatcmpl-1",
                    "object": "chat.completion",
                    "created": 1,
                    "model": "gpt-4o",
                    "choices": [{"index":0,"message":{"role":"assistant","content":"Hello"},"finish_reason":"stop"}],
                    "usage": {"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
                }))
            })
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "my-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("search the web")]),
                name: None,
                ..Default::default()
            }],
            tools: Some(vec![Tool {
                r#type: "function".to_string(),
                function: FunctionDefinition {
                    name: "web_search".to_string(),
                    description: None,
                    parameters: None,
                },
            }]),
            ..Default::default()
        };
        let model = mapped_model("my-4o", "gpt-4o-2024-08-06");
        let _ = client.chat(&profile, &model, req).await.unwrap();

        let body = captured_body.lock().unwrap().take().expect("request body captured");
        let tools = body["tools"].as_array().expect("tools present");
        assert_eq!(tools[0]["function"]["name"], "web_search");
    }

    #[tokio::test]
    async fn chat_multimodal_text_and_image_url() {
        use godwit_core::{ChatContentPart, ImageUrl};
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "chatcmpl-1",
                    "object": "chat.completion",
                    "created": 1,
                    "model": "gpt-4o",
                    "choices": [{"index":0,"message":{"role":"assistant","content":"I see an image"},"finish_reason":"stop"}],
                    "usage": {"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}
                }))
            })
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![
                    ChatContent::Text("What's in this image?".to_string()),
                    ChatContent::Parts(vec![
                        ChatContentPart::Text { text: "Describe this:".to_string() },
                        ChatContentPart::ImageUrl { image_url: ImageUrl { url: "https://example.com/img.png".to_string(), detail: Some("high".to_string()) } },
                    ]),
                ]),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };
        let _ = client.chat(&profile, &dummy_model(), req).await.unwrap();

        let body = captured_body.lock().unwrap().take().expect("request body captured");
        let messages = body["messages"].as_array().expect("messages present");
        assert_eq!(messages.len(), 1);
        let content = messages[0]["content"].as_array().expect("content is array");
        assert_eq!(content.len(), 3);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "What's in this image?");
        assert_eq!(content[1]["type"], "text");
        assert_eq!(content[1]["text"], "Describe this:");
        assert_eq!(content[2]["type"], "image_url");
        assert_eq!(content[2]["image_url"]["url"], "https://example.com/img.png");
        assert_eq!(content[2]["image_url"]["detail"], "high");
    }

    #[tokio::test]
    async fn embedding_sends_provider_model_id_not_public_id() {
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "object": "list",
                    "data": [{"object": "embedding", "embedding": [0.1], "index": 0}],
                    "model": "text-embedding-3-small",
                    "usage": {"prompt_tokens": 2, "completion_tokens": 0, "total_tokens": 2}
                }))
            })
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = EmbeddingRequest {
            model: "my-embedder".to_string(),
            input: vec!["hello".to_string()],
        };
        let model = mapped_model("my-embedder", "text-embedding-3-small");
        let _ = client.embedding(&profile, &model, req).await.unwrap();

        let body = captured_body
            .lock()
            .unwrap()
            .take()
            .expect("request body captured");
        assert_eq!(body["model"], "text-embedding-3-small");
        assert_ne!(body["model"], "my-embedder");
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

        let client = OpenAiAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Hi")]),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };
        let (resp, usage) = client.chat(&profile, &dummy_model(), req).await.unwrap();
        let ProviderResponse::Chat(completion) = resp else {
            panic!("expected chat response");
        };
        assert_eq!(completion.choices[0].message.content_as_text(), Some("Hello".to_string()));
        assert_eq!(usage.prompt_tokens, Some(1));
        assert_eq!(usage.completion_tokens, Some(1));
    }

    #[tokio::test]
    async fn openai_image_generation() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "created": 1,
            "data": [{"url": "https://example.com/image.png", "b64_json": null, "revised_prompt": null}]
        });
        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ImageGenerationRequest {
            model: "dall-e-3".to_string(),
            prompt: "a cat in a hat".to_string(),
            n: Some(1),
            size: Some("1024x1024".to_string()),
            quality: Some("hd".to_string()),
            style: Some("vivid".to_string()),
        };
        let (ProviderResponse::Image(resp), _) = client
            .image_generation(&profile, &dummy_model(), req)
            .await
            .unwrap()
        else {
            panic!("expected image response");
        };
        assert_eq!(
            resp.data[0].url.as_deref().unwrap(),
            "https://example.com/image.png"
        );
    }

    #[tokio::test]
    async fn openai_image_generation_propagates_http_error() {
        let server = MockServer::start().await;
        let body = serde_json::json!({ "error": "Internal Server Error" });
        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .respond_with(ResponseTemplate::new(500).set_body_json(body))
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ImageGenerationRequest {
            model: "dall-e-3".to_string(),
            prompt: "a cat in a hat".to_string(),
            n: None,
            size: None,
            quality: None,
            style: None,
        };
        let err = client
            .image_generation(&profile, &dummy_model(), req)
            .await
            .unwrap_err();
        match err {
            ProviderError::Http { status, .. } => assert_eq!(status, 500),
            _ => panic!("expected http error, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn openai_image_edit() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "created": 1,
            "data": [{"url": "https://example.com/edited.png", "b64_json": null, "revised_prompt": null}]
        });
        // Assert on the outgoing multipart form, not just the response: without these
        // matchers the test would still pass if prompt/n/size/mask/response_format were
        // silently dropped from the form.
        Mock::given(method("POST"))
            .and(path("/images/edits"))
            .and(body_string_contains("name=\"prompt\""))
            .and(body_string_contains("add a hat"))
            .and(body_string_contains("name=\"model\""))
            .and(body_string_contains("gpt-image-1"))
            .and(body_string_contains("name=\"image\""))
            .and(body_string_contains("edit-me.png"))
            .and(body_string_contains("name=\"mask\""))
            .and(body_string_contains("mask.png"))
            .and(body_string_contains("name=\"n\""))
            .and(body_string_contains("name=\"size\""))
            .and(body_string_contains("1024x1024"))
            .and(body_string_contains("name=\"response_format\""))
            .and(body_string_contains("b64_json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = godwit_core::ImageEditRequest {
            model: "my-image-editor".to_string(),
            prompt: "add a hat".to_string(),
            n: Some(2),
            size: Some("1024x1024".to_string()),
            response_format: Some("b64_json".to_string()),
        };
        let (resp, _usage) = client
            .image_edit(
                &profile,
                &mapped_model("my-image-editor", "gpt-image-1"),
                req,
                vec![1, 2, 3],
                "edit-me.png".to_string(),
                Some(vec![4, 5, 6]),
            )
            .await
            .unwrap();
        let ProviderResponse::Image(image) = resp else {
            panic!("expected image response")
        };
        assert_eq!(
            image.data[0].url.as_deref(),
            Some("https://example.com/edited.png")
        );

        // The form must carry the upstream provider_model_id, never the public_id.
        let received = server
            .received_requests()
            .await
            .expect("request recording enabled");
        let form = String::from_utf8_lossy(&received[0].body);
        assert!(
            !form.contains("my-image-editor"),
            "the client-supplied public_id must not be forwarded upstream, got: {form}"
        );
    }

    #[tokio::test]
    async fn openai_audio_tts() {
        let server = MockServer::start().await;
        let audio = b"fake-audio-bytes".to_vec();
        Mock::given(method("POST"))
            .and(path("/audio/speech"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "audio/mpeg")
                    .set_body_bytes(audio.clone()),
            )
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = AudioTtsRequest {
            model: "tts-1".to_string(),
            input: "Hello world".to_string(),
            voice: "alloy".to_string(),
            response_format: Some("mp3".to_string()),
        };
        let (ProviderResponse::Bytes(resp_bytes, resp_content_type), _) = client
            .audio_tts(&profile, &dummy_model(), req)
            .await
            .unwrap()
        else {
            panic!("expected audio tts response");
        };
        assert_eq!(resp_bytes, audio);
        assert_eq!(resp_content_type, "audio/mpeg");
    }

    #[tokio::test]
    async fn openai_audio_stt() {
        let server = MockServer::start().await;
        let body = serde_json::json!({ "text": "hello there" });
        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            // The multipart form must carry the upstream provider_model_id, not the
            // friendly public_id the client sent.
            .and(body_string_contains("whisper-1"))
            .and(body_string_contains("recording.mp3"))
            .and(body_string_contains("audio/mpeg"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = AudioSttRequest {
            model: "my-transcriber".to_string(),
            language: Some("en".to_string()),
            response_format: Some("json".to_string()),
        };
        let (ProviderResponse::AudioStt(resp), _) = client
            .audio_stt(
                &profile,
                &mapped_model("my-transcriber", "whisper-1"),
                req,
                b"fake-audio-data".to_vec(),
                "recording.mp3".to_string(),
                "audio/mpeg".to_string(),
            )
            .await
            .unwrap()
        else {
            panic!("expected audio stt response");
        };
        assert_eq!(resp.text, "hello there");

        let received = server
            .received_requests()
            .await
            .expect("request recording enabled");
        let form = String::from_utf8_lossy(&received[0].body);
        assert!(
            !form.contains("my-transcriber"),
            "the client-supplied public_id must not be forwarded upstream, got: {form}"
        );
    }

    #[tokio::test]
    async fn openai_embedding() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "object": "list",
            "data": [{"object": "embedding", "embedding": [0.1, 0.2, 0.3], "index": 0}],
            "model": "text-embedding-3-small",
            "usage": {"prompt_tokens": 2, "completion_tokens": 0, "total_tokens": 2}
        });
        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: vec!["hello".to_string()],
        };
        let (ProviderResponse::Embedding(resp), _) = client
            .embedding(&profile, &dummy_model(), req)
            .await
            .unwrap()
        else {
            panic!("expected embedding response");
        };
        assert_eq!(resp.model, "text-embedding-3-small");
        assert_eq!(resp.data[0].embedding.len(), 3);
    }

    #[test]
    fn test_image_usage_estimate() {
        let usage = UsageReport {
            image_count: Some(4),
            ..Default::default()
        };
        
        assert_eq!(usage.image_count, Some(4));
    }

    #[test]
    fn test_tts_usage_estimate() {
        let input = "Hello, world!";
        let usage = UsageReport {
            tts_characters: Some(input.chars().count() as i64),
            ..Default::default()
        };
        
        assert_eq!(usage.tts_characters, Some(13));
    }
}
