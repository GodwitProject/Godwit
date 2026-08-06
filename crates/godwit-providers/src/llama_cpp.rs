use crate::adapter::{
    Adapter, ProviderError, ProviderResponse, ResolvedProfile, SseEvent, UsageReport,
};
use crate::streaming::{normalize_openai_sse_event, parse_sse_events};
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use godwit_core::{
    AudioSttRequest, AudioTtsRequest, Batch, BatchRequest, Capability, ChatCompletionRequest,
    ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse, ImageGenerationRequest,
    VideoGenerationRequest, ResponseFormat,
};
use godwit_db::models::Model;
use reqwest::Client;
use serde_json::Value;

pub struct LlamaCppProvider {
    client: Client,
}

pub type LlamaCppAdapter = LlamaCppProvider;

fn json_schema_to_gbnf(schema: &Value) -> String {
    let mut grammar = String::new();
    grammar.push_str("root ::= object\n");
    
    if let Some(obj) = schema.as_object() {
        if let Some(properties) = obj.get("properties").and_then(|p| p.as_object()) {
            for (key, value) in properties {
                if let Some(prop_schema) = value.as_object() {
                    let prop_type = prop_schema.get("type").and_then(|t| t.as_str()).unwrap_or("string");
                    grammar.push_str(&format!("{} ::= {}\n", key, match prop_type {
                        "string" => "\"\\\"\" string \"\\\"\"",
                        "number" => "number",
                        "integer" => "integer",
                        "boolean" => "boolean",
                        "array" => "array",
                        "object" => "object",
                        _ => "string",
                    }));
                }
            }
        }
    }
    
    grammar.push_str("object ::= \"{\" (ws string ws \":\" ws value (ws \",\" ws string ws \":\" ws value)*)? ws \"}\"\n");
    grammar.push_str("array ::= \"[\" (ws value (ws \",\" ws value)*)? ws \"]\"\n");
    grammar.push_str("value ::= string | number | integer | boolean | object | array | null\n");
    grammar.push_str("string ::= \"\\\"\" ([^\"\\\\] | \"\\\\\" [\"\\\\/bfnrt] | \"\\\\\" u [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F])* \"\\\"\"\n");
    grammar.push_str("number ::= \"-\"? ([0-9] | [1-9] [0-9]*) (\".\" [0-9]+)? ([eE] [+-]? [0-9]+)?\n");
    grammar.push_str("integer ::= \"-\"? ([0-9] | [1-9] [0-9]*)\n");
    grammar.push_str("boolean ::= \"true\" | \"false\"\n");
    grammar.push_str("null ::= \"null\"\n");
    grammar.push_str("ws ::= ([ \\t\\n\\r])*\n");
    
    grammar
}

