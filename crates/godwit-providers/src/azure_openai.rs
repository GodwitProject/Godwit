use crate::adapter::{
    Adapter, ProviderError, ProviderResponse, ResolvedProfile, SseEvent, UsageReport,
};
use crate::streaming::{normalize_openai_sse_event, parse_sse_events};
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use godwit_core::{
    AudioSttRequest, AudioSttResponse, AudioTtsRequest, Batch, BatchRequest, Capability,
    ChatCompletionRequest, ChatCompletionResponse, ChatContent, ChatContentPart,
    EmbeddingRequest, EmbeddingResponse, ImageGenerationRequest, ImageGenerationResponse,
    ResponseFormat,
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
struct OpenAiJsonSchema {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strict: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenAiResponseFormat {
    Text,
    JsonObject,
    JsonSchema { json_schema: OpenAiJsonSchema },
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
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<OpenAiResponseFormat>,
}

pub struct AzureOpenAiProvider {
    client: Client,
}

pub type AzureOpenAiAdapter = AzureOpenAiProvider;

impl AzureOpenAiProvider {
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

impl Default for AzureOpenAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for AzureOpenAiProvider {
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
        request.model = model.provider_model_id.clone();
        let openai_messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(OpenAiMessage::from_chat_message)
            .collect();
        let response_format = request.response_format.as_ref().map(|rf| match rf {
            ResponseFormat::Text => OpenAiResponseFormat::Text,
            ResponseFormat::JsonObject => OpenAiResponseFormat::JsonObject,
            ResponseFormat::JsonSchema { json_schema } => OpenAiResponseFormat::JsonSchema {
                json_schema: OpenAiJsonSchema {
                    name: json_schema.name.clone(),
                    schema: json_schema.schema.clone(),
                    strict: Some(json_schema.strict.unwrap_or(true)),
                },
            },
        });
        let openai_request = OpenAiChatRequest {
            model: request.model.clone(),
            messages: openai_messages,
            stream: request.stream,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: request.tools.clone(),
            tool_choice: request.tool_choice.clone(),
            response_format,
        };
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2024-02-15-preview",
            profile.base_url, request.model
        );
        let mut req = self.client.post(&url).json(&openai_request);
        if let Some(key) = &profile.api_key {
            req = req.header("api-key", key.clone());
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
        request.model = model.provider_model_id.clone();
        let openai_messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(OpenAiMessage::from_chat_message)
            .collect();
        let response_format = request.response_format.as_ref().map(|rf| match rf {
            ResponseFormat::Text => OpenAiResponseFormat::Text,
            ResponseFormat::JsonObject => OpenAiResponseFormat::JsonObject,
            ResponseFormat::JsonSchema { json_schema } => OpenAiResponseFormat::JsonSchema {
                json_schema: OpenAiJsonSchema {
                    name: json_schema.name.clone(),
                    schema: json_schema.schema.clone(),
                    strict: Some(json_schema.strict.unwrap_or(true)),
                },
            },
        });
        let openai_request = OpenAiChatRequest {
            model: request.model.clone(),
            messages: openai_messages,
            stream: request.stream,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            tools: request.tools.clone(),
            tool_choice: request.tool_choice.clone(),
            response_format,
        };
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2024-02-15-preview&stream=true",
            profile.base_url, request.model
        );
        let mut req = self.client.post(&url).json(&openai_request);
        if let Some(key) = &profile.api_key {
            req = req.header("api-key", key.clone());
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
        request.model = model.provider_model_id.clone();
        let url = format!(
            "{}/openai/deployments/{}/images/generations?api-version=2024-02-15-preview",
            profile.base_url, request.model
        );
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("api-key", key.clone());
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
        request.model = model.provider_model_id.clone();
        let url = format!(
            "{}/openai/deployments/{}/images/edits?api-version=2024-02-15-preview",
            profile.base_url, request.model
        );
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
            req = req.header("api-key", key.clone());
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
            "video generation is not supported for Azure OpenAI".to_string(),
        ))
    }

    async fn audio_tts(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        request.model = model.provider_model_id.clone();
        let url = format!(
            "{}/openai/deployments/{}/audio/speech?api-version=2024-02-15-preview",
            profile.base_url, request.model
        );
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("api-key", key.clone());
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
        request.model = model.provider_model_id.clone();
        let url = format!(
            "{}/openai/deployments/{}/audio/transcriptions?api-version=2024-02-15-preview",
            profile.base_url, request.model
        );
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
            req = req.header("api-key", key.clone());
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
        request.model = model.provider_model_id.clone();
        let url = format!(
            "{}/openai/deployments/{}/embeddings?api-version=2024-02-15-preview",
            profile.base_url, request.model
        );
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("api-key", key.clone());
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

    async fn create_batch(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        request: BatchRequest,
    ) -> Result<Batch, ProviderError> {
        let url = format!(
            "{}/openai/batches?api-version=2024-02-15-preview",
            profile.base_url
        );
        let mut req = self.client.post(&url).json(&request);
        if let Some(key) = &profile.api_key {
            req = req.header("api-key", key.clone());
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
        let body: Batch = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok(body)
    }

    async fn retrieve_batch(
        &self,
        profile: &ResolvedProfile,
        batch_id: String,
    ) -> Result<Batch, ProviderError> {
        let url = format!(
            "{}/openai/batches/{}?api-version=2024-02-15-preview",
            profile.base_url, batch_id
        );
        let mut req = self.client.get(&url);
        if let Some(key) = &profile.api_key {
            req = req.header("api-key", key.clone());
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
        let body: Batch = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok(body)
    }

    async fn cancel_batch(
        &self,
        profile: &ResolvedProfile,
        batch_id: String,
    ) -> Result<Batch, ProviderError> {
        let url = format!(
            "{}/openai/batches/{}/cancel?api-version=2024-02-15-preview",
            profile.base_url, batch_id
        );
        let mut req = self.client.post(&url);
        if let Some(key) = &profile.api_key {
            req = req.header("api-key", key.clone());
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
        let body: Batch = res
            .json()
            .await
            .map_err(|e| ProviderError::Serialization(e.to_string()))?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use godwit_core::BatchRequest;
    use uuid::Uuid;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    fn dummy_model() -> Model {
        Model {
            id: Uuid::nil(),
            public_id: "gpt-4o".to_string(),
            provider: "azure_openai".to_string(),
            provider_profile_id: Uuid::nil(),
            provider_model_id: "gpt-4o".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn azure_create_batch() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "batch_123",
            "object": "batch",
            "endpoint": "/v1/chat/completions",
            "errors": null,
            "input_file_id": "file-abc",
            "completion_window": "24h",
            "status": "validating",
            "output_file_id": null,
            "error_file_id": null,
            "created_at": 1234567890,
            "in_progress_at": null,
            "expires_at": null,
            "finalizing_at": null,
            "completed_at": null,
            "failed_at": null,
            "expired_at": null,
            "cancelling_at": null,
            "cancelled_at": null,
            "request_counts": {
                "total": 0,
                "completed": 0,
                "failed": 0
            },
            "metadata": null
        });
        Mock::given(method("POST"))
            .and(path("/openai/batches"))
            .and(query_param("api-version", "2024-02-15-preview"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = AzureOpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = BatchRequest {
            input_file_id: "file-abc".to_string(),
            endpoint: "/v1/chat/completions".to_string(),
            completion_window: "24h".to_string(),
            metadata: None,
        };
        let batch = client
            .create_batch(&profile, &dummy_model(), req)
            .await
            .unwrap();
        assert_eq!(batch.id, "batch_123");
        assert_eq!(batch.status, "validating");
    }

    #[tokio::test]
    async fn azure_retrieve_batch() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "batch_123",
            "object": "batch",
            "endpoint": "/v1/chat/completions",
            "errors": null,
            "input_file_id": "file-abc",
            "completion_window": "24h",
            "status": "completed",
            "output_file_id": "file-xyz",
            "error_file_id": null,
            "created_at": 1234567890,
            "in_progress_at": 1234567900,
            "expires_at": null,
            "finalizing_at": null,
            "completed_at": 1234568000,
            "failed_at": null,
            "expired_at": null,
            "cancelling_at": null,
            "cancelled_at": null,
            "request_counts": {
                "total": 10,
                "completed": 10,
                "failed": 0
            },
            "metadata": null
        });
        Mock::given(method("GET"))
            .and(path("/openai/batches/batch_123"))
            .and(query_param("api-version", "2024-02-15-preview"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = AzureOpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let batch = client
            .retrieve_batch(&profile, "batch_123".to_string())
            .await
            .unwrap();
        assert_eq!(batch.id, "batch_123");
        assert_eq!(batch.status, "completed");
        assert_eq!(batch.output_file_id, Some("file-xyz".to_string()));
    }

    #[tokio::test]
    async fn azure_cancel_batch() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "batch_123",
            "object": "batch",
            "endpoint": "/v1/chat/completions",
            "errors": null,
            "input_file_id": "file-abc",
            "completion_window": "24h",
            "status": "cancelling",
            "output_file_id": null,
            "error_file_id": null,
            "created_at": 1234567890,
            "in_progress_at": 1234567900,
            "expires_at": null,
            "finalizing_at": null,
            "completed_at": null,
            "failed_at": null,
            "expired_at": null,
            "cancelling_at": 1234568000,
            "cancelled_at": null,
            "request_counts": {
                "total": 10,
                "completed": 5,
                "failed": 0
            },
            "metadata": null
        });
        Mock::given(method("POST"))
            .and(path("/openai/batches/batch_123/cancel"))
            .and(query_param("api-version", "2024-02-15-preview"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = AzureOpenAiAdapter::new();
        let profile = ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let batch = client
            .cancel_batch(&profile, "batch_123".to_string())
            .await
            .unwrap();
        assert_eq!(batch.id, "batch_123");
        assert_eq!(batch.status, "cancelling");
    }
}
