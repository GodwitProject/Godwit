use crate::adapter::{
    Adapter, ProviderError, ProviderResponse, ResolvedProfile, SseEvent, UsageReport,
};
use crate::streaming::parse_sse_events;
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::{self, BoxStream, StreamExt};
use godwit_core::{
    AudioSttRequest, AudioTtsRequest, Batch, BatchRequest, Capability, ChatCompletionRequest,
    ChatCompletionResponse, ChatContent, ChatContentPart, ChatMessage, EmbeddingRequest,
    ImageGenerationRequest, VideoGenerationRequest, ResponseFormat,
};
use godwit_db::models::Model;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};

pub struct AnthropicProvider {
    client: Client,
}

pub type AnthropicAdapter = AnthropicProvider;

impl AnthropicProvider {
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

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
struct AnthropicCacheControl {
    r#type: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContentBlock {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<AnthropicCacheControl>,
    },
    Image {
        source: AnthropicImageSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<AnthropicCacheControl>,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AnthropicImageSource {
    r#type: String,
    media_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContentBlock>,
}

impl AnthropicMessage {
    fn from_chat_message(msg: &ChatMessage) -> Self {
        let cache_control = msg.cache_control.as_ref().map(|cc| AnthropicCacheControl {
            r#type: cc.r#type.clone(),
        });

        let content = msg
            .content
            .as_ref()
            .map(|contents| {
                contents
                    .iter()
                    .flat_map(|c| match c {
                        ChatContent::Text(text) => {
                            vec![AnthropicContentBlock::Text {
                                text: text.clone(),
                                cache_control: cache_control.clone(),
                            }]
                        }
                        ChatContent::Parts(parts) => parts
                            .iter()
                            .map(|p| match p {
                                ChatContentPart::Text { text } => {
                                    AnthropicContentBlock::Text {
                                        text: text.clone(),
                                        cache_control: cache_control.clone(),
                                    }
                                }
                                ChatContentPart::ImageUrl { image_url } => {
                                    let (media_type, data) = Self::parse_image_url(&image_url.url);
                                    AnthropicContentBlock::Image {
                                        source: AnthropicImageSource {
                                            r#type: "base64".to_string(),
                                            media_type,
                                            data,
                                        },
                                        cache_control: cache_control.clone(),
                                    }
                                }
                            })
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            role: msg.role.clone(),
            content,
        }
    }

    fn parse_image_url(url: &str) -> (String, String) {
        if url.starts_with("data:") {
            let parts: Vec<&str> = url.split(',').collect();
            if parts.len() == 2 {
                let header = parts[0];
                let data = parts[1];
                let media_type = if header.contains("image/png") {
                    "image/png".to_string()
                } else if header.contains("image/jpeg") || header.contains("image/jpg") {
                    "image/jpeg".to_string()
                } else if header.contains("image/gif") {
                    "image/gif".to_string()
                } else if header.contains("image/webp") {
                    "image/webp".to_string()
                } else {
                    "image/png".to_string()
                };
                return (media_type, data.to_string());
            }
        }
        ("image/png".to_string(), url.to_string())
    }
}

#[derive(Debug, Serialize)]
struct AnthropicJsonSchema {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum AnthropicToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

#[derive(Debug, Serialize)]
pub struct AnthropicChatRequest {
    model: String,
    max_tokens: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<AnthropicToolChoice>,
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: Option<String>,
    input_schema: AnthropicJsonSchema,
}

impl AnthropicChatRequest {
    pub fn from_chat_request(request: ChatCompletionRequest, model_id: String) -> Self {
        let mut system_parts: Vec<String> = Vec::new();
        let mut messages: Vec<AnthropicMessage> = Vec::new();
        let mut tools: Vec<AnthropicTool> = Vec::new();
        let mut tool_choice: Option<AnthropicToolChoice> = None;

        for msg in request.messages {
            if msg.role == "system" {
                if let Some(text) = msg.content_as_text() {
                    system_parts.push(text);
                }
            } else {
                messages.push(AnthropicMessage::from_chat_message(&msg));
            }
        }

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        if let Some(ResponseFormat::JsonSchema { json_schema }) = &request.response_format {
            tools.push(AnthropicTool {
                name: json_schema.name.clone(),
                description: Some(format!("Output must conform to this schema")),
                input_schema: AnthropicJsonSchema {
                    name: json_schema.name.clone(),
                    schema: json_schema.schema.clone(),
                },
            });
            tool_choice = Some(AnthropicToolChoice::Tool {
                name: json_schema.name.clone(),
            });
        }

        Self {
            model: model_id,
            max_tokens: request.max_tokens.unwrap_or(4096),
            temperature: request.temperature,
            system,
            messages,
            stream: request.stream == Some(true),
            tools: if tools.is_empty() { None } else { Some(tools) },
            tool_choice,
        }
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicResponseContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: i32,
    output_tokens: i32,
    #[serde(default)]
    cache_read_input_tokens: Option<i32>,
    #[serde(default)]
    cache_creation_input_tokens: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicMessageResponse {
    id: String,
    #[serde(rename = "type")]
    message_type: String,
    role: String,
    model: String,
    content: Vec<AnthropicResponseContentBlock>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

fn anthropic_response_to_chat_completion(
    response: AnthropicMessageResponse,
) -> ChatCompletionResponse {
    let content = response
        .content
        .into_iter()
        .filter_map(|block| {
            if block.block_type == "text" {
                block.text
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");

    ChatCompletionResponse {
        id: response.id,
        object: "chat.completion".to_string(),
        created: Utc::now().timestamp(),
        model: response.model,
        choices: vec![godwit_core::ChatCompletionChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: Some(vec![ChatContent::Text(content)]),
                name: None,
                ..Default::default()
            },
            finish_reason: response.stop_reason,
            ..Default::default()
        }],
        usage: Some(godwit_core::Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            ..Default::default()
        }),
    }
}

fn build_sse_delta(text: &str) -> String {
    serde_json::json!({"type": "delta", "delta": text}).to_string()
}

fn build_sse_finish(usage: &serde_json::Value) -> String {
    let prompt_tokens = usage["input_tokens"].as_i64().unwrap_or(0) as i32;
    let completion_tokens = usage["output_tokens"].as_i64().unwrap_or(0) as i32;
    serde_json::json!({
        "type": "finish",
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens,
        }
    })
    .to_string()
}

fn build_sse_error(message: &str) -> String {
    serde_json::json!({"type": "error", "message": message}).to_string()
}

fn normalize_anthropic_sse_event(raw: SseEvent) -> Vec<Result<SseEvent, ProviderError>> {
    let parsed: serde_json::Value = match serde_json::from_str(&raw.data) {
        Ok(v) => v,
        Err(e) => {
            return vec![Ok(SseEvent {
                data: build_sse_error(&format!("failed to parse anthropic sse event: {e}")),
            })];
        }
    };

    let event_type = parsed.get("type").and_then(|t| t.as_str());

    match event_type {
        Some("content_block_delta") => {
            if let Some(text) = parsed["delta"]["text"].as_str() {
                vec![Ok(SseEvent {
                    data: build_sse_delta(text),
                })]
            } else {
                vec![]
            }
        }
        Some("message_delta") => {
            if let Some(usage) = parsed.get("usage") {
                vec![Ok(SseEvent {
                    data: build_sse_finish(usage),
                })]
            } else {
                vec![]
            }
        }
        Some("message_start") => {
            debug!("anthropic stream message_start received");
            vec![]
        }
        Some(other) => {
            debug!("ignoring anthropic sse event type: {}", other);
            vec![]
        }
        None => vec![Ok(SseEvent {
            data: build_sse_error("anthropic sse event missing type field"),
        })],
    }
}

#[async_trait]
impl Adapter for AnthropicProvider {
    fn supported_capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat]
    }

    #[instrument(skip(self, profile, model, request))]
    async fn chat(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!("{}/v1/messages", profile.base_url);
        let anthropic_request =
            AnthropicChatRequest::from_chat_request(request, model.provider_model_id.clone());

        info!("sending anthropic chat request to {}", url);
        debug!("anthropic request body: {:?}", anthropic_request);

        let mut req = self
            .client
            .post(&url)
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_request);
        if let Some(key) = &profile.api_key {
            req = req.header("x-api-key", key);
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
            status: 0,
            message: e.to_string(),
        })?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            error!(
                "anthropic chat request failed with status {}: {}",
                status, text
            );
            return Err(ProviderError::Http {
                status,
                message: text,
            });
        }

        let body: AnthropicMessageResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;

        debug!("anthropic response body: {:?}", body);

        let usage_report = UsageReport {
            prompt_tokens: Some(body.usage.input_tokens),
            completion_tokens: Some(body.usage.output_tokens),
            cache_read_tokens: body.usage.cache_read_input_tokens,
            cache_write_tokens: body.usage.cache_creation_input_tokens,
            ..Default::default()
        };

        let chat_response = anthropic_response_to_chat_completion(body);

        Ok((ProviderResponse::Chat(chat_response), usage_report))
    }

    #[instrument(skip(self, profile, model, request))]
    async fn chat_stream(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        request.stream = Some(true);
        let url = format!("{}/v1/messages", profile.base_url);
        let anthropic_request =
            AnthropicChatRequest::from_chat_request(request, model.provider_model_id.clone());

        info!("sending anthropic streaming chat request to {}", url);
        debug!("anthropic streaming request body: {:?}", anthropic_request);

        let mut req = self
            .client
            .post(&url)
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_request);
        if let Some(key) = &profile.api_key {
            req = req.header("x-api-key", key);
        }
        let res = req.send().await.map_err(|e| ProviderError::Http {
            status: 0,
            message: e.to_string(),
        })?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            let text = res.text().await.unwrap_or_default();
            error!(
                "anthropic streaming chat request failed with status {}: {}",
                status, text
            );
            return Err(ProviderError::Http {
                status,
                message: text,
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
                .flat_map(normalize_anthropic_sse_event)
                .collect();
            stream::iter(normalized)
        });

        Ok(event_stream.boxed())
    }

    async fn image_generation(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: ImageGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "image generation is not supported for Anthropic".to_string(),
        ))
    }

