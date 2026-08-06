use async_trait::async_trait;
use futures::stream::BoxStream;
use godwit_core::{
    AudioSttRequest, AudioSttResponse, AudioTtsRequest, Batch, BatchRequest, Capability,
    ChatCompletionRequest, ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse,
    ImageGenerationRequest, ImageGenerationResponse, VideoGenerationRequest,
    VideoGenerationResponse,
};
use godwit_db::models::Model;
use thiserror::Error;

#[derive(Debug)]
pub enum ProviderResponse {
    Chat(ChatCompletionResponse),
    Image(ImageGenerationResponse),
    Video(VideoGenerationResponse),
    AudioStt(AudioSttResponse),
    Embedding(EmbeddingResponse),
    Bytes(Vec<u8>, String),
    Json(serde_json::Value),
}

#[derive(Debug, Clone, Default)]
pub struct UsageReport {
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub image_count: Option<i64>,
    pub audio_seconds: Option<f64>,
    pub video_seconds: Option<f64>,
    pub tts_characters: Option<i64>,
    pub embedding_tokens: Option<i64>,
    pub cache_read_tokens: Option<i32>,
    pub cache_write_tokens: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub data: String,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("http error: {status}: {message}")]
    Http { status: u16, message: String },
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("capability not supported: {0}")]
    CapabilityNotSupported(String),
}

#[derive(Clone)]
pub struct ResolvedProfile {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl std::fmt::Debug for ResolvedProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedProfile")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "***redacted***"))
            .finish()
    }
}

#[async_trait]
pub trait Adapter: Send + Sync {
    fn supported_capabilities(&self) -> Vec<Capability>;

    async fn chat(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    async fn chat_stream(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError>;

    async fn image_generation(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: ImageGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    /// The source image (and optional mask) are passed as owned bytes (plus filename) because
    /// `async_trait` methods cannot easily be generic over lifetimes.
    async fn image_edit(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: godwit_core::ImageEditRequest,
        image_bytes: Vec<u8>,
        image_filename: String,
        mask_bytes: Option<Vec<u8>>,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    async fn video_generation(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: VideoGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    async fn audio_tts(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    /// The audio file is passed as owned bytes (plus filename and content type) because
    /// `async_trait` methods cannot easily be generic over lifetimes.
    async fn audio_stt(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: AudioSttRequest,
        file_bytes: Vec<u8>,
        filename: String,
        content_type: String,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    async fn embedding(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: EmbeddingRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    async fn create_batch(
        &self,
        profile: &ResolvedProfile,
        model: &Model,
        request: BatchRequest,
    ) -> Result<Batch, ProviderError>;

    async fn retrieve_batch(
        &self,
        profile: &ResolvedProfile,
        batch_id: String,
    ) -> Result<Batch, ProviderError>;

    async fn cancel_batch(
        &self,
        profile: &ResolvedProfile,
        batch_id: String,
    ) -> Result<Batch, ProviderError>;
}
