use crate::adapter::{
    Adapter, ProviderError, ProviderResponse, ResolvedProfile, SseEvent, UsageReport,
};
use crate::streaming::{build_sse_delta, build_sse_finish, build_sse_error, parse_sse_events};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::{self, BoxStream, StreamExt};
use godwit_core::{
    AudioSttRequest, AudioTtsRequest, Capability, ChatCompletionChoice, ChatCompletionRequest,
    ChatCompletionResponse, ChatContent, ChatMessage, EmbeddingData, EmbeddingRequest,
    EmbeddingResponse, ImageGenerationRequest, VideoGenerationRequest,
};
use godwit_db::models::Model;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};

pub struct GeminiProvider {
    client: Client,
}

pub type GeminiAdapter = GeminiProvider;

impl GeminiProvider {
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

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
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
struct GeminiTool {
    google_search_grounding: GeminiGoogleSearchGrounding,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGoogleSearchGrounding {}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
}

impl GeminiChatRequest {
    fn from_chat_request(request: ChatCompletionRequest) -> Self {
        let mut system_parts: Vec<String> = Vec::new();
        let mut contents: Vec<GeminiContent> = Vec::new();

        for msg in request.messages {
            if msg.role == "system" {
                if let Some(text) = msg.content.as_text() {
                    system_parts.push(text);
                }
            } else {
                let role = if msg.role == "assistant" {
                    "model".to_string()
                } else {
                    msg.role
                };
                contents.push(GeminiContent {
                    role,
                    parts: vec![GeminiPart { text: msg.content.as_text().unwrap_or_default() }],
                });
            }
        }

        let system_instruction = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        // Gemini supports native web search via the `googleSearchGrounding` tool. Any
        // native web search tools requested by the client are mapped into that form so
        // they pass through to the provider instead of being silently dropped.
        let has_native_web_search = request
            .tools
            .as_ref()
            .map(|tools| crate::web_search::has_native_web_search_tool(tools))
            .unwrap_or(false);

        Self {
            system_instruction,
            contents,
            generation_config: GeminiGenerationConfig {
                max_output_tokens: Some(request.max_tokens.unwrap_or(4096)),
                temperature: request.temperature,
            },
            tools: if has_native_web_search {
                Some(vec![GeminiTool {
                    google_search_grounding: GeminiGoogleSearchGrounding {},
                }])
            } else {
                None
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiEmbedContentRequest {
    model: String,
    content: GeminiEmbedContent,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiEmbedContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiBatchEmbedContentsRequest {
    requests: Vec<GeminiEmbedContentRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiEmbeddingValue {
    values: Vec<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiBatchEmbedContentsResponse {
    embeddings: Vec<GeminiEmbeddingValue>,
}

fn gemini_embedding_response_to_embedding_response(
    response: GeminiBatchEmbedContentsResponse,
    model_id: &str,
) -> Result<EmbeddingResponse, ProviderError> {
    let data: Vec<EmbeddingData> = response
        .embeddings
        .into_iter()
        .enumerate()
        .map(|(index, embedding)| EmbeddingData {
            object: "embedding".to_string(),
            embedding: embedding.values,
            index: index as i32,
        })
        .collect();

    Ok(EmbeddingResponse {
        object: "list".to_string(),
        data,
        model: model_id.to_string(),
        usage: godwit_core::Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            ..Default::default()
        },
    })
}

fn gemini_response_to_chat_completion(
    response: GeminiChatResponse,
    model_id: &str,
) -> Result<ChatCompletionResponse, ProviderError> {
    let candidate = response.candidates.into_iter().next().ok_or_else(|| {
        ProviderError::Provider("Gemini response contained no candidates".to_string())
    })?;

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
        ..Default::default()
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
                content: ChatContent::Text(content),
                name: None,
                ..Default::default()
            },
            finish_reason: candidate.finish_reason,
            ..Default::default()
        }],
        usage,
    })
}

/// Normalizes a single Gemini `streamGenerateContent` SSE chunk into proxy-canonical
/// `{ "type": "delta" }` / `{ "type": "finish" }` events.
///
/// Each Gemini streaming chunk carries `candidates[].content.parts[].text` fragments and,
/// on the terminating chunk, a `finishReason` plus `usageMetadata` token counts.
fn normalize_gemini_sse_event(raw: SseEvent) -> Vec<Result<SseEvent, ProviderError>> {
    let parsed: serde_json::Value = match serde_json::from_str(&raw.data) {
        Ok(v) => v,
        Err(e) => {
            return vec![Ok(SseEvent {
                data: build_sse_error(&format!("failed to parse gemini sse event: {e}")),
            })];
        }
    };

    let mut out: Vec<Result<SseEvent, ProviderError>> = Vec::new();

    if let Some(candidates) = parsed.get("candidates").and_then(|c| c.as_array()) {
        for candidate in candidates {
            if let Some(parts) = candidate
                .get("content")
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.as_array())
            {
                for part in parts {
                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                        if !text.is_empty() {
                            out.push(Ok(SseEvent {
                                data: build_sse_delta(text),
                            }));
                        }
                    }
                }
            }
            let finish_reason = candidate
                .get("finishReason")
                .and_then(|f| f.as_str())
                .filter(|r| !r.is_empty())
                .map(|s| s.to_string());
            if finish_reason.is_some() {
                let meta = parsed.get("usageMetadata").cloned().unwrap_or_default();
                let prompt = meta
                    .get("promptTokenCount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let completion = meta
                    .get("candidatesTokenCount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                out.push(Ok(SseEvent {
                    data: build_sse_finish(prompt, completion, finish_reason.as_deref()),
                }));
            }
        }
    }

    if out.is_empty() {
        debug!("ignoring gemini streaming chunk: no emit-able parts");
    }

    out
}

