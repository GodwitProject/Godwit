pub mod adapter;
pub mod anthropic;
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
    strip_native_web_search_tools, NATIVE_WEB_SEARCH_TOOLS,
};
pub use registry::AdapterRegistry;