    async fn image_edit(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: godwit_core::ImageEditRequest,
        _image_bytes: Vec<u8>,
        _image_filename: String,
        _mask_bytes: Option<Vec<u8>>,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "image edit is not supported by anthropic".to_string(),
        ))
    }

    async fn video_generation(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: VideoGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "video generation is not supported for Anthropic".to_string(),
        ))
    }

    async fn audio_tts(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "audio TTS is not supported for Anthropic".to_string(),
        ))
    }

    async fn audio_stt(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: AudioSttRequest,
        _file_bytes: Vec<u8>,
        _filename: String,
        _content_type: String,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "audio STT is not supported for Anthropic".to_string(),
        ))
    }

    async fn embedding(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: EmbeddingRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "embedding is not supported for Anthropic".to_string(),
        ))
    }

    async fn create_batch(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: BatchRequest,
    ) -> Result<Batch, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "batch is not supported for Anthropic".to_string(),
        ))
    }

    async fn retrieve_batch(
        &self,
        _profile: &ResolvedProfile,
        _batch_id: String,
    ) -> Result<Batch, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "batch is not supported for Anthropic".to_string(),
        ))
    }

    async fn cancel_batch(
        &self,
        _profile: &ResolvedProfile,
        _batch_id: String,
    ) -> Result<Batch, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "batch is not supported for Anthropic".to_string(),
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

    fn dummy_profile() -> crate::adapter::ResolvedProfile {
        crate::adapter::ResolvedProfile {
            base_url: "https://api.anthropic.com".to_string(),
            api_key: Some("fake-key".to_string()),
        }
    }

    fn dummy_model() -> Model {
        Model {
            id: Uuid::nil(),
            public_id: "claude-sonnet".to_string(),
            provider: "anthropic".to_string(),
            provider_profile_id: Uuid::nil(),
            provider_model_id: "claude-3-5-sonnet-20241022".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    fn chat_request_with_system() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: Some(vec![ChatContent::text("You are a helpful assistant.")]),
                    name: None,
                    ..Default::default()
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: Some(vec![ChatContent::text("Hello")]),
                    name: None,
                    ..Default::default()
                },
            ],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn chat_request_body_has_model_system_and_default_max_tokens() {
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "msg_01",
                    "type": "message",
                    "role": "assistant",
                    "model": "claude-3-5-sonnet-20241022",
                    "content": [{"type": "text", "text": "Hi there"}],
                    "stop_reason": "end_turn",
                    "usage": {"input_tokens": 10, "output_tokens": 5}
                }))
            })
            .mount(&server)
            .await;

        let client = AnthropicAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let _ = client
            .chat(&profile, &dummy_model(), chat_request_with_system())
            .await
            .unwrap();

        let body = captured_body
            .lock()
            .unwrap()
            .take()
            .expect("request body captured");
        // The outgoing model must be the fixture's upstream `provider_model_id`, NOT its
        // friendly `public_id` ("claude-sonnet") and not the `request.model` the client
        // sent ("claude-3-5-sonnet"): a catalog row exists precisely to translate those
        // into the real upstream id.
        assert_eq!(body["model"], "claude-3-5-sonnet-20241022");
        assert_ne!(body["model"], "claude-sonnet");
        assert_ne!(body["model"], "claude-3-5-sonnet");
        assert_eq!(body["system"], "You are a helpful assistant.");
        assert_eq!(body["max_tokens"], 4096);
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"][0]["type"], "text");
        assert_eq!(body["messages"][0]["content"][0]["text"], "Hello");
    }

    #[tokio::test]
    async fn chat_parses_non_streaming_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg_01",
                "type": "message",
                "role": "assistant",
                "model": "claude-3-5-sonnet-20241022",
                "content": [{"type": "text", "text": "Hello, world!"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 10, "output_tokens": 5}
            })))
            .mount(&server)
            .await;

        let client = AnthropicAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "claude-3-5-sonnet".to_string(),
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

        let (ProviderResponse::Chat(resp), usage_report) =
            client.chat(&profile, &dummy_model(), req).await.unwrap()
        else {
            panic!("expected chat response");
        };

        assert_eq!(
            resp.choices[0].message.content_as_text(),
            Some("Hello, world!".to_string())
        );
        assert_eq!(resp.choices[0].message.role, "assistant");
        assert_eq!(
            resp.choices[0].finish_reason.as_deref().unwrap(),
            "end_turn"
        );
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
        assert_eq!(usage_report.prompt_tokens, Some(10));
        assert_eq!(usage_report.completion_tokens, Some(5));
    }

    #[tokio::test]
    async fn chat_stream_emits_delta_and_finish_events() {
        let server = MockServer::start().await;
        let sse_body = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_01\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-3-5-sonnet-20241022\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" world\"}}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":5}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n"
        );

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse_body),
            )
            .mount(&server)
            .await;

        let client = AnthropicAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Hi")]),
                name: None,
                ..Default::default()
            }],
            stream: Some(true),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };

        let stream = client
            .chat_stream(&profile, &dummy_model(), req)
            .await
            .unwrap();
        let events: Vec<SseEvent> = stream.filter_map(|r| async move { r.ok() }).collect().await;

        assert_eq!(events.len(), 3);

        let delta1: serde_json::Value = serde_json::from_str(&events[0].data).unwrap();
        assert_eq!(delta1["type"], "delta");
        assert_eq!(delta1["delta"], "Hello");

        let delta2: serde_json::Value = serde_json::from_str(&events[1].data).unwrap();
        assert_eq!(delta2["type"], "delta");
        assert_eq!(delta2["delta"], " world");

        let finish: serde_json::Value = serde_json::from_str(&events[2].data).unwrap();
        assert_eq!(finish["type"], "finish");
        assert_eq!(finish["usage"]["prompt_tokens"], 0);
        assert_eq!(finish["usage"]["completion_tokens"], 5);
        assert_eq!(finish["usage"]["total_tokens"], 5);
    }

    #[tokio::test]
    async fn chat_multimodal_text_and_image_base64() {
        use godwit_core::{ChatContentPart, ImageUrl};
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "msg_01",
                    "type": "message",
                    "role": "assistant",
                    "model": "claude-3-5-sonnet-20241022",
                    "content": [{"type": "text", "text": "I see an image"}],
                    "stop_reason": "end_turn",
                    "usage": {"input_tokens": 10, "output_tokens": 5}
                }))
            })
            .mount(&server)
            .await;

        let client = AnthropicAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let base64_image = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let data_url = format!("data:image/png;base64,{}", base64_image);
        let req = ChatCompletionRequest {
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![
                    ChatContent::Text("What's in this image?".to_string()),
                    ChatContent::Parts(vec![
                        ChatContentPart::Text { text: "Describe this:".to_string() },
                        ChatContentPart::ImageUrl { image_url: ImageUrl { url: data_url, detail: None } },
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
        assert_eq!(content[2]["type"], "image");
        assert_eq!(content[2]["source"]["type"], "base64");
        assert_eq!(content[2]["source"]["media_type"], "image/png");
        assert_eq!(content[2]["source"]["data"], base64_image);
    }

    #[tokio::test]
    async fn unsupported_capabilities_return_error() {
        let client = AnthropicAdapter::new();
        let profile = dummy_profile();
        let model = dummy_model();

        let image_req = ImageGenerationRequest {
            model: "claude".to_string(),
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
            model: "claude".to_string(),
            input: "hello".to_string(),
            voice: "default".to_string(),
            response_format: None,
        };
        let err = client
            .audio_tts(&profile, &model, audio_req)
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::CapabilityNotSupported(_)));

        let embedding_req = EmbeddingRequest {
            model: "claude".to_string(),
            input: vec!["hello".to_string()],
        };
        let err = client
            .embedding(&profile, &model, embedding_req)
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::CapabilityNotSupported(_)));
    }

    #[test]
    fn test_anthropic_usage_parsed() {
        let json = r#"{
            "id": "msg_01",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-5-sonnet-20241022",
            "content": [{"type": "text", "text": "Hello"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_read_input_tokens": 20,
                "cache_creation_input_tokens": 10
            }
        }"#;
        
        let response: AnthropicMessageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.usage.input_tokens, 100);
        assert_eq!(response.usage.output_tokens, 50);
        assert_eq!(response.usage.cache_read_input_tokens, Some(20));
        assert_eq!(response.usage.cache_creation_input_tokens, Some(10));
    }

    #[tokio::test]
    async fn chat_sends_anthropic_version_without_api_key() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("anthropic-version", "2023-06-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg_01",
                "type": "message",
                "role": "assistant",
                "model": "claude-3-5-sonnet-20241022",
                "content": [{"type": "text", "text": "Hello from keyless profile"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 10, "output_tokens": 5}
            })))
            .mount(&server)
            .await;

        let client = AnthropicAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: None,
        };
        let req = ChatCompletionRequest {
            model: "claude-3-5-sonnet".to_string(),
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

        let (ProviderResponse::Chat(resp), _) =
            client.chat(&profile, &dummy_model(), req).await.unwrap()
        else {
            panic!("expected chat response");
        };

        assert_eq!(
            resp.choices[0].message.content_as_text(),
            Some("Hello from keyless profile".to_string())
        );
    }

    #[tokio::test]
    async fn chat_request_with_cache_control_sends_header() {
        use godwit_core::CacheControl;
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "msg_01",
                    "type": "message",
                    "role": "assistant",
                    "model": "claude-3-5-sonnet-20241022",
                    "content": [{"type": "text", "text": "Hi"}],
                    "stop_reason": "end_turn",
                    "usage": {
                        "input_tokens": 10,
                        "output_tokens": 5,
                        "cache_read_input_tokens": 8,
                        "cache_creation_input_tokens": 2
                    }
                }))
            })
            .mount(&server)
            .await;

        let client = AnthropicAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Hello with cache")]),
                name: None,
                cache_control: Some(CacheControl {
                    r#type: "ephemeral".to_string(),
                }),
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };

        let _ = client.chat(&profile, &dummy_model(), req).await.unwrap();

        let body = captured_body
            .lock()
            .unwrap()
            .take()
            .expect("request body captured");
        
        let messages = body["messages"].as_array().expect("messages present");
        assert_eq!(messages.len(), 1);
        let content = messages[0]["content"].as_array().expect("content is array");
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["cache_control"]["type"], "ephemeral");
    }

    #[tokio::test]
    async fn chat_response_reports_cache_tokens() {
        use godwit_core::CacheControl;
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg_01",
                "type": "message",
                "role": "assistant",
                "model": "claude-3-5-sonnet-20241022",
                "content": [{"type": "text", "text": "Hello"}],
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 5,
                    "cache_read_input_tokens": 8,
                    "cache_creation_input_tokens": 2
                }
            })))
            .mount(&server)
            .await;

        let client = AnthropicAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Hello")]),
                name: None,
                cache_control: Some(CacheControl {
                    r#type: "ephemeral".to_string(),
                }),
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };

        let (_, usage_report) = client.chat(&profile, &dummy_model(), req).await.unwrap();

        assert_eq!(usage_report.prompt_tokens, Some(10));
        assert_eq!(usage_report.completion_tokens, Some(5));
        assert_eq!(usage_report.cache_read_tokens, Some(8));
        assert_eq!(usage_report.cache_write_tokens, Some(2));
    }
}
