use async_trait::async_trait;
use futures::stream::BoxStream;
use godwit_core::{
    AudioSttRequest, AudioSttResponse, AudioTtsRequest, Capability, ChatCompletionRequest,
    ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse, ImageGenerationRequest,
    ImageGenerationResponse, VideoGenerationRequest, VideoGenerationResponse,
};
use godwit_db::models::{Model, ProviderProfile};
use thiserror::Error;

#[derive(Debug)]
pub enum ProviderResponse {
    Chat(ChatCompletionResponse),
    Image(ImageGenerationResponse),
    Video(VideoGenerationResponse),
    AudioStt(AudioSttResponse),
    Embedding(EmbeddingResponse),
    AudioTts(AudioTtsResponse),
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
}

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct AudioTtsResponse {
    pub bytes: Vec<u8>,
    pub content_type: String,
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

#[async_trait]
pub trait Adapter: Send + Sync {
    fn supported_capabilities(&self) -> Vec<Capability>;

    async fn chat(
        &self,
        profile: &ProviderProfile,
        model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    async fn chat_stream(
        &self,
        profile: &ProviderProfile,
        model: &Model,
        request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError>;

    async fn image_generation(
        &self,
        profile: &ProviderProfile,
        model: &Model,
        request: ImageGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    async fn video_generation(
        &self,
        profile: &ProviderProfile,
        model: &Model,
        request: VideoGenerationRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    async fn audio_tts(
        &self,
        profile: &ProviderProfile,
        model: &Model,
        request: AudioTtsRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    /// The audio file is passed as owned bytes (plus filename and content type) because
    /// `async_trait` methods cannot easily be generic over lifetimes.
    async fn audio_stt(
        &self,
        profile: &ProviderProfile,
        model: &Model,
        request: AudioSttRequest,
        file_bytes: Vec<u8>,
        filename: String,
        content_type: String,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;

    async fn embedding(
        &self,
        profile: &ProviderProfile,
        model: &Model,
        request: EmbeddingRequest,
    ) -> Result<(ProviderResponse, UsageReport), ProviderError>;
}
