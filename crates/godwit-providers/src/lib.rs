pub mod anthropic;
pub mod openai;
pub mod streaming;

use async_trait::async_trait;
use futures::stream::BoxStream;
use godwit_core::{ChatCompletionRequest, ChatCompletionResponse};

#[derive(Debug)]
pub enum ProviderError {
    Http(String),
    Serialization(String),
    Provider(String),
    NotImplemented,
}

#[derive(Debug)]
pub enum ProviderResponse {
    Json(ChatCompletionResponse),
}

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub data: String,
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ProviderResponse, ProviderError>;

    async fn stream_chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<BoxStream<'static, Result<SseEvent, ProviderError>>, ProviderError>;
}
