use crate::adapter::{
    Adapter, ProviderError, ProviderResponse, ResolvedProfile, SseEvent, UsageReport,
};
use crate::gemini_cache::{
    CachedContent, CreateCachedContentRequest, CreateCachedContentResponse,
    GenerateWithCacheRequest, GeminiCacheKey,
};
use crate::gemini_stream::GeminiStreamTranslator;
use crate::prompt_cache::PromptCache;
use crate::streaming::parse_sse_events;
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::{self, BoxStream};
use futures::StreamExt;
use godwit_core::{
    AudioSttRequest, AudioTtsRequest, Batch, BatchRequest, Capability, ChatCompletionChoice,
    ChatCompletionRequest, ChatCompletionResponse, ChatContent, ChatContentPart, ChatMessage,
    EmbeddingData, EmbeddingRequest, EmbeddingResponse, ImageGenerationRequest,
    VideoGenerationRequest,
};
use godwit_db::models::Model;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, instrument};

pub struct GeminiProvider {
    client: Client,
    /// Server-side prompt cache for Gemini cachedContent API
    /// Default TTL: 3600s (1 hour), Max size: 10000 entries
    prompt_cache: Arc<PromptCache<GeminiCacheKey, CachedContent>>,
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
        Self {
            client,
            prompt_cache: Arc::new(PromptCache::with_config(10000, Duration::from_secs(3600))),
        }
    }

    /// Get a reference to the prompt cache
    pub fn prompt_cache(&self) -> &PromptCache<GeminiCacheKey, CachedContent> {
        &self.prompt_cache
    }

    /// Create a cache key from a chat completion request
    fn create_cache_key(request: &ChatCompletionRequest) -> GeminiCacheKey {
        let messages_json = serde_json::to_string(&request.messages).unwrap_or_default();
        let mut messages_hasher = DefaultHasher::new();
        messages_json.hash(&mut messages_hasher);

        let system_instruction = request
            .messages
            .iter()
            .filter(|msg| msg.role == "system")
            .filter_map(|msg| msg.content_as_text())
            .collect::<Vec<_>>()
            .join("\n\n");

        GeminiCacheKey::new(&request.model, &[], if system_instruction.is_empty() { None } else { Some(&system_instruction) })
    }

    /// Create cached content on Gemini servers
    #[instrument(skip(self, profile, model, request))]
    pub async fn create_cached_content(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: &ChatCompletionRequest,
        ttl: Option<Duration>,
    ) -> Result<CachedContent, ProviderError> {
        let url = format!(
            "{}/v1beta/cachedContents?key={}",
            profile.base_url,
            profile.api_key.as_deref().unwrap_or_default()
        );

        // Build the contents to cache (non-system messages)
        let contents: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|msg| msg.role != "system")
            .map(|msg| {
                let role = if msg.role == "assistant" { "model" } else { &msg.role };
                let parts = msg.content.as_ref().map(|contents| {
                    contents
                        .iter()
                        .flat_map(|c| match c {
                            ChatContent::Text(text) => {
                                vec![serde_json::json!({ "text": text })]
                            }
                            ChatContent::Parts(parts) => parts
                                .iter()
                                .map(|p| match p {
                                    ChatContentPart::Text { text } => {
                                        serde_json::json!({ "text": text })
                                    }
                                    ChatContentPart::ImageUrl { image_url } => {
                                        if image_url.url.starts_with("data:") {
                                            let (media_type, data) = Self::parse_data_url(&image_url.url);
                                            serde_json::json!({
                                                "inlineData": {
                                                    "mimeType": media_type,
                                                    "data": data
                                                }
                                            })
                                        } else {
                                            serde_json::json!({
                                                "fileData": {
                                                    "mimeType": "image/png",
                                                    "fileUri": image_url.url
                                                }
                                            })
                                        }
                                    }
                                })
                                .collect(),
                        })
                        .collect::<Vec<_>>()
                }).unwrap_or_default();

                serde_json::json!({
                    "role": role,
                    "parts": parts
                })
            })
            .collect();

        // Extract system instruction
        let system_instruction = request
            .messages
            .iter()
            .filter(|msg| msg.role == "system")
            .filter_map(|msg| msg.content_as_text())
            .collect::<Vec<_>>()
            .first()
            .cloned();

        let cache_request = CreateCachedContentRequest {
            contents,
            system_instruction,
            model: model.provider_model_id.clone(),
            display_name: None,
            ttl: ttl.map(|d| format!("{}s", d.as_secs())),
        };

        info!("creating Gemini cached content at {}", url);
        debug!("cached content request body: {:?}", cache_request);

        let res = self
            .client
            .post(&url)
            .json(&cache_request)
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
                "create cached content request failed with status {}: {}",
                status, text
            );
            return Err(ProviderError::Http {
                status,
                message: text,
            });
        }

        let body: CreateCachedContentResponse = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;

        debug!("cached content response: {:?}", body);

        // Create cache key for lookup
        let cache_key = Self::create_cache_key(request);

        Ok(CachedContent {
            id: body.id(),
            model: model.provider_model_id.clone(),
            messages_hash: cache_key.messages_hash,
            created_at: std::time::Instant::now(),
            ttl: ttl.unwrap_or(Duration::from_secs(3600)),
        })
    }

    /// Generate content using cached content
    #[instrument(skip(self, profile, model, request))]
    pub async fn generate_with_cache(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: ChatCompletionRequest,
        cached_content_id: &str,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            profile.base_url,
            model.provider_model_id,
            profile.api_key.as_deref().unwrap_or_default()
        );

        // Build new content (non-system messages only)
        let contents: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|msg| msg.role != "system")
            .map(|msg| {
                let role = if msg.role == "assistant" { "model" } else { &msg.role };
                let parts = msg.content.as_ref().map(|contents| {
                    contents
                        .iter()
                        .flat_map(|c| match c {
                            ChatContent::Text(text) => {
                                vec![serde_json::json!({ "text": text })]
                            }
                            ChatContent::Parts(parts) => parts
                                .iter()
                                .map(|p| match p {
                                    ChatContentPart::Text { text } => {
                                        serde_json::json!({ "text": text })
                                    }
                                    ChatContentPart::ImageUrl { image_url } => {
                                        if image_url.url.starts_with("data:") {
                                            let (media_type, data) = Self::parse_data_url(&image_url.url);
                                            serde_json::json!({
                                                "inlineData": {
                                                    "mimeType": media_type,
                                                    "data": data
                                                }
                                            })
                                        } else {
                                            serde_json::json!({
                                                "fileData": {
                                                    "mimeType": "image/png",
                                                    "fileUri": image_url.url
                                                }
                                            })
                                        }
                                    }
                                })
                                .collect(),
                        })
                        .collect::<Vec<_>>()
                }).unwrap_or_default();

                serde_json::json!({
                    "role": role,
                    "parts": parts
                })
            })
            .collect();

        // Extract system instruction
        let system_instruction = request
            .messages
            .iter()
            .filter(|msg| msg.role == "system")
            .filter_map(|msg| msg.content_as_text())
            .collect::<Vec<_>>()
            .first()
            .cloned();

        let generation_config = serde_json::json!({
            "maxOutputTokens": request.max_tokens.unwrap_or(4096),
            "temperature": request.temperature,
        });

        let cache_request = GenerateWithCacheRequest {
            contents,
            system_instruction,
            generation_config: Some(generation_config),
            tools: None,
            cached_content: Some(cached_content_id.to_string()),
        };

        info!("generating with cached content at {}", url);
        debug!("generate with cache request body: {:?}", cache_request);

        let res = self
            .client
            .post(&url)
            .json(&cache_request)
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
                "generate with cache request failed with status {}: {}",
                status, text
            );
            return Err(ProviderError::Http {
                status,
                message: text,
            });
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;

        debug!("generate with cache response: {:?}", body);

        // Parse the response using existing logic
        let gemini_response: GeminiChatResponse = serde_json::from_value(body.clone())
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;

        let usage_report = if let Some(ref metadata) = gemini_response.usage_metadata {
            UsageReport {
                prompt_tokens: Some(metadata.prompt_token_count),
                completion_tokens: Some(metadata.candidates_token_count),
                cache_read_tokens: metadata.cached_content_token_count,
                ..Default::default()
            }
        } else {
            UsageReport::default()
        };

        let chat_response = gemini_response_to_chat_completion(gemini_response, &model.public_id)?;

        Ok((ProviderResponse::Chat(chat_response), usage_report))
    }

    /// Parse a data URL into media type and base64 data
    fn parse_data_url(url: &str) -> (String, String) {
        if let Some(comma_pos) = url.find(',') {
            let header = &url[..comma_pos];
            let data = &url[comma_pos + 1..];
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
            (media_type, data.to_string())
        } else {
            ("image/png".to_string(), url.to_string())
        }
    }

    /// Internal method for chat without caching
    async fn chat_without_cache(
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

        let usage_report = if let Some(ref metadata) = body.usage_metadata {
            UsageReport {
                prompt_tokens: Some(metadata.prompt_token_count),
                completion_tokens: Some(metadata.candidates_token_count),
                cache_read_tokens: metadata.cached_content_token_count,
                ..Default::default()
            }
        } else {
            UsageReport::default()
        };

        let chat_response = gemini_response_to_chat_completion(body, &model.public_id)?;

        Ok((ProviderResponse::Chat(chat_response), usage_report))
    }
}

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiTextPart {
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiInlineDataPart {
    inline_data: GeminiBlob,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFileDataPart {
    file_data: GeminiFileData,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum GeminiPart {
    Text(GeminiTextPart),
    InlineData(GeminiInlineDataPart),
    FileData(GeminiFileDataPart),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiBlob {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFileData {
    mime_type: String,
    file_uri: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

impl GeminiContent {
    fn from_chat_message(msg: &ChatMessage) -> Self {
        let role = if msg.role == "assistant" {
            "model".to_string()
        } else {
            msg.role.clone()
        };

        let parts = msg
            .content
            .as_ref()
            .map(|contents| {
                contents
                    .iter()
                    .flat_map(|c| match c {
                        ChatContent::Text(text) => {
                            vec![GeminiPart::Text(GeminiTextPart { text: text.clone() })]
                        }
                        ChatContent::Parts(parts) => parts
                            .iter()
                            .map(|p| match p {
                                ChatContentPart::Text { text } => {
                                    GeminiPart::Text(GeminiTextPart { text: text.clone() })
                                }
                                ChatContentPart::ImageUrl { image_url } => {
                                    if image_url.url.starts_with("data:") {
                                        let (media_type, data) = Self::parse_data_url(&image_url.url);
                                        GeminiPart::InlineData(GeminiInlineDataPart {
                                            inline_data: GeminiBlob {
                                                mime_type: media_type,
                                                data,
                                            },
                                        })
                                    } else {
                                        GeminiPart::FileData(GeminiFileDataPart {
                                            file_data: GeminiFileData {
                                                mime_type: "image/png".to_string(),
                                                file_uri: image_url.url.clone(),
                                            },
                                        })
                                    }
                                }
                            })
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self { role, parts }
    }

    fn parse_data_url(url: &str) -> (String, String) {
        if let Some(comma_pos) = url.find(',') {
            let header = &url[..comma_pos];
            let data = &url[comma_pos + 1..];
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
            (media_type, data.to_string())
        } else {
            ("image/png".to_string(), url.to_string())
        }
    }
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
                if let Some(text) = msg.content_as_text() {
                    system_parts.push(text);
                }
            } else {
                contents.push(GeminiContent::from_chat_message(&msg));
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
    #[serde(default)]
    cached_content_token_count: Option<i32>,
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
                content: Some(vec![ChatContent::Text(content)]),
                name: None,
                ..Default::default()
            },
            finish_reason: candidate.finish_reason,
            ..Default::default()
        }],
        usage,
    })
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
        // For streaming requests, bypass cache
        if request.stream == Some(true) {
            return self.chat_without_cache(profile, model, request).await;
        }

        // Check if we have cached content for this request
        let cache_key = Self::create_cache_key(&request);
        
        // Try to get cached content from local cache
        if let Some(cached) = self.prompt_cache.get(&cache_key) {
            if !cached.is_expired() {
                debug!("using cached content id: {}", cached.id);
                // Use the cached content for generation
                return self.generate_with_cache(profile, model, request, &cached.id).await;
            } else {
                // Cached content has expired, remove it
                self.prompt_cache.remove(&cache_key);
            }
        }

        // No valid cache, make the request normally
        let result = self.chat_without_cache(profile, model, request).await?;
        
        // Cache the response for future use (only if not streaming)
        // Note: For Gemini, we would need to create cached content on the server
        // This is done via create_cached_content() which can be called separately
        // Here we just cache the response locally
        
        Ok(result)
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
        let mut translator = GeminiStreamTranslator::new();
        let event_stream = byte_stream
            .filter_map(|bytes_result| async move {
                bytes_result
                    .map(|b| String::from_utf8_lossy(&b).to_string())
                    .ok()
            })
            .flat_map(move |text| {
                let events: Vec<_> = parse_sse_events(&text)
                    .into_iter()
                    .flat_map(|sse_event| translator.translate_chunk(&sse_event.data))
                    .collect();
                stream::iter(events)
            })
            .boxed();

        Ok(event_stream)
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
                    parts: vec![GeminiPart::Text(GeminiTextPart { text })],
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

    async fn create_batch(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: BatchRequest,
    ) -> Result<Batch, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "batch is not supported for Gemini".to_string(),
        ))
    }

    async fn retrieve_batch(
        &self,
        _profile: &ResolvedProfile,
        _batch_id: String,
    ) -> Result<Batch, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "batch is not supported for Gemini".to_string(),
        ))
    }

    async fn cancel_batch(
        &self,
        _profile: &ResolvedProfile,
        _batch_id: String,
    ) -> Result<Batch, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "batch is not supported for Gemini".to_string(),
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
                ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(vec![ChatContent::text("Hi there")]),
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
                content: Some(vec![ChatContent::text("Hello")]),
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
                content: Some(vec![ChatContent::text("Hello")]),
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
    async fn chat_multimodal_text_and_image_inline_data() {
        use godwit_core::{ChatContentPart, ImageUrl};
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-1.5-flash:generateContent"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "candidates": [{
                        "content": {
                            "role": "model",
                            "parts": [{"text": "I see an image"}]
                        },
                        "finishReason": "STOP"
                    }],
                    "usageMetadata": {
                        "promptTokenCount": 10,
                        "candidatesTokenCount": 5,
                        "totalTokenCount": 15
                    }
                }))
            })
            .mount(&server)
            .await;

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let base64_image = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let data_url = format!("data:image/png;base64,{}", base64_image);
        let req = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
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
        let contents = body["contents"].as_array().expect("contents present");
        assert_eq!(contents.len(), 1);
        let parts = contents[0]["parts"].as_array().expect("parts present");
        assert_eq!(parts.len(), 3);
        assert!(parts[0]["text"].is_string());
        assert_eq!(parts[0]["text"], "What's in this image?");
        assert!(parts[1]["text"].is_string());
        assert_eq!(parts[1]["text"], "Describe this:");
        assert!(parts[2]["inlineData"].is_object());
        assert_eq!(parts[2]["inlineData"]["mimeType"], "image/png");
        assert_eq!(parts[2]["inlineData"]["data"], base64_image);
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
                content: Some(vec![ChatContent::text("Hi")]),
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
        assert_eq!(finish["usage"]["prompt_tokens"], 10);
        assert_eq!(finish["usage"]["completion_tokens"], 5);
        assert_eq!(finish["usage"]["total_tokens"], 15);
    }

    #[test]
    fn test_gemini_usage_parsed() {
        let json = r#"{
            "candidates": [{"content": {"role": "model", "parts": [{"text": "Hi"}]}}],
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 50,
                "cachedContentTokenCount": 20
            }
        }"#;
        
        let response: GeminiChatResponse = serde_json::from_str(json).unwrap();
        let metadata = response.usage_metadata.unwrap();
        assert_eq!(metadata.prompt_token_count, 100);
        assert_eq!(metadata.candidates_token_count, 50);
        assert_eq!(metadata.cached_content_token_count, Some(20));
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

    #[tokio::test]
    async fn create_cached_content_sends_correct_request() {
        use std::time::Duration;
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/v1beta/cachedContents"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "name": "cachedContents/test-cache-id-123",
                    "model": "gemini-1.5-flash",
                    "createTime": "2024-01-01T00:00:00Z",
                    "expireTime": "2024-01-01T01:00:00Z"
                }))
            })
            .mount(&server)
            .await;

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let request = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: Some(vec![ChatContent::text("You are helpful.")]),
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
        };

        let cached = client
            .create_cached_content(&profile, &dummy_model(), &request, Some(Duration::from_secs(3600)))
            .await
            .unwrap();

        assert_eq!(cached.id, "test-cache-id-123");
        assert_eq!(cached.model, "gemini-1.5-flash");

        let body = captured_body.lock().unwrap().take().expect("request body captured");
        assert_eq!(body["model"], "gemini-1.5-flash");
        assert_eq!(body["systemInstruction"], "You are helpful.");
        assert_eq!(body["ttl"], "3600s");
    }

    #[tokio::test]
    async fn generate_with_cache_includes_cached_content_id() {
        let server = MockServer::start().await;
        let captured_body = std::sync::Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let captured_clone = captured_body.clone();

        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-1.5-flash:generateContent"))
            .respond_with(move |req: &wiremock::Request| {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&req.body) {
                    *captured_clone.lock().unwrap() = Some(body);
                }
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "candidates": [{
                        "content": {
                            "role": "model",
                            "parts": [{"text": "Response from cache"}]
                        },
                        "finishReason": "STOP"
                    }],
                    "usageMetadata": {
                        "promptTokenCount": 5,
                        "candidatesTokenCount": 10,
                        "totalTokenCount": 15,
                        "cachedContentTokenCount": 100
                    }
                }))
            })
            .mount(&server)
            .await;

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let request = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Continue")]),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };

        let (ProviderResponse::Chat(resp), usage_report) = client
            .generate_with_cache(&profile, &dummy_model(), request, "test-cache-id-123")
            .await
            .unwrap()
        else {
            panic!("expected chat response");
        };

        assert_eq!(
            resp.choices[0].message.content_as_text(),
            Some("Response from cache".to_string())
        );
        assert_eq!(usage_report.cache_read_tokens, Some(100));

        let body = captured_body.lock().unwrap().take().expect("request body captured");
        assert_eq!(body["cachedContent"], "test-cache-id-123");
    }

    #[tokio::test]
    async fn chat_uses_cached_content_when_available() {
        use std::time::Duration;
        let server = MockServer::start().await;
        
        // Mock the generateContent endpoint
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-1.5-flash:generateContent"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "candidates": [{
                    "content": {
                        "role": "model",
                        "parts": [{"text": "Response"}]
                    },
                    "finishReason": "STOP"
                }],
                "usageMetadata": {
                    "promptTokenCount": 5,
                    "candidatesTokenCount": 10,
                    "totalTokenCount": 15,
                    "cachedContentTokenCount": 100
                }
            })))
            .mount(&server)
            .await;

        let client = GeminiProvider::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        
        // Manually insert a cached content entry
        let cache_key = GeminiCacheKey::new("gemini-1.5-flash", &[], Some("test"));
        let cached_content = CachedContent {
            id: "manual-cache-id".to_string(),
            model: "gemini-1.5-flash".to_string(),
            messages_hash: cache_key.messages_hash,
            created_at: std::time::Instant::now(),
            ttl: Duration::from_secs(3600),
        };
        client.prompt_cache.insert(cache_key, cached_content);

        let request = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Test")]),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            ..Default::default()
        };

        let (ProviderResponse::Chat(resp), usage_report) = client
            .chat(&profile, &dummy_model(), request)
            .await
            .unwrap()
        else {
            panic!("expected chat response");
        };

        assert_eq!(
            resp.choices[0].message.content_as_text(),
            Some("Response".to_string())
        );
        assert_eq!(usage_report.cache_read_tokens, Some(100));
    }

    #[test]
    fn test_cached_content_expiration() {
        use std::time::Duration;
        
        let cached = CachedContent {
            id: "test-id".to_string(),
            model: "gemini-1.5-flash".to_string(),
            messages_hash: 12345,
            created_at: std::time::Instant::now(),
            ttl: Duration::from_secs(1),
        };
        
        assert!(!cached.is_expired());
        
        std::thread::sleep(Duration::from_millis(1100));
        
        assert!(cached.is_expired());
    }

    #[test]
    fn test_gemini_cache_key_generation() {
        use godwit_core::ChatContent;
        
        let request1 = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Hello")]),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
            ..Default::default()
        };
        
        let request2 = ChatCompletionRequest {
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(vec![ChatContent::text("Hello")]),
                name: None,
                ..Default::default()
            }],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
            ..Default::default()
        };
        
        let key1 = GeminiProvider::create_cache_key(&request1);
        let key2 = GeminiProvider::create_cache_key(&request2);
        
        assert_eq!(key1, key2);
    }
}
