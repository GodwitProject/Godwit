pub mod adapter;
pub mod anthropic;
pub mod azure_openai;
pub mod gemini;
pub mod llama_cpp;
pub mod ollama;
pub mod openai;
pub mod registry;
pub mod searxng;
pub mod sglang;
pub mod sse_egress;
pub mod streaming;
pub mod usage;
pub mod vllm;
pub mod web_search;

pub use adapter::{Adapter, ProviderError, ProviderResponse, SseEvent, UsageReport};
pub use searxng::{SearxngProvider, SearxngResult, SearxngResponse};
pub use web_search::{
    has_native_web_search_tool, is_native_web_search_tool, strip_native_web_search_from_request,
    strip_native_web_search_tools, web_search_tool, NATIVE_WEB_SEARCH_TOOLS,
};
pub use registry::AdapterRegistry;
pub use usage::{
    compute_cost, compute_chat_cost, compute_embedding_cost, compute_image_cost,
    compute_audio_tts_cost, compute_audio_stt_cost, chat_usage_report,
};