#[async_trait]
impl Adapter for GeminiProvider {
    fn supported_capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat, Capability::Embedding]
    }

    #[instrument(skip(self, profile, model, request))]
    async fn chat(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            profile.base_url,
            model.provider_model_id,
            profile.api_key.as_deref().unwrap_or_default()
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
            error!(
                "gemini chat request failed with status {}: {}",
                status, text
            );
            return Err(ProviderError::Http {
                status,
                message: text,
            });
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

    #[instrument(skip(self, profile, model, request))]
    async fn chat_stream(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            profile.base_url,
            model.provider_model_id,
            profile.api_key.as_deref().unwrap_or_default()
        );
        let gemini_request = GeminiChatRequest::from_chat_request(request);

        info!("sending gemini streaming chat request to {}", url);
        debug!("gemini streaming request body: {:?}", gemini_request);

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
            error!(
                "gemini streaming chat request failed with status {}: {}",
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
                .flat_map(normalize_gemini_sse_event)
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
            "image generation is not supported for Gemini".to_string(),
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
            "image edit is not supported by gemini".to_string(),
        ))
    }

    async fn video_generation(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: VideoGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "video generation is not supported for Gemini".to_string(),
        ))
    }

    async fn audio_tts(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "audio TTS is not supported for Gemini".to_string(),
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
            "audio STT is not supported for Gemini".to_string(),
        ))
    }

    async fn embedding(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: EmbeddingRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!(
            "{}/v1beta/models/{}:batchEmbedContents?key={}",
            profile.base_url,
            model.provider_model_id,
            profile.api_key.as_deref().unwrap_or_default()
        );

        let requests: Vec<GeminiEmbedContentRequest> = request
            .input
            .into_iter()
            .map(|text| GeminiEmbedContentRequest {
                model: model.provider_model_id.clone(),
                content: GeminiEmbedContent {
                    parts: vec![GeminiPart { text }],
                },
            })
            .collect();
        let gemini_request = GeminiBatchEmbedContentsRequest { requests };

        info!("sending gemini embedding request to {}", url);
        debug!("gemini embedding request body: {:?}", gemini_request);

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
            error!(
                "gemini embedding request failed with status {}: {}",
                status, text
            );
            return Err(ProviderError::Http {
                status,
                message: text,
            });
        }

        let body: GeminiBatchEmbedContentsResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;

        debug!("gemini embedding response body: {:?}", body);

        let embedding =
            gemini_embedding_response_to_embedding_response(body, &model.public_id)?;
        let total = embedding.data.iter().map(|d| d.embedding.len() as i64).sum();
        Ok((
            ProviderResponse::Embedding(embedding),
            UsageReport {
                embedding_tokens: Some(total),
                ..Default::default()
            },
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
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            api_key: Some("fake-key".to_string()),
        }
    }

    fn dummy_model() -> Model {
        Model {
            id: Uuid::nil(),
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
                    content: ChatContent::text("You are a helpful assistant."),
                    name: None,
                    ..Default::default()
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::text("Hello"),
                    name: None,
                    ..Default::default()
                },
                ChatMessage {
                    role: "assistant".to_string(),
                    content: ChatContent::text("Hi there"),
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

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::text("Hello"),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };

        let _ = client.chat(&profile, &dummy_model(), req).await.unwrap();

        let url = captured_url
            .lock()
            .unwrap()
            .take()
            .expect("request url captured");
        assert!(
            url.contains("/v1beta/models/gemini-1.5-flash:generateContent"),
            "url={}",
            url
        );
        assert!(url.contains("?key=fake-key"), "url={}", url);
    }

    /// Regression guard: the model segment of the Gemini URL must come from the catalog
    /// row's upstream `provider_model_id`, not from its friendly `public_id` (and not from
    /// the `<profile>/<suffix>` string a wildcard-resolved request carries in `public_id`).
    #[tokio::test]
    async fn chat_request_url_uses_provider_model_id_not_public_id() {
        let server = MockServer::start().await;
        let captured_url = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let captured_clone = captured_url.clone();

        Mock::given(method("POST"))
            .respond_with(move |req: &wiremock::Request| {
                *captured_clone.lock().unwrap() = Some(req.url.to_string());
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "candidates": [{
                        "content": { "role": "model", "parts": [{"text": "Hi"}] }
                    }]
                }))
            })
            .mount(&server)
            .await;

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        // Simulates a wildcard-resolved request: public_id is the whole model_ref.
        let model = Model {
            public_id: "google/gemini-2.0-flash-001".to_string(),
            provider_model_id: "gemini-2.0-flash-001".to_string(),
            ..dummy_model()
        };
        let req = ChatCompletionRequest {
            model: "google/gemini-2.0-flash-001".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::text("Hello"),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };

        let _ = client.chat(&profile, &model, req).await.unwrap();

        let url = captured_url
            .lock()
            .unwrap()
            .take()
            .expect("request url captured");
        assert!(
            url.contains("/v1beta/models/gemini-2.0-flash-001:generateContent"),
            "url={url}"
        );
        assert!(
            !url.contains("google/gemini-2.0-flash-001"),
            "the profile-prefixed public_id must not leak into the upstream URL, url={url}"
        );
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

        let client = GeminiProvider::new();
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
        assert_eq!(body["systemInstruction"], "You are a helpful assistant.");
        assert_eq!(body["contents"].as_array().unwrap().len(), 2);
        assert_eq!(body["contents"][0]["role"], "user");
        assert_eq!(body["contents"][0]["parts"][0]["text"], "Hello");
        assert_eq!(body["contents"][1]["role"], "model");
        assert_eq!(body["contents"][1]["parts"][0]["text"], "Hi there");
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 4096);
        assert!(body["generationConfig"]["temperature"].is_null());
    }

    #[tokio::test]
    async fn chat_passes_through_native_web_search_tools() {
        use godwit_core::{FunctionDefinition, Tool};
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
                        "content": {"role": "model", "parts": [{"text": "Hi"}]}
                    }]
                }))
            })
            .mount(&server)
            .await;

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let mut req = chat_request_with_system();
        req.tools = Some(vec![Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "web_search".to_string(),
                description: None,
                parameters: None,
            },
        }]);
        let _ = client.chat(&profile, &dummy_model(), req).await.unwrap();

        let body = captured_body.lock().unwrap().take().expect("request body captured");
        let tools = body["tools"].as_array().expect("tools present");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["googleSearchGrounding"], serde_json::json!({}));
    }

    #[tokio::test]
    async fn chat_drops_tools_when_no_native_web_search() {
        use godwit_core::{FunctionDefinition, Tool};
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
                        "content": {"role": "model", "parts": [{"text": "Hi"}]}
                    }]
                }))
            })
            .mount(&server)
            .await;

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let mut req = chat_request_with_system();
        req.tools = Some(vec![Tool {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: None,
                parameters: None,
            },
        }]);
        let _ = client.chat(&profile, &dummy_model(), req).await.unwrap();

        let body = captured_body.lock().unwrap().take().expect("request body captured");
        assert!(
            body["tools"].is_null(),
            "ordinary function tools are not supported yet and must not be forwarded"
        );
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

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::text("Hi"),
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
            resp.choices[0].message.content.as_text(),
            Some("Hello, world!".to_string())
        );
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

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::text("Hi"),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };

        let err = client
            .chat(&profile, &dummy_model(), req)
            .await
            .unwrap_err();
        match err {
            ProviderError::Http { status, .. } => assert_eq!(status, 400),
            _ => panic!("expected http error, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn unsupported_capabilities_return_error() {
        let client = GeminiProvider::new();
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
        let err = client
            .audio_tts(&profile, &model, audio_req)
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::CapabilityNotSupported(_)));
    }

    #[tokio::test]
    async fn chat_stream_emits_delta_and_finish_events() {
        let server = MockServer::start().await;
        let sse_body = concat!(
            "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]},\"finishReason\":null,\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":0,\"candidatesTokenCount\":0,\"totalTokenCount\":0}}\n\n",
            "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\" world\"}]},\"finishReason\":null,\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":0,\"candidatesTokenCount\":0,\"totalTokenCount\":0}}\n\n",
            "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":5,\"totalTokenCount\":15}}\n\n"
        );

        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-1.5-flash:streamGenerateContent"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse_body),
            )
            .mount(&server)
            .await;

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::text("Hi"),
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
        assert_eq!(finish["usage"]["prompt_tokens"], 10);
        assert_eq!(finish["usage"]["completion_tokens"], 5);
        assert_eq!(finish["usage"]["total_tokens"], 15);
    }

    #[tokio::test]
    async fn embedding_returns_normalized_response() {
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-1.5-flash:batchEmbedContents"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "embeddings": [
                        {"values": [0.1, 0.2, 0.3]},
                        {"values": [0.4, 0.5]}
                    ]
                }))
            })
            .mount(&server)
            .await;

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = EmbeddingRequest {
            model: "gemini".to_string(),
            input: vec!["hello".to_string(), "world".to_string()],
        };
        let (ProviderResponse::Embedding(resp), usage_report) =
            client.embedding(&profile, &dummy_model(), req).await.unwrap()
        else {
            panic!("expected embedding response");
        };
        assert_eq!(resp.data.len(), 2);
        assert_eq!(resp.data[0].embedding, vec![0.1, 0.2, 0.3]);
        assert_eq!(resp.model, "gemini-1.5-flash");
        assert_eq!(usage_report.embedding_tokens, Some(5));

        let body = captured_body
            .lock()
            .unwrap()
            .take()
            .expect("request body captured");
        assert_eq!(body["requests"].as_array().unwrap().len(), 2);
        assert_eq!(body["requests"][0]["content"]["parts"][0]["text"], "hello");
    }
}
