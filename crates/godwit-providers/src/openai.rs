use crate::adapter::{
    Adapter, ProviderError, ProviderResponse, ResolvedProfile, SseEvent, UsageReport,
};
use crate::streaming::parse_sse_events;
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use godwit_core::{
    AudioSttRequest, AudioSttResponse, AudioTtsRequest, Capability, ChatCompletionRequest,
    ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse, ImageGenerationRequest,
    ImageGenerationResponse,
};
use godwit_db::models::Model;
use reqwest::Client;

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
        _model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
        let url = format!("{}/chat/completions", profile.base_url);
        let mut req = self.client.post(&url).json(&request);
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
        Ok((ProviderResponse::Chat(body), UsageReport::default()))
    }

    async fn chat_stream(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        mut request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError> {
        request.stream = Some(true);
        let url = format!("{}/chat/completions", profile.base_url);
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
        profile: &ResolvedProfile,
        _model: &Model,
        request: ImageGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
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
        // TODO: Populate UsageReport once OpenAI exposes usage metadata for image generation.
        Ok((ProviderResponse::Image(body), UsageReport::default()))
    }

    async fn image_edit(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        request: godwit_core::ImageEditRequest,
        image_bytes: Vec<u8>,
        image_filename: String,
        mask_bytes: Option<Vec<u8>>,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
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
        Ok((ProviderResponse::Image(body), UsageReport::default()))
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
        _model: &Model,
        request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
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
        // TODO: Populate UsageReport once OpenAI exposes usage metadata for audio TTS.
        Ok((
            ProviderResponse::Bytes(bytes, content_type),
            UsageReport::default(),
        ))
    }

    async fn audio_stt(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        request: AudioSttRequest,
        file_bytes: Vec<u8>,
        filename: String,
        content_type: String,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
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
        // TODO: Populate UsageReport once OpenAI exposes usage metadata for audio STT.
        Ok((ProviderResponse::AudioStt(body), UsageReport::default()))
    }

    async fn embedding(
        &self,
        profile: &ResolvedProfile,
        _model: &Model,
        request: EmbeddingRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError> {
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
        AudioSttRequest, AudioTtsRequest, ChatCompletionRequest, ChatMessage, EmbeddingRequest,
        ImageGenerationRequest,
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
                content: "Hi".to_string(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
        };
        let (resp, _usage) = client.chat(&profile, &dummy_model(), req).await.unwrap();
        let ProviderResponse::Chat(completion) = resp else {
            panic!("expected chat response");
        };
        assert_eq!(completion.choices[0].message.content, "Hello");
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
        Mock::given(method("POST"))
            .and(path("/images/edits"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = OpenAiAdapter::new();
        let profile = crate::adapter::ResolvedProfile {
            base_url: server.uri(),
            api_key: Some("fake-key".to_string()),
        };
        let req = godwit_core::ImageEditRequest {
            model: "gpt-image-1".to_string(),
            prompt: "add a hat".to_string(),
            n: Some(1),
            size: None,
            response_format: None,
        };
        let (resp, _usage) = client
            .image_edit(&profile, &dummy_model(), req, vec![1, 2, 3], "image.png".to_string(), None)
            .await
            .unwrap();
        let ProviderResponse::Image(image) = resp else { panic!("expected image response") };
        assert_eq!(image.data[0].url.as_deref(), Some("https://example.com/edited.png"));
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
            model: "whisper-1".to_string(),
            language: Some("en".to_string()),
            response_format: Some("json".to_string()),
        };
        let (ProviderResponse::AudioStt(resp), _) = client
            .audio_stt(
                &profile,
                &dummy_model(),
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
}