impl LlamaCppProvider {
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

impl Default for LlamaCppProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Adapter for LlamaCppProvider {
    fn supported_capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat, Capability::Embedding]
    }

    async fn chat(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        mut request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        // Translate the catalog/wildcard-resolved public id into the upstream model id.
        request.model = model.provider_model_id.clone();
        crate::web_search::strip_native_web_search_from_request(&mut request);
        let url = format!("{}/chat/completions", profile.base_url);
        
        let mut request_body = serde_json::to_value(&request).map_err(|e| ProviderError::Serialization(e.to_string()))?;
        
        if let Some(ResponseFormat::JsonSchema { json_schema }) = &request.response_format {
            if let Some(obj) = request_body.as_object_mut() {
                obj.remove("response_format");
                if let Some(schema) = &json_schema.schema {
                    let grammar = json_schema_to_gbnf(schema);
                    obj.insert("grammar".to_string(), Value::String(grammar));
                }
            }
        }
        
        let mut req = self.client.post(&url).json(&request_body);
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
        crate::web_search::strip_native_web_search_from_request(&mut request);
        let url = format!("{}/chat/completions", profile.base_url);
        
        let mut request_body = serde_json::to_value(&request).map_err(|e| ProviderError::Serialization(e.to_string()))?;
        
        if let Some(ResponseFormat::JsonSchema { json_schema }) = &request.response_format {
            if let Some(obj) = request_body.as_object_mut() {
                obj.remove("response_format");
                if let Some(schema) = &json_schema.schema {
                    let grammar = json_schema_to_gbnf(schema);
                    obj.insert("grammar".to_string(), Value::String(grammar));
                }
            }
        }
        
        let mut req = self.client.post(&url).json(&request_body);
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
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: ImageGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "image generation is not supported by llama.cpp".to_string(),
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
            "image edit is not supported by llama.cpp".to_string(),
        ))
    }

    async fn video_generation(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: VideoGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "video generation is not supported by llama.cpp".to_string(),
        ))
    }

    async fn audio_tts(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "audio TTS is not supported by llama.cpp".to_string(),
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
            "audio STT is not supported by llama.cpp".to_string(),
        ))
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

    async fn create_batch(
        &self,
        _profile: &ResolvedProfile,
        _model: &Model,
        _request: BatchRequest,
    ) -> Result<Batch, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "batch is not supported for llama.cpp".to_string(),
        ))
    }

    async fn retrieve_batch(
        &self,
        _profile: &ResolvedProfile,
        _batch_id: String,
    ) -> Result<Batch, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "batch is not supported for llama.cpp".to_string(),
        ))
    }

    async fn cancel_batch(
        &self,
        _profile: &ResolvedProfile,
        _batch_id: String,
    ) -> Result<Batch, ProviderError> {
        Err(ProviderError::CapabilityNotSupported(
            "batch is not supported for llama.cpp".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use godwit_core::{ChatContent, ChatMessage};
    use uuid::Uuid;
    use wiremock::{matchers::*, Mock, MockServer, ResponseTemplate};

    fn dummy_profile(base_url: String) -> ResolvedProfile {
        ResolvedProfile {
            base_url,
            api_key: None,
        }
    }

    fn dummy_model() -> Model {
        Model {
            id: Uuid::nil(),
            public_id: "llama-3-8b-instruct".to_string(),
            provider: "llama_cpp".to_string(),
            provider_profile_id: Uuid::nil(),
            provider_model_id: "llama-3-8b-instruct.Q4_K_M.gguf".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({}),
            config: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn chat_returns_openai_shape_without_auth_header() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "id": "chatcmpl-1", "object": "chat.completion", "created": 1,
            "model": "llama-3-8b-instruct.Q4_K_M.gguf",
            "choices": [{"index":0,"message":{"role":"assistant","content":"Hi there"},"finish_reason":"stop"}],
            "usage": {"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
        });
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = LlamaCppAdapter::new();
        let profile = dummy_profile(server.uri());
        let req = ChatCompletionRequest {
            model: "llama-3-8b-instruct".to_string(),
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
        let (resp, _usage) = client.chat(&profile, &dummy_model(), req).await.unwrap();
        let ProviderResponse::Chat(completion) = resp else {
            panic!("expected chat response")
        };
        assert_eq!(completion.choices[0].message.content_as_text(), Some("Hi there".to_string()));

        // Verify no Authorization header was sent (since api_key is None)
        let received = server
            .received_requests()
            .await
            .expect("request recording enabled");
        assert_eq!(received.len(), 1);
        assert!(
            received[0].headers.get("authorization").is_none(),
            "expected no Authorization header when api_key is None, got: {:?}",
            received[0].headers.get("authorization")
        );
    }

    #[tokio::test]
    async fn unsupported_capabilities_return_error() {
        let client = LlamaCppAdapter::new();
        let profile = dummy_profile("http://localhost:8000/v1".to_string());
        let err = client
            .image_generation(
                &profile,
                &dummy_model(),
                ImageGenerationRequest {
                    model: "llama-3-8b-instruct".to_string(),
                    prompt: "a cat".to_string(),
                    n: None,
                    size: None,
                    quality: None,
                    style: None,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::CapabilityNotSupported(_)));
    }
}
